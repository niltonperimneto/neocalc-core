use crate::engine::ast::Context;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

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
    pub history: Vec<HistoryEntry>,
    pub context: Context,
    pub buffer: String,
    pub last_result: Option<String>,
    pub mode: String,
}

#[derive(Serialize, Deserialize)]
struct PersistentState {
    sessions: Vec<Session>,
    current_session_id: String,
    show_fractions: bool,
}

pub struct AppState {
    pub sessions: HashMap<String, Session>,
    pub current_session_id: String,
    pub show_fractions: bool,
    pub storage_path: PathBuf,
}

pub struct AppSessionManager {
    state: Mutex<AppState>,
}

#[derive(Clone, Debug)]
pub struct SessionOverview {
    pub id: String,
    pub name: String,
    pub is_active: bool,
}

impl AppSessionManager {
    pub fn new(storage_path: String) -> Self {
        let path = PathBuf::from(&storage_path);

        // Try load
        let (sessions, current_id, show_fractions) = if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(state) = serde_json::from_str::<PersistentState>(&content) {
                    let mut map = HashMap::new();
                    for s in state.sessions {
                        map.insert(s.id.clone(), s);
                    }
                    (map, state.current_session_id, state.show_fractions)
                } else {
                    Self::default_state()
                }
            } else {
                Self::default_state()
            }
        } else {
            Self::default_state()
        };

        Self {
            state: Mutex::new(AppState {
                sessions,
                current_session_id: current_id,
                show_fractions,
                storage_path: path,
            }),
        }
    }

    fn default_state() -> (HashMap<String, Session>, String, bool) {
        let id = uuid::Uuid::new_v4().to_string();
        let session = Session {
            id: id.clone(),
            name: "Session 1".to_string(),
            history: Vec::new(),
            context: Context::new(),
            buffer: "0".to_string(),
            last_result: None,
            mode: "STANDARD".to_string(),
        };
        let mut map = HashMap::new();
        map.insert(id.clone(), session);
        (map, id, false)
    }

    pub fn get_sessions_overview(&self) -> Vec<SessionOverview> {
        let state = self.state.lock().unwrap();
        let mut overview = Vec::new();
        for session in state.sessions.values() {
            overview.push(SessionOverview {
                id: session.id.clone(),
                name: session.name.clone(),
                is_active: session.id == state.current_session_id,
            });
        }
        overview.sort_by(|a, b| a.name.cmp(&b.name));
        overview
    }

    pub fn create_session(&self) -> String {
        let mut state = self.state.lock().unwrap();
        let id = uuid::Uuid::new_v4().to_string();
        let name = format!("Session {}", state.sessions.len() + 1);

        let session = Session {
            id: id.clone(),
            name,
            history: Vec::new(),
            context: Context::new(),
            buffer: "0".to_string(),
            last_result: None,
            mode: "STANDARD".to_string(),
        };

        state.sessions.insert(id.clone(), session);
        state.current_session_id = id.clone();
        Self::save(&state);
        id
    }

    pub fn switch_session(&self, id: String) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.sessions.contains_key(&id) {
            state.current_session_id = id;
            Self::save(&state);
            true
        } else {
            false
        }
    }

    pub fn delete_session(&self, id: String) -> bool {
        let mut state = self.state.lock().unwrap();
        if state.sessions.len() <= 1 {
            return false; // Cannot delete last session
        }

        if state.sessions.remove(&id).is_some() {
            if state.current_session_id == id {
                // Switch to another session if current was deleted
                if let Some(first_id) = state.sessions.keys().next().cloned() {
                    state.current_session_id = first_id;
                }
            }
            Self::save(&state);
            true
        } else {
            false
        }
    }

    pub fn rename_session(&self, id: String, new_name: String) -> bool {
        let mut state = self.state.lock().unwrap();
        if let Some(session) = state.sessions.get_mut(&id) {
            session.name = new_name;
            Self::save(&state);
            true
        } else {
            false
        }
    }

    pub fn input(&self, text: String) -> String {
        self.update_buffer(|buf| {
            if buf == "0" && text != "." {
                text.clone()
            } else {
                let mut s = buf.clone();
                s.push_str(&text);
                s
            }
        })
    }

    pub fn clear(&self) -> String {
        self.update_buffer(|_| "0".to_string())
    }

    pub fn backspace(&self) -> String {
        self.update_buffer(|buf| {
            if buf.len() > 0 && buf != "0" {
                let mut new_buf = buf.clone();
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

    // Helper for buffer updates
    fn update_buffer<F>(&self, op: F) -> String
    where
        F: Fn(&String) -> String,
    {
        let mut state = self.state.lock().unwrap();
        let id = state.current_session_id.clone();

        let new_buf = if let Some(session) = state.sessions.get_mut(&id) {
            session.buffer = op(&session.buffer);
            Some(session.buffer.clone())
        } else {
            None
        };

        if let Some(res) = new_buf {
            Self::save(&state);
            res
        } else {
            "Error".to_string()
        }
    }

    pub fn evaluate(&self) -> String {
        let mut state = self.state.lock().unwrap();
        let id = state.current_session_id.clone();
        let show_fractions = state.show_fractions;

        // Split borrow: get needed data from session, do logic, then update
        let (expr, result) = if let Some(session) = state.sessions.get_mut(&id) {
            let expr = session.buffer.clone();
            let result = match crate::engine::evaluate(&expr, &mut session.context) {
                Ok(num) => {
                    if show_fractions {
                        crate::utils::format_number(num, false)
                    } else {
                        crate::utils::format_number(num, true)
                    }
                }
                Err(e) => format!("Error: {:?}", e),
            };
            (expr, result)
        } else {
            return "Error".to_string();
        };

        // Update state with result and history
        if let Some(session) = state.sessions.get_mut(&id) {
            let is_error = result.starts_with("Error");
            let timestamp = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0);

            session.history.push(HistoryEntry {
                expression: expr.clone(),
                result: result.clone(),
                timestamp,
                is_error,
            });
            session.buffer = result.clone();
            session.last_result = Some(result.clone());
        }

        Self::save(&state);
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
        let mut state = self.state.lock().unwrap();
        let id = state.current_session_id.clone();

        if let Some(session) = state.sessions.get_mut(&id) {
            let expr = session.buffer.clone();
            match crate::engine::evaluate(&expr, &mut session.context) {
                Ok(crate::engine::types::Number::Integer(i)) => {
                    let val = match radix {
                        16 => format!("{:X}", i),
                        8 => format!("{:o}", i),
                        2 => format!("{:b}", i),
                        _ => return "Error".to_string(),
                    };
                    let result = format!("{}{}", prefix, val);
                    session.buffer = result.clone();
                    Self::save(&state);
                    result
                }
                _ => "Not an integer".to_string(),
            }
        } else {
            "Error".to_string()
        }
    }

    pub fn get_buffer(&self) -> String {
        let state = self.state.lock().unwrap();
        if let Some(session) = state.sessions.get(&state.current_session_id) {
            session.buffer.clone()
        } else {
            "0".to_string()
        }
    }

    pub fn get_last_result(&self) -> Option<String> {
        let state = self.state.lock().unwrap();
        state
            .sessions
            .get(&state.current_session_id)
            .and_then(|s| s.last_result.clone())
    }

    pub fn get_history(&self) -> Vec<HistoryEntry> {
        let state = self.state.lock().unwrap();
        if let Some(session) = state.sessions.get(&state.current_session_id) {
            session.history.clone()
        } else {
            Vec::new()
        }
    }

    pub fn set_fraction_display(&self, enabled: bool) {
        let mut state = self.state.lock().unwrap();
        state.show_fractions = enabled;
        Self::save(&state);
    }

    // Internal helper
    fn save(state: &AppState) {
        let sessions: Vec<Session> = state.sessions.values().cloned().collect();
        let persistent = PersistentState {
            sessions,
            current_session_id: state.current_session_id.clone(),
            show_fractions: state.show_fractions,
        };

        if let Ok(json) = serde_json::to_string(&persistent) {
            let _ = fs::write(&state.storage_path, json);
        }
    }
}
