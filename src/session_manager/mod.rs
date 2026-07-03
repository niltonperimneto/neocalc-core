//! Concurrent, crash-safe multi-session manager for the calculator engine.
//!
//! Design goals (see the concurrency/safety redesign):
//! - **Fine-grained locking.** Each session lives behind its own `RwLock`, held
//!   inside a `DashMap`. Operations on different sessions run concurrently, and
//!   an expensive evaluation only blocks the one session it runs on.
//! - **Off-hot-path persistence.** Mutations mark state dirty and a background
//!   thread writes atomically, debounced. Nothing does disk I/O while holding a
//!   session lock.
//! - **No lock poisoning.** `parking_lot` locks do not poison, so a panic in one
//!   operation cannot brick the whole engine. Evaluation is additionally
//!   isolated with `catch_unwind` so a bug never unwinds across an FFI boundary.
//! - **Bounded resources.** History is a ring buffer; session count and buffer
//!   length are capped. Safe to embed on constrained devices.

mod persistence;

pub use persistence::PersistError;
use persistence::PersistMsg;

use crate::engine::ast::Context;
use dashmap::DashMap;
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{self, SyncSender};
use std::thread::JoinHandle;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub expression: String,
    pub result: String,
    pub timestamp: u64,
    pub is_error: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub name: String,
    /// Bounded, oldest-first ring buffer of evaluations.
    pub history: VecDeque<HistoryEntry>,
    pub context: Context,
    pub buffer: String,
    pub last_result: Option<String>,
    pub mode: String,
}

#[derive(Clone, Debug)]
pub struct SessionOverview {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

/// Runtime configuration and resource limits.
#[derive(Clone, Debug)]
pub struct Settings {
    /// Whether results are shown as fractions (persisted).
    pub show_fractions: bool,
    /// Maximum number of concurrent sessions.
    pub max_sessions: usize,
    /// Maximum history entries retained per session (oldest dropped).
    pub max_history: usize,
    /// Maximum input-buffer length (guards memory and pathological parses).
    pub max_buffer_len: usize,
    /// How long the writer coalesces changes before flushing to disk.
    pub persist_debounce: Duration,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            show_fractions: false,
            max_sessions: 100,
            max_history: 500,
            max_buffer_len: 4096,
            persist_debounce: Duration::from_millis(250),
        }
    }
}

/// Shared state, owned via `Arc` so the background writer can snapshot it
/// independently of the public handle.
pub(crate) struct Shared {
    pub sessions: DashMap<String, Arc<RwLock<Session>>>,
    pub active_id: RwLock<String>,
    pub settings: RwLock<Settings>,
    pub path: PathBuf,
    /// Result of the most recent persistence attempt (`None` == last write ok).
    pub last_error: RwLock<Option<String>>,
    /// Note about recovery/corruption detected at load time.
    pub load_warning: RwLock<Option<String>>,
}

/// Bound on the persistence signal queue. `Dirty` messages are idempotent
/// wake-ups (each write snapshots the whole state), so a full queue simply
/// means a write is already pending and further signals can be dropped. This
/// keeps memory bounded even under sustained input, while leaving ample room
/// for interleaved control messages (`Flush`/`Shutdown`).
const PERSIST_QUEUE_BOUND: usize = 1024;

pub struct AppSessionManager {
    shared: Arc<Shared>,
    persist: SyncSender<PersistMsg>,
    writer: Option<JoinHandle<()>>,
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn default_session(n: usize) -> Session {
    Session {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("Session {}", n),
        history: VecDeque::new(),
        context: Context::new(),
        buffer: "0".to_string(),
        last_result: None,
        mode: "STANDARD".to_string(),
    }
}

fn trim_history(history: &mut VecDeque<HistoryEntry>, max: usize) {
    while history.len() > max {
        history.pop_front();
    }
}

impl AppSessionManager {
    /// Create a manager with default settings, loading any persisted state.
    pub fn new(storage_path: String) -> Self {
        Self::with_config(storage_path, Settings::default())
    }

