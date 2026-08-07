use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultStatusDto {
    pub initialized: bool,
    pub totp_enabled: bool,
    pub publisher: String,
    pub version: String,
    pub github_connected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfigDto {
    pub locked_apps: Vec<LockedApp>,
    pub system_presets: SystemPresets,
    pub installer_guard_enabled: bool,
    pub locked_files: Vec<String>,
    pub locked_folders: Vec<String>,
    pub locked_drives: Vec<String>,
    pub auto_lock_minutes: u32,
    pub totp_enabled: bool,
    pub recovery_key: String,
    pub security_question: String,
    pub usb_key_enabled: bool,
    pub usb_key_drive_label: String,
    pub cloud_sync_enabled: bool,
    pub github_username: String,
    pub biometric_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubSyncStatusDto {
    pub connected: bool,
    pub github_user: Option<String>,
    pub avatar_url: Option<String>,
    pub last_sync: Option<u64>,
    pub device_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitHubDeviceFlowDto {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WatchdogStatusDto {
    pub pid: u32,
    pub uptime_secs: u64,
    pub process_count: usize,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfoDto {
    pub os: String,
    pub arch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnlockTarget {
    pub target_type: String,
    pub target_id: String,
    pub display_name: String,
}

impl From<&VaultConfig> for VaultConfigDto {
    fn from(config: &VaultConfig) -> Self {
        Self {
            locked_apps: config.locked_apps.clone(),
            system_presets: config.system_presets.clone(),
            installer_guard_enabled: config.installer_guard_enabled,
            locked_files: config.locked_files.clone(),
            locked_folders: config.locked_folders.clone(),
            locked_drives: config.locked_drives.clone(),
            auto_lock_minutes: config.auto_lock_minutes,
            totp_enabled: config.totp_enabled,
            recovery_key: config.recovery_key.clone(),
            security_question: config.security_question.clone(),
            usb_key_enabled: config.usb_key_enabled,
            usb_key_drive_label: config.usb_key_drive_label.clone(),
            cloud_sync_enabled: config.cloud_sync_enabled,
            github_username: config.github_username.clone(),
            biometric_enabled: config.biometric_enabled,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetupPayload {
    pub master_password: String,
    pub security_question: String,
    pub security_answer: String,
    pub totp_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthPayload {
    pub master_password: String,
    pub totp_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionToken {
    pub token: String,
    pub expires_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    pub password_hash: Vec<u8>,
    pub password_salt: Vec<u8>,
    pub security_question: String,
    pub security_answer_hash: Vec<u8>,
    pub recovery_key: String,
    #[serde(default)]
    pub totp_enabled: bool,
    pub totp_secret: String,
    pub locked_apps: Vec<LockedApp>,
    pub system_presets: SystemPresets,
    pub installer_guard_enabled: bool,
    pub locked_files: Vec<String>,
    pub locked_folders: Vec<String>,
    pub locked_drives: Vec<String>,
    pub auto_lock_minutes: u32,
    #[serde(default)]
    pub usb_key_enabled: bool,
    #[serde(default)]
    pub usb_key_drive_serial: u32,
    #[serde(default)]
    pub usb_key_drive_label: String,
    #[serde(default)]
    pub github_username: String,
    #[serde(default)]
    pub github_user_id: u64,
    #[serde(default)]
    pub cloud_sync_enabled: bool,
    #[serde(default)]
    pub biometric_enabled: bool,
    /// 32-byte random key used to AES-256-GCM encrypt locked files.
    /// Generated once at vault creation, never rotated when password changes.
    #[serde(default)]
    pub file_encryption_key: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedApp {
    pub name: String,
    pub path: String,
    pub sha256: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SystemPresets {
    pub task_manager: bool,
    pub control_panel: bool,
    pub registry_editor: bool,
    pub powershell: bool,
    pub cmd: bool,
    pub system_restore: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedVault {
    pub header: [u8; 4],
    #[serde(default)]
    pub version: u32,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            password_hash: Vec::new(),
            password_salt: Vec::new(),
            security_question: String::new(),
            security_answer_hash: Vec::new(),
            recovery_key: String::new(),
            totp_enabled: false,
            totp_secret: String::new(),
            locked_apps: Vec::new(),
            system_presets: SystemPresets::default(),
            installer_guard_enabled: false,
            locked_files: Vec::new(),
            locked_folders: Vec::new(),
            locked_drives: Vec::new(),
            auto_lock_minutes: 5,
            usb_key_enabled: false,
            usb_key_drive_serial: 0,
            usb_key_drive_label: String::new(),
            github_username: String::new(),
            github_user_id: 0,
            cloud_sync_enabled: false,
            biometric_enabled: false,
            file_encryption_key: Vec::new(),
        }
    }
}
