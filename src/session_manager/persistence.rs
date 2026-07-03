//! Persistence layer for the session manager.
//!
//! Responsibilities:
//! - Serialization format (`PersistentState`).
//! - Crash-safe atomic writes (temp file + fsync + `.bak` rotation + rename).
//! - Recovery on load (primary file, then `.bak`, then a clean default).
//! - The background writer thread that keeps disk I/O off the hot path.

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender};
use std::time::{Duration, Instant};

use super::{Session, Shared};

/// Errors surfaced by a persistence attempt. Kept as strings so the type is
/// cheap to clone and easy to hand across an FFI boundary.
#[derive(Debug, Clone, thiserror::Error)]
pub enum PersistError {
    #[error("failed to serialize session state: {0}")]
    Serialize(String),
    #[error("failed to write session state: {0}")]
    Io(String),
}

/// On-disk representation. Sessions are stored as a flat vector; the in-memory
/// map is rebuilt on load.
#[derive(Serialize, Deserialize)]
pub(crate) struct PersistentState {
    pub sessions: Vec<Session>,
    pub current_session_id: String,
    pub show_fractions: bool,
}

/// Messages accepted by the background writer thread.
pub(crate) enum PersistMsg {
    /// State changed; schedule a (debounced) write.
    Dirty,
    /// Write immediately and report the result back.
    Flush(SyncSender<Result<(), PersistError>>),
    /// Write a final snapshot and stop.
    Shutdown,
}

/// Result of attempting to load state from disk.
pub(crate) struct LoadResult {
    /// The parsed state, or `None` when nothing usable was found (fresh start).
    pub state: Option<PersistentState>,
    /// A human-readable note when we recovered from a backup or found the
    /// data corrupt. Surfaced to callers via `load_warning()`.
    pub warning: Option<String>,
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut os = path.as_os_str().to_os_string();
    os.push(suffix);
    PathBuf::from(os)
}

fn tmp_path(path: &Path) -> PathBuf {
    with_suffix(path, ".tmp")
}

fn bak_path(path: &Path) -> PathBuf {
    with_suffix(path, ".bak")
}

fn try_read(path: &Path) -> Option<PersistentState> {
    let content = fs::read_to_string(path).ok()?;
    serde_json::from_str::<PersistentState>(&content).ok()
}

/// Load persisted state, preferring the primary file and falling back to the
/// `.bak` copy before giving up. Never silently discards recoverable data.
pub(crate) fn load(path: &Path) -> LoadResult {
    let primary_exists = path.exists();
    let bak = bak_path(path);
    let bak_exists = bak.exists();

    if let Some(state) = try_read(path) {
        return LoadResult {
            state: Some(state),
            warning: None,
        };
    }

    // Primary missing or corrupt: try the backup.
    if let Some(state) = try_read(&bak) {
        let warning = if primary_exists {
            Some("primary session file was corrupt; recovered from backup".to_string())
        } else {
            Some("primary session file was missing; recovered from backup".to_string())
        };
        return LoadResult {
            state: Some(state),
            warning,
        };
    }

    // Nothing usable.
    let warning = if primary_exists || bak_exists {
        Some("session files were unreadable or corrupt; starting from a clean state".to_string())
    } else {
        None // genuine first run
    };
    LoadResult {
        state: None,
        warning,
    }
}

/// Atomically write `state` to `path`. Writes to a temp file, fsyncs it,
/// preserves the previous file as `.bak`, then renames into place so a crash
/// can never leave a torn primary file.
pub(crate) fn atomic_write(path: &Path, state: &PersistentState) -> Result<(), PersistError> {
    let json =
        serde_json::to_string(state).map_err(|e| PersistError::Serialize(e.to_string()))?;

    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        fs::create_dir_all(parent).map_err(|e| PersistError::Io(e.to_string()))?;
    }

    let tmp = tmp_path(path);
    {
        let mut f = fs::File::create(&tmp).map_err(|e| PersistError::Io(e.to_string()))?;
        f.write_all(json.as_bytes())
            .map_err(|e| PersistError::Io(e.to_string()))?;
        f.sync_all().map_err(|e| PersistError::Io(e.to_string()))?;
    }

    // Preserve the previous good file as a backup before replacing it. If we
    // cannot create the backup, fail *before* the rename: overwriting the only
    // good copy without a recovery path would defeat the crash-safety guarantee.
    if path.exists() {
        fs::copy(path, bak_path(path)).map_err(|e| PersistError::Io(e.to_string()))?;
    }

    fs::rename(&tmp, path).map_err(|e| PersistError::Io(e.to_string()))?;
    Ok(())
}

/// Take a consistent-enough snapshot of the shared state. Each session is
/// cloned under its own read lock; sessions are emitted in a deterministic
/// (name-sorted) order so the on-disk file is stable.
fn snapshot(shared: &Shared) -> PersistentState {
    let current_session_id = shared.active_id.read().clone();
    let show_fractions = shared.settings.read().show_fractions;

    let mut sessions: Vec<Session> = shared
        .sessions
        .iter()
        .map(|entry| entry.value().read().clone())
        .collect();
    sessions.sort_by(|a, b| a.name.cmp(&b.name));

    PersistentState {
        sessions,
        current_session_id,
        show_fractions,
    }
}

/// Snapshot and write, recording the outcome in `shared.last_error`.
pub(crate) fn snapshot_and_write(shared: &Shared) -> Result<(), PersistError> {
    let state = snapshot(shared);
    let res = atomic_write(&shared.path, &state);
    match &res {
        Ok(()) => *shared.last_error.write() = None,
        Err(e) => *shared.last_error.write() = Some(e.to_string()),
    }
    res
}

/// Background writer loop. Coalesces bursts of `Dirty` messages within the
/// debounce window into a single write, honors `Flush` promptly, and always
/// performs a final write on shutdown.
pub(crate) fn writer_loop(shared: Arc<Shared>, rx: Receiver<PersistMsg>, debounce: Duration) {
    loop {
        match rx.recv() {
            // Sender side gone: nothing more will change, persist and stop.
            Err(_) => {
                let _ = snapshot_and_write(&shared);
                break;
            }
            Ok(PersistMsg::Shutdown) => {
                let _ = snapshot_and_write(&shared);
                break;
            }
            Ok(PersistMsg::Flush(reply)) => {
                let _ = reply.send(snapshot_and_write(&shared));
            }
            Ok(PersistMsg::Dirty) => {
                // Coalesce further activity within the debounce window.
                let deadline = Instant::now() + debounce;
                let mut flush_reply: Option<SyncSender<Result<(), PersistError>>> = None;
                let mut shutdown = false;
                loop {
                    let now = Instant::now();
                    if now >= deadline {
                        break;
                    }
                    match rx.recv_timeout(deadline - now) {
                        Ok(PersistMsg::Dirty) => continue,
                        Ok(PersistMsg::Flush(reply)) => {
                            flush_reply = Some(reply);
                            break;
                        }
                        Ok(PersistMsg::Shutdown) => {
                            shutdown = true;
                            break;
                        }
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => {
                            shutdown = true;
                            break;
                        }
                    }
                }
                let res = snapshot_and_write(&shared);
                if let Some(reply) = flush_reply {
                    let _ = reply.send(res);
                }
                if shutdown {
                    break;
                }
            }
        }
    }
}
