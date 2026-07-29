use serde::{Deserialize, Serialize};

pub const PIPE_NAME: &str = r"\\.\pipe\OmniLockService";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedItem {
    pub item_type: String,
    pub path: String,
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "cmd", content = "args")]
pub enum SvcRequest {
    Ping,
    GetStatus,
    GetLockedItems,
    LockFile { path: String, display_name: String },
    LockFolder { path: String, display_name: String },
    LockDrive { drive_letter: String, display_name: String },
    LockApp { name: String, path: String, display_name: String },
    UnlockItem { path: String, password: String },
    ForceRemoveLockedItem { path: String },
    SyncVault { vault_data: Vec<u8> },
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SvcResponse {
    Pong,
    Ok { message: String },
    Error { message: String },
    Status { running: bool, locked_count: usize },
    LockedItems(Vec<LockedItem>),
}