    /// Create a manager with explicit settings/limits.
    pub fn with_config(storage_path: String, mut settings: Settings) -> Self {
        let path = PathBuf::from(&storage_path);
        let loaded = persistence::load(&path);
        let mut warning = loaded.warning;

        // Extract sessions + proposed active id from whatever we loaded.
        let (mut sess_vec, mut current, had_state) = match loaded.state {
            Some(state) => {
                settings.show_fractions = state.show_fractions;
                (state.sessions, state.current_session_id, true)
            }
            None => (Vec::new(), String::new(), false),
        };

        // Invariant: always at least one session.
        if sess_vec.is_empty() {
            sess_vec.push(default_session(1));
            if had_state && warning.is_none() {
                warning = Some("no sessions found in saved state; created a default".to_string());
            }
        }

        // Invariant: the active id must reference an existing session.
        if !sess_vec.iter().any(|s| s.id == current) {
            let mut by_name: Vec<&Session> = sess_vec.iter().collect();
            by_name.sort_by(|a, b| a.name.cmp(&b.name));
            current = by_name[0].id.clone();
            if had_state && warning.is_none() {
                warning =
                    Some("active session id was invalid; reset to an existing session".to_string());
            }
        }

        // Build the concurrent map, enforcing the history bound on load.
        let sessions: DashMap<String, Arc<RwLock<Session>>> = DashMap::new();
        for mut s in sess_vec {
            trim_history(&mut s.history, settings.max_history);
            sessions.insert(s.id.clone(), Arc::new(RwLock::new(s)));
        }

        let debounce = settings.persist_debounce;
        let shared = Arc::new(Shared {
            sessions,
            active_id: RwLock::new(current),
            settings: RwLock::new(settings),
            path,
            last_error: RwLock::new(None),
            load_warning: RwLock::new(warning),
        });

        let (tx, rx) = mpsc::sync_channel(PERSIST_QUEUE_BOUND);
        let writer = {
            let shared = Arc::clone(&shared);
            std::thread::Builder::new()
                .name("neocalc-persist".to_string())
                .spawn(move || persistence::writer_loop(shared, rx, debounce))
                .expect("failed to spawn persistence thread")
        };

        Self {
            shared,
            persist: tx,
            writer: Some(writer),
        }
    }

    fn mark_dirty(&self) {
        // Never block the hot path. A `Dirty` is only a wake-up; if the queue
        // is full a write is already pending, so a dropped signal is harmless
        // (the eventual write snapshots the latest state regardless).
        let _ = self.persist.try_send(PersistMsg::Dirty);
    }

    /// Clone the `Arc` for the active session out of the map, releasing the
    /// shard lock immediately so the session lock is held independently.
    fn active_session(&self) -> Option<Arc<RwLock<Session>>> {
        let id = self.shared.active_id.read().clone();
        self.shared
            .sessions
            .get(&id)
            .map(|entry| Arc::clone(entry.value()))
    }

    fn first_session_id_by_name(&self) -> Option<String> {
        let mut best: Option<(String, String)> = None; // (name, id)
        for entry in self.shared.sessions.iter() {
            let guard = entry.value().read();
            let candidate = (guard.name.clone(), guard.id.clone());
            match &best {
                Some((name, _)) if *name <= candidate.0 => {}
                _ => best = Some(candidate),
            }
        }
        best.map(|(_, id)| id)
    }

    // ----- session lifecycle -------------------------------------------------

    pub fn get_sessions_overview(&self) -> Vec<SessionOverview> {
        let active = self.shared.active_id.read().clone();
        let mut overview: Vec<SessionOverview> = self
            .shared
            .sessions
            .iter()
            .map(|entry| {
                let guard = entry.value().read();
                SessionOverview {
                    id: guard.id.clone(),
                    name: guard.name.clone(),
                    is_active: guard.id == active,
                }
            })
            .collect();
        overview.sort_by(|a, b| a.name.cmp(&b.name));
        overview
    }

