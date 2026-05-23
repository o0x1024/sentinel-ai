use crate::MachineId;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const TRIAL_FILE: &str = ".sentinel_trial";
const TRIAL_DURATION_SECONDS: i64 = 7 * 24 * 60 * 60;
const CLOCK_ROLLBACK_GRACE_SECONDS: i64 = 300;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrialState {
    started_at: i64,
    last_seen_at: i64,
    machine_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrialStatus {
    pub exists: bool,
    pub active: bool,
    pub started_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub remaining_seconds: Option<i64>,
    pub days_remaining: Option<i64>,
    pub machine_id_match: bool,
    pub clock_tampered: bool,
    pub error: Option<String>,
}

pub fn has_active_trial() -> bool {
    get_or_create_trial_status().active
}

pub fn get_or_create_trial_status() -> TrialStatus {
    match load_or_create_trial_state() {
        Ok(state) => evaluate_and_persist_trial_state(state),
        Err(error) => TrialStatus {
            exists: trial_path().map(|path| path.exists()).unwrap_or(false),
            active: false,
            started_at: None,
            expires_at: None,
            remaining_seconds: None,
            days_remaining: None,
            machine_id_match: false,
            clock_tampered: false,
            error: Some(error),
        },
    }
}

fn load_or_create_trial_state() -> Result<TrialState, String> {
    let path = trial_path().ok_or_else(|| "Cannot determine trial storage path".to_string())?;
    if path.exists() {
        return load_trial_state(&path);
    }

    let now = current_unix_timestamp();
    let state = TrialState {
        started_at: now,
        last_seen_at: now,
        machine_id: MachineId::generate().to_full_hex(),
    };
    save_trial_state(&path, &state)?;
    Ok(state)
}

fn evaluate_and_persist_trial_state(mut state: TrialState) -> TrialStatus {
    let now = current_unix_timestamp();
    let expires_at = state.started_at + TRIAL_DURATION_SECONDS;
    let machine_id_match = state.machine_id == MachineId::generate().to_full_hex();
    let clock_tampered = now + CLOCK_ROLLBACK_GRACE_SECONDS < state.last_seen_at;
    let active = machine_id_match && !clock_tampered && now < expires_at;
    let remaining_seconds = (expires_at - now).max(0);

    if machine_id_match && !clock_tampered && now > state.last_seen_at {
        state.last_seen_at = now;
        if let Some(path) = trial_path() {
            let _ = save_trial_state(&path, &state);
        }
    }

    TrialStatus {
        exists: true,
        active,
        started_at: Some(state.started_at),
        expires_at: Some(expires_at),
        remaining_seconds: Some(remaining_seconds),
        days_remaining: Some(days_remaining(remaining_seconds)),
        machine_id_match,
        clock_tampered,
        error: None,
    }
}

fn load_trial_state(path: &PathBuf) -> Result<TrialState, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Failed to read trial file: {}", e))?;
    let decrypted =
        decrypt_storage(&content).ok_or_else(|| "Failed to decrypt trial file".to_string())?;
    serde_json::from_str(&decrypted).map_err(|e| format!("Invalid trial file: {}", e))
}

fn save_trial_state(path: &PathBuf, state: &TrialState) -> Result<(), String> {
    let json = serde_json::to_string(state).map_err(|e| format!("Invalid trial state: {}", e))?;
    fs::write(path, encrypt_storage(&json))
        .map_err(|e| format!("Failed to write trial file: {}", e))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let permissions = fs::Permissions::from_mode(0o600);
        let _ = fs::set_permissions(path, permissions);
    }

    Ok(())
}

fn trial_path() -> Option<PathBuf> {
    let data_dir = dirs::data_local_dir()?;
    let app_dir = data_dir.join("sentinel-ai");

    if !app_dir.exists() {
        fs::create_dir_all(&app_dir).ok()?;
    }

    Some(app_dir.join(TRIAL_FILE))
}

fn encrypt_storage(data: &str) -> String {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha2::{Digest, Sha256};

    let machine_id = MachineId::generate();
    let mut hasher = Sha256::new();
    hasher.update(machine_id.to_hash());
    hasher.update(b"trial_storage_key_salt_v1");
    let key = hasher.finalize().to_vec();

    let encrypted: Vec<u8> = data
        .as_bytes()
        .iter()
        .enumerate()
        .map(|(index, byte)| *byte ^ key[index % key.len()])
        .collect();

    BASE64.encode(encrypted)
}

fn decrypt_storage(data: &str) -> Option<String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha2::{Digest, Sha256};

    let encrypted = BASE64.decode(data.trim()).ok()?;
    let machine_id = MachineId::generate();
    let mut hasher = Sha256::new();
    hasher.update(machine_id.to_hash());
    hasher.update(b"trial_storage_key_salt_v1");
    let key = hasher.finalize().to_vec();

    let decrypted: Vec<u8> = encrypted
        .iter()
        .enumerate()
        .map(|(index, byte)| *byte ^ key[index % key.len()])
        .collect();

    String::from_utf8(decrypted).ok()
}

fn current_unix_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

fn days_remaining(remaining_seconds: i64) -> i64 {
    if remaining_seconds <= 0 {
        return 0;
    }

    (remaining_seconds + 86_399) / 86_400
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_partial_days_up_for_trial_display() {
        assert_eq!(days_remaining(0), 0);
        assert_eq!(days_remaining(1), 1);
        assert_eq!(days_remaining(86_400), 1);
        assert_eq!(days_remaining(86_401), 2);
    }
}
