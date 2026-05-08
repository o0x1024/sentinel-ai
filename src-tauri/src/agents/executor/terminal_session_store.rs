use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;

static ACTIVE_TERMINAL_SESSIONS: Lazy<Mutex<HashMap<String, String>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

pub struct ActiveTerminalSessionGuard {
    execution_id: String,
}

impl Drop for ActiveTerminalSessionGuard {
    fn drop(&mut self) {
        clear_active_terminal_session(&self.execution_id);
    }
}

pub fn scope_active_terminal_session(
    execution_id: &str,
    session_id: Option<&str>,
) -> ActiveTerminalSessionGuard {
    set_active_terminal_session(execution_id, session_id);
    ActiveTerminalSessionGuard {
        execution_id: execution_id.to_string(),
    }
}

pub fn set_active_terminal_session(execution_id: &str, session_id: Option<&str>) {
    let Ok(mut sessions) = ACTIVE_TERMINAL_SESSIONS.lock() else {
        return;
    };

    let normalized_execution_id = execution_id.trim();
    if normalized_execution_id.is_empty() {
        return;
    }

    let normalized_session_id = session_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);

    if let Some(session_id) = normalized_session_id {
        sessions.insert(normalized_execution_id.to_string(), session_id);
    } else {
        sessions.remove(normalized_execution_id);
    }
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn get_active_terminal_session(execution_id: &str) -> Option<String> {
    let Ok(sessions) = ACTIVE_TERMINAL_SESSIONS.lock() else {
        return None;
    };

    sessions.get(execution_id.trim()).cloned()
}

pub fn clear_active_terminal_session(execution_id: &str) {
    let Ok(mut sessions) = ACTIVE_TERMINAL_SESSIONS.lock() else {
        return;
    };
    sessions.remove(execution_id.trim());
}

#[cfg(test)]
mod tests {
    use super::{
        clear_active_terminal_session, get_active_terminal_session, scope_active_terminal_session,
        set_active_terminal_session,
    };

    #[test]
    fn set_and_get_active_terminal_session() {
        clear_active_terminal_session("exec-1");
        set_active_terminal_session("exec-1", Some("sess-1"));
        assert_eq!(
            get_active_terminal_session("exec-1").as_deref(),
            Some("sess-1")
        );
        clear_active_terminal_session("exec-1");
    }

    #[test]
    fn guard_clears_session_on_drop() {
        {
            let _guard = scope_active_terminal_session("exec-2", Some("sess-2"));
            assert_eq!(
                get_active_terminal_session("exec-2").as_deref(),
                Some("sess-2")
            );
        }

        assert!(get_active_terminal_session("exec-2").is_none());
    }
}