    /// Create a new session and make it active. Returns its id, or `None` if
    /// the session limit has been reached.
    pub fn create_session(&self) -> Option<String> {
        let max = self.shared.settings.read().max_sessions;
        if self.shared.sessions.len() >= max {
            return None;
        }
        let session = default_session(self.shared.sessions.len() + 1);
        let id = session.id.clone();
        self.shared
            .sessions
            .insert(id.clone(), Arc::new(RwLock::new(session)));
        *self.shared.active_id.write() = id.clone();
        self.mark_dirty();
        Some(id)
    }

    pub fn switch_session(&self, id: String) -> bool {
        if self.shared.sessions.contains_key(&id) {
            *self.shared.active_id.write() = id;
            self.mark_dirty();
            true
        } else {
            false
        }
    }

    pub fn delete_session(&self, id: String) -> bool {
        // Never delete the last remaining session.
        if self.shared.sessions.len() <= 1 {
            return false;
        }
        if self.shared.sessions.remove(&id).is_none() {
            return false;
        }
        // Repair the active pointer if it no longer refers to a live session.
        // Checking membership (rather than `== id`) closes a race where a
        // concurrent delete leaves the active pointer dangling.
        {
            let mut active = self.shared.active_id.write();
            if !self.shared.sessions.contains_key(&*active)
                && let Some(new_id) = self.first_session_id_by_name()
            {
                *active = new_id;
            }
        }
        self.mark_dirty();
        true
    }

    pub fn rename_session(&self, id: String, new_name: String) -> bool {
        match self
            .shared
            .sessions
            .get(&id)
            .map(|entry| Arc::clone(entry.value()))
        {
            Some(session) => {
                session.write().name = new_name;
                self.mark_dirty();
                true
            }
            None => false,
        }
    }

    // ----- buffer editing ----------------------------------------------------

    fn with_buffer<F: FnOnce(&str) -> String>(&self, op: F) -> String {
        let Some(session) = self.active_session() else {
            return "Error".to_string();
        };
        let (new_buf, changed) = {
            let mut guard = session.write();
            let updated = op(&guard.buffer);
            let changed = updated != guard.buffer;
            guard.buffer = updated;
            (guard.buffer.clone(), changed)
        };
        // Skip persistence work when the buffer is untouched (e.g. input past
        // `max_buffer_len`, or backspace on an already-empty buffer).
        if changed {
            self.mark_dirty();
        }
        new_buf
    }

    pub fn input(&self, text: String) -> String {
        let max = self.shared.settings.read().max_buffer_len;
        self.with_buffer(|buf| {
            let candidate = if buf == "0" && text != "." {
                text.clone()
            } else {
                let mut s = buf.to_string();
                s.push_str(&text);
                s
            };
            // Reject growth past the cap; keep the existing buffer unchanged.
            if candidate.len() > max {
                buf.to_string()
            } else {
                candidate
            }
        })
    }

    pub fn clear(&self) -> String {
        self.with_buffer(|_| "0".to_string())
    }

    pub fn backspace(&self) -> String {
        self.with_buffer(|buf| {
            if !buf.is_empty() && buf != "0" {
                let mut new_buf = buf.to_string();
                new_buf.pop();
                if new_buf.is_empty() {
                    "0".to_string()
                } else {
                    new_buf
                }
            } else {
                "0".to_string()
            }
        })
    }

    // ----- evaluation --------------------------------------------------------

