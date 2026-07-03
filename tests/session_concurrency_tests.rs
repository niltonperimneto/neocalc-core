//! Concurrency and safety tests for the session manager.
//!
//! These exercise the redesigned `AppSessionManager`: parallel access,
//! debounced/atomic persistence, crash-recovery from backups, invariant
//! repair on load, and the resource bounds.

use neocalc_core::session_manager::AppSessionManager;
use neocalc_core::Settings;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

static COUNTER: AtomicU64 = AtomicU64::new(0);

fn unique_path() -> PathBuf {
    let n = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!(
        "neocalc_sess_{}_{}_{}.json",
        std::process::id(),
        nanos,
        n
    ))
}

fn path_str(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn sibling(p: &Path, suffix: &str) -> PathBuf {
    let mut s = p.as_os_str().to_os_string();
    s.push(suffix);
    PathBuf::from(s)
}

fn cleanup(p: &Path) {
    let _ = fs::remove_file(p);
    let _ = fs::remove_file(sibling(p, ".bak"));
    let _ = fs::remove_file(sibling(p, ".tmp"));
}

#[test]
fn persist_round_trip_via_flush() {
    let path = unique_path();
    {
        let m = AppSessionManager::new(path_str(&path));
        m.clear();
        m.input("6*7".to_string());
        assert_eq!(m.evaluate(), "42");
        m.flush().expect("flush should succeed");
    }
    let m2 = AppSessionManager::new(path_str(&path));
    let hist = m2.get_history();
    assert_eq!(hist.len(), 1);
    assert_eq!(hist[0].result, "42");
    assert!(!hist[0].is_error);
    assert_eq!(m2.get_buffer(), "42");
    cleanup(&path);
}

#[test]
fn drop_flushes_state() {
    let path = unique_path();
    {
        let m = AppSessionManager::new(path_str(&path));
        m.clear();
        m.input("100".to_string());
        m.evaluate();
        // No explicit flush: rely on the Drop-triggered final write.
    }
    let m2 = AppSessionManager::new(path_str(&path));
    assert_eq!(m2.get_buffer(), "100");
    cleanup(&path);
}

#[test]
fn error_flag_is_accurate() {
    let path = unique_path();
    let m = AppSessionManager::new(path_str(&path));
    m.clear();
    m.input("undefined_var_xyz".to_string());
    let r = m.evaluate();
    let hist = m.get_history();
    assert_eq!(hist.len(), 1);
    assert!(
        hist[0].is_error,
        "an undefined variable should be flagged as an error, got {r:?}"
    );

    // A valid expression is not flagged.
    m.clear();
    m.input("2+2".to_string());
    m.evaluate();
    let hist = m.get_history();
    assert!(!hist.last().unwrap().is_error);
    cleanup(&path);
}

#[test]
fn recovers_from_backup_when_primary_corrupt() {
    let path = unique_path();
    {
        let m = AppSessionManager::new(path_str(&path));
        m.clear();
        m.input("22".to_string());
        m.flush().expect("flush should succeed");
        // Drop writes a second time, which copies the good primary into `.bak`.
    }
    assert!(sibling(&path, ".bak").exists(), "a backup should exist");

    // Corrupt the primary file only.
    fs::write(&path, b"{ this is not valid json").unwrap();

    let m = AppSessionManager::new(path_str(&path));
    assert_eq!(m.get_buffer(), "22", "should have recovered from backup");
    assert!(
        m.load_warning().is_some(),
        "recovery should surface a warning"
    );
    cleanup(&path);
}

#[test]
fn both_files_corrupt_yields_clean_usable_default() {
    let path = unique_path();
    // Create both files, then corrupt both.
    {
        let m = AppSessionManager::new(path_str(&path));
        m.clear();
        m.input("9".to_string());
        m.flush().unwrap();
    }
    fs::write(&path, b"garbage").unwrap();
    fs::write(sibling(&path, ".bak"), b"garbage").unwrap();

    let m = AppSessionManager::new(path_str(&path));
    assert!(m.load_warning().is_some(), "corruption should be surfaced");
    // Manager is fully usable from a clean default.
    assert_eq!(m.get_buffer(), "0");
    m.input("5".to_string());
    assert_eq!(m.evaluate(), "5");
    let ov = m.get_sessions_overview();
    assert_eq!(ov.len(), 1);
    cleanup(&path);
}

#[test]
fn repairs_invalid_active_session_on_load() {
    let path = unique_path();
    {
        let m = AppSessionManager::new(path_str(&path));
        m.clear();
        m.input("7".to_string());
        m.flush().unwrap();
    }
    // Point the active id at a session that does not exist.
    let content = fs::read_to_string(&path).unwrap();
    let mut v: serde_json::Value = serde_json::from_str(&content).unwrap();
    v["current_session_id"] = serde_json::Value::String("no-such-session".to_string());
    fs::write(&path, serde_json::to_string(&v).unwrap()).unwrap();

    let m = AppSessionManager::new(path_str(&path));
    // No panic; the active pointer was repaired to the existing session.
    assert_eq!(m.get_buffer(), "7");
    assert!(m.load_warning().is_some());
    let ov = m.get_sessions_overview();
    assert_eq!(ov.iter().filter(|s| s.is_active).count(), 1);
    cleanup(&path);
}

#[test]
fn history_is_bounded() {
    let path = unique_path();
    let settings = Settings {
        max_history: 3,
        ..Settings::default()
    };
    let m = AppSessionManager::with_config(path_str(&path), settings);
    for i in 1..=5 {
        m.clear();
        m.input(i.to_string());
        m.evaluate();
    }
    let hist = m.get_history();
    assert_eq!(hist.len(), 3, "history should be capped at max_history");
    // Oldest entries dropped; the three most recent remain, oldest-first.
    assert_eq!(hist[0].expression, "3");
    assert_eq!(hist[2].expression, "5");
    cleanup(&path);
}

#[test]
fn session_count_is_bounded() {
    let path = unique_path();
    let settings = Settings {
        max_sessions: 2,
        ..Settings::default()
    };
    let m = AppSessionManager::with_config(path_str(&path), settings);
    // Starts with one default session.
    assert!(m.create_session().is_some(), "second session allowed");
    assert!(
        m.create_session().is_none(),
        "third session should be rejected at the cap"
    );
    assert_eq!(m.get_sessions_overview().len(), 2);
    cleanup(&path);
}

#[test]
fn buffer_length_is_bounded() {
    let path = unique_path();
    let settings = Settings {
        max_buffer_len: 4,
        ..Settings::default()
    };
    let m = AppSessionManager::with_config(path_str(&path), settings);
    m.clear();
    assert_eq!(m.input("123".to_string()), "123");
    // "123456" exceeds the cap of 4; the buffer is left unchanged.
    assert_eq!(m.input("456".to_string()), "123");
    cleanup(&path);
}

#[test]
fn parallel_access_is_safe_and_consistent() {
    let path = unique_path();
    let m = Arc::new(AppSessionManager::new(path_str(&path)));

    let mut handles = Vec::new();
    for t in 0..8u64 {
        let m = Arc::clone(&m);
        handles.push(thread::spawn(move || {
            for i in 0..300u64 {
                match (i + t) % 6 {
                    0 => {
                        m.create_session();
                    }
                    1 => {
                        let ov = m.get_sessions_overview();
                        if let Some(s) = ov.first() {
                            m.switch_session(s.id.clone());
                        }
                    }
                    2 => {
                        m.input("1".to_string());
                    }
                    3 => {
                        m.clear();
                        m.input("2+2".to_string());
                        m.evaluate();
                    }
                    4 => {
                        let _ = m.get_history();
                        let _ = m.get_buffer();
                        let _ = m.get_last_result();
                    }
                    _ => {
                        let ov = m.get_sessions_overview();
                        if ov.len() > 1 {
                            m.delete_session(ov.last().unwrap().id.clone());
                        }
                    }
                }
            }
        }));
    }
    for h in handles {
        h.join().expect("no worker thread should panic");
    }

    // At rest the core invariants must hold.
    let ov = m.get_sessions_overview();
    assert!(!ov.is_empty(), "at least one session must remain");
    assert_eq!(
        ov.iter().filter(|s| s.is_active).count(),
        1,
        "exactly one session must be active and it must exist"
    );
    // Manager is still usable.
    m.clear();
    m.input("3*3".to_string());
    assert_eq!(m.evaluate(), "9");

    // Persistence still works after the stress run.
    m.flush().expect("flush should succeed");
    assert!(m.last_persist_error().is_none());
    cleanup(&path);
}

#[test]
fn oversized_persisted_buffer_is_reset_on_load() {
    let path = unique_path();
    {
        let m = AppSessionManager::new(path_str(&path));
        m.clear();
        m.input("123".to_string());
        m.flush().expect("flush should succeed");
    }

    // Tamper with the state file: inflate the buffer past max_buffer_len.
    let raw = fs::read_to_string(&path).unwrap();
    let mut state: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let huge = "9".repeat(Settings::default().max_buffer_len + 1);
    state["sessions"][0]["buffer"] = serde_json::Value::String(huge);
    fs::write(&path, serde_json::to_string(&state).unwrap()).unwrap();

    // The oversized buffer must be rejected on load, not parsed or kept.
    let m2 = AppSessionManager::new(path_str(&path));
    assert_eq!(m2.get_buffer(), "0");
    cleanup(&path);
}
