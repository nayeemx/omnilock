use std::fs;
use std::path::PathBuf;

pub use omnilock_shared::LockedItem;

const SERVICE_DIR: &str = "InnologyBD\\OmniLock";
const STATE_FILE: &str = "service-state.json";
const VAULT_FILE: &str = "vault.enc";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct LockedItemsState {
    pub locked_items: Vec<LockedItem>,
}

fn programdata_dir() -> PathBuf {
    let pd = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    let dir = PathBuf::from(pd).join(SERVICE_DIR);
    let _ = fs::create_dir_all(&dir);
    dir
}

pub fn state_path() -> PathBuf {
    programdata_dir().join(STATE_FILE)
}

pub fn vault_dir() -> PathBuf {
    let pd = std::env::var("PROGRAMDATA").unwrap_or_else(|_| "C:\\ProgramData".to_string());
    PathBuf::from(pd).join(SERVICE_DIR)
}

pub fn vault_path_appdata() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("InnologyBD\\OmniLock\\vault.enc")
}

pub fn vault_path_programdata() -> PathBuf {
    programdata_dir().join(VAULT_FILE)
}

pub fn load_state() -> LockedItemsState {
    let path = state_path();
    if path.exists() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(state) = serde_json::from_str(&data) {
                return state;
            }
        }
    }
    LockedItemsState::default()
}

pub fn save_state(state: &LockedItemsState) -> Result<(), String> {
    let json = serde_json::to_string_pretty(state).map_err(|e| e.to_string())?;
    fs::write(state_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn sync_vault_to_programdata() {
    let src = vault_path_appdata();
    let dst = vault_path_programdata();
    if src.exists() {
        let _ = fs::copy(&src, &dst);
    }
}