    pub fn evaluate(&self) -> String {
        let (show_fractions, max_history) = {
            let s = self.shared.settings.read();
            (s.show_fractions, s.max_history)
        };
        let Some(session) = self.active_session() else {
            return "Error".to_string();
        };

        let mut guard = session.write();
        let expr = guard.buffer.clone();

        // Isolate evaluation: a panic in the engine yields a clean error rather
        // than poisoning state or unwinding across the FFI boundary.
        let outcome = {
            let ctx = &mut guard.context;
            catch_unwind(AssertUnwindSafe(|| crate::engine::evaluate(&expr, ctx)))
        };

        let (result, is_error) = match outcome {
            Ok(Ok(num)) => (crate::utils::format_number(num, !show_fractions), false),
            // Use Display (not Debug) so the user sees the human-readable message.
            Ok(Err(e)) => (format!("Error: {}", e), true),
            Err(_) => ("Error: internal evaluation failure".to_string(), true),
        };

        guard.history.push_back(HistoryEntry {
            expression: expr,
            result: result.clone(),
            timestamp: now_millis(),
            is_error,
        });
        trim_history(&mut guard.history, max_history);
        guard.buffer = result.clone();
        guard.last_result = Some(result.clone());
        drop(guard);

        self.mark_dirty();
        result
    }

    pub fn convert_to_hex(&self) -> String {
        self.convert_base(16, "0x")
    }

    pub fn convert_to_bin(&self) -> String {
        self.convert_base(2, "0b")
    }

    pub fn convert_to_oct(&self) -> String {
        self.convert_base(8, "0o")
    }

    fn convert_base(&self, radix: u32, prefix: &str) -> String {
        let Some(session) = self.active_session() else {
            return "Error".to_string();
        };
        let mut guard = session.write();
        let expr = guard.buffer.clone();

        let outcome = {
            let ctx = &mut guard.context;
            catch_unwind(AssertUnwindSafe(|| crate::engine::evaluate(&expr, ctx)))
        };

        match outcome {
            Ok(Ok(crate::engine::types::Number::Integer(i))) => {
                let val = match radix {
                    16 => format!("{:X}", i),
                    8 => format!("{:o}", i),
                    2 => format!("{:b}", i),
                    _ => return "Error".to_string(),
                };
                let result = format!("{}{}", prefix, val);
                guard.buffer = result.clone();
                drop(guard);
                self.mark_dirty();
                result
            }
            // Non-integer value or evaluation error.
            Ok(_) => "Not an integer".to_string(),
            // Panic during evaluation.
            Err(_) => "Error".to_string(),
        }
    }

    // ----- reads -------------------------------------------------------------

    pub fn get_buffer(&self) -> String {
        self.active_session()
            .map(|s| s.read().buffer.clone())
            .unwrap_or_else(|| "0".to_string())
    }

    pub fn get_last_result(&self) -> Option<String> {
        self.active_session()
            .and_then(|s| s.read().last_result.clone())
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        self.active_session()
            .map(|s| s.read().history.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub fn set_fraction_display(&self, enabled: bool) {
        self.shared.settings.write().show_fractions = enabled;
        self.mark_dirty();
    }

    // ----- persistence control ----------------------------------------------

    /// Force an immediate, synchronous, atomic save. Call this from app
    /// lifecycle hooks (e.g. suspend/close) to guarantee durability.
    pub fn flush(&self) -> Result<(), PersistError> {
        let (tx, rx) = mpsc::sync_channel(1);
        if self.persist.send(PersistMsg::Flush(tx)).is_err() {
            // Writer is gone; persist directly from the calling thread.
            return persistence::snapshot_and_write(&self.shared);
        }
        match rx.recv() {
            Ok(res) => res,
            // The writer dropped the reply channel (e.g. it died) before
            // reporting a result. Don't claim durability we didn't achieve:
            // persist directly and return that outcome.
            Err(_) => persistence::snapshot_and_write(&self.shared),
        }
    }

    /// Error from the most recent persistence attempt, if any.
    pub fn last_persist_error(&self) -> Option<String> {
        self.shared.last_error.read().clone()
    }

    /// A note about recovery or corruption detected when state was loaded.
    pub fn load_warning(&self) -> Option<String> {
        self.shared.load_warning.read().clone()
    }
}

impl Drop for AppSessionManager {
    fn drop(&mut self) {
        // Ask the writer to flush a final snapshot, then wait for it.
        let _ = self.persist.send(PersistMsg::Shutdown);
        if let Some(handle) = self.writer.take() {
            let _ = handle.join();
        }
    }
}
