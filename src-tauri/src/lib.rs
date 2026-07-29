#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod models;
pub mod vault;
pub mod auth;
pub mod totp;
pub mod process_guard;
pub mod system_presets;
pub mod installer_guard;
pub mod panic_hotkey;
pub mod biometric;
pub mod file_locker;
pub mod drive_locker;
pub mod watchdog;
pub mod auto_lock;
pub mod usb_key;
pub mod service_client;
pub mod github_sync;
pub mod system_monitor;
pub mod logger;
pub mod diagnostics;

use tauri::State;
use tauri::Manager;
use tauri::Emitter;
use std::sync::{Mutex, OnceLock};

use models::*;

pub static UNLOCK_TARGET: OnceLock<Mutex<Option<UnlockTarget>>> = OnceLock::new();

/// Create a process Command with CREATE_NO_WINDOW to prevent visible console flashes.
#[cfg(target_os = "windows")]
pub fn hidden_cmd(program: &str) -> std::process::Command {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x08000000;
    let mut cmd = std::process::Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

struct AppState {
    session_token: Mutex<Option<SessionToken>>,
    vault_config: Mutex<Option<VaultConfig>>,
    password: Mutex<Option<String>>,
}

fn require_valid_session(state: &AppState) -> Result<(), String> {
    let session_guard = state.session_token.lock().map_err(|e| e.to_string())?;
    let token = session_guard.as_ref().ok_or("Session not unlocked")?;
    if auth::is_session_expired(token) {
        drop(session_guard);
        let mut session_guard = state.session_token.lock().map_err(|e| e.to_string())?;
        *session_guard = None;
        return Err("Session expired. Please log in again.".to_string());
    }
    Ok(())
}

fn save_locked_items_summary(config: &VaultConfig) {
    let mut targets = Vec::new();
    for path in &config.locked_files {
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        targets.push(UnlockTarget {
            target_type: "file".to_string(),
            target_id: path.clone(),
            display_name: name,
        });
    }
    for path in &config.locked_folders {
        let name = std::path::Path::new(path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| path.clone());
        targets.push(UnlockTarget {
            target_type: "folder".to_string(),
            target_id: path.clone(),
            display_name: name,
        });
    }
    for app in &config.locked_apps {
        targets.push(UnlockTarget {
            target_type: "app".to_string(),
            target_id: app.path.clone(),
            display_name: app.name.clone(),
        });
    }
    for drive in &config.locked_drives {
        targets.push(UnlockTarget {
            target_type: "drive".to_string(),
            target_id: drive.clone(),
            display_name: format!("{}:\\", drive),
        });
    }
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let path = std::path::PathBuf::from(appdata)
        .join("InnologyBD\\OmniLock\\locked_items.json");
    let _ = std::fs::create_dir_all(path.parent().unwrap());
    if let Ok(json) = serde_json::to_string(&targets) {
        let _ = std::fs::write(path, json);
    }
}

#[tauri::command]
fn cmd_get_vault_status(_state: State<'_, AppState>) -> VaultStatusDto {
    let totp_enabled = vault::load_vault_meta();
    let sync_status = github_sync::get_sync_status();
    VaultStatusDto {
        initialized: vault::vault_exists(),
        totp_enabled,
        publisher: "InnologyBD".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        github_connected: sync_status.connected,
    }
}

#[tauri::command]
fn cmd_get_vault_config(state: State<'_, AppState>) -> Result<VaultConfigDto, String> {
    require_valid_session(&state)?;
    let config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_ref().ok_or("Session not unlocked")?;
    Ok(VaultConfigDto::from(config))
}

#[tauri::command]
fn cmd_setup_vault(payload: SetupPayload) -> Result<(), String> {
    auth::setup_vault(payload)
}

#[tauri::command]
fn cmd_unlock_session(
    state: State<'_, AppState>,
    auth_payload: AuthPayload,
    app: tauri::AppHandle,
) -> Result<SessionToken, String> {
    let token = auth::unlock_session(auth_payload.clone())?;
    let config = vault::decrypt_vault(&auth_payload.master_password)?;
    let auto_lock_min = config.auto_lock_minutes;

    let mut session_guard = state.session_token.lock().map_err(|e| e.to_string())?;
    *session_guard = Some(token.clone());

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    *config_guard = Some(config);

    let mut password_guard = state.password.lock().map_err(|e| e.to_string())?;
    *password_guard = Some(auth_payload.master_password);

    auto_lock::set_auto_lock_minutes(auto_lock_min);
    auto_lock::start_auto_lock_monitor();

    let current_config = config_guard.as_ref().unwrap();
    process_guard::update_locked_apps(current_config.locked_apps.clone());
    process_guard::start_process_monitor(app);

    Ok(token)
}

#[tauri::command]
fn cmd_toggle_system_preset(
    state: State<'_, AppState>,
    preset_id: String,
    enabled: bool,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    match preset_id.as_str() {
        "task_manager" => config.system_presets.task_manager = enabled,
        "control_panel" => config.system_presets.control_panel = enabled,
        "registry_editor" => config.system_presets.registry_editor = enabled,
        "powershell" => config.system_presets.powershell = enabled,
        "cmd" => config.system_presets.cmd = enabled,
        "system_restore" => config.system_presets.system_restore = enabled,
        _ => return Err(format!("Unknown preset: {}", preset_id)),
    }
    let presets = config.system_presets.clone();
    system_presets::apply_system_presets(&presets)?;
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_toggle_installer_guard(
    state: State<'_, AppState>,
    enabled: bool,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.installer_guard_enabled = enabled;
    if enabled {
        installer_guard::monitor_installer_guard(true);
    } else {
        installer_guard::stop_installer_guard();
    }
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_lock_now() -> Result<(), String> {
    panic_hotkey::panic_lock();
    Ok(())
}

#[tauri::command]
fn cmd_add_locked_drive(
    state: State<'_, AppState>,
    drive_letter: String,
) -> Result<(), String> {
    require_valid_session(&state)?;
    logger::log("DRIVE", &format!("lock_drive: {}", drive_letter));
    match drive_locker::lock_drive(&drive_letter) {
        Ok(()) => logger::log("DRIVE", &format!("lock_drive ok: {}", drive_letter)),
        Err(e) => {
            logger::log("DRIVE", &format!("lock_drive failed: {} err={}", drive_letter, e));
            return Err(e);
        }
    }
    service_client::notify_lock_drive(&drive_letter);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if !config.locked_drives.contains(&drive_letter) {
        config.locked_drives.push(drive_letter);
    }
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    save_locked_items_summary(config);
    Ok(())
}

#[tauri::command]
fn cmd_remove_locked_drive(
    state: State<'_, AppState>,
    drive_letter: String,
) -> Result<(), String> {
    require_valid_session(&state)?;
    logger::log("DRIVE", &format!("unlock_drive: {}", drive_letter));
    let remaining = {
        let config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
        let config = config_guard.as_ref().ok_or("Session not unlocked")?;
        config.locked_drives.iter()
            .filter(|d| d != &&drive_letter)
            .cloned()
            .collect::<Vec<_>>()
    };
    match drive_locker::unlock_drive(&drive_letter, &remaining) {
        Ok(()) => logger::log("DRIVE", &format!("unlock_drive ok: {}", drive_letter)),
        Err(e) => {
            logger::log("DRIVE", &format!("unlock_drive failed: {} err={}", drive_letter, e));
            return Err(e);
        }
    }
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    service_client::notify_unlock_item(&format!("{}:\\", drive_letter), password);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_drives.retain(|d| d != &drive_letter);
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_add_locked_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    require_valid_session(&state)?;
    logger::log("LOCK", &format!("lock_file start: {}", path));
    match file_locker::lock_file(&path) {
        Ok(()) => logger::log("LOCK", &format!("lock_file ok: {}", path)),
        Err(e) => {
            logger::log("LOCK", &format!("lock_file failed: {} err={}", path, e));
            return Err(e);
        }
    }
    let verified = file_locker::verify_lock(&path).unwrap_or(false);
    logger::log("LOCK", &format!("lock_file verify={} path={}", verified, path));
    service_client::notify_lock_file(&path);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if !config.locked_files.contains(&path) {
        config.locked_files.push(path.clone());
    }
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    save_locked_items_summary(config);
    if verified { Ok("locked".to_string()) } else { Ok("locked_unverified".to_string()) }
}

#[tauri::command]
fn cmd_remove_locked_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    require_valid_session(&state)?;
    logger::log("UNLOCK", &format!("unlock_file start: {}", path));
    match file_locker::unlock_file(&path) {
        Ok(()) => logger::log("UNLOCK", &format!("unlock_file ok: {}", path)),
        Err(e) => {
            logger::log("UNLOCK", &format!("unlock_file failed: {} err={}", path, e));
            return Err(e);
        }
    }
    let verified = file_locker::verify_lock(&path).unwrap_or(true);
    logger::log("UNLOCK", &format!("unlock_file verify={} path={}", verified, path));
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    service_client::notify_unlock_item(&path, password);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_files.retain(|f| f != &path);
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    save_locked_items_summary(config);
    if !verified { Ok("unlocked".to_string()) } else { Ok("unlock_failed".to_string()) }
}

#[tauri::command]
fn cmd_add_locked_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    require_valid_session(&state)?;
    logger::log("LOCK", &format!("lock_folder start: {}", path));
    match file_locker::lock_folder(&path) {
        Ok(()) => logger::log("LOCK", &format!("lock_folder ok: {}", path)),
        Err(e) => {
            logger::log("LOCK", &format!("lock_folder failed: {} err={}", path, e));
            return Err(e);
        }
    }
    let verified = file_locker::verify_lock(&path).unwrap_or(false);
    logger::log("LOCK", &format!("lock_folder verify={} path={}", verified, path));
    service_client::notify_lock_folder(&path);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if !config.locked_folders.contains(&path) {
        config.locked_folders.push(path.clone());
    }
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    save_locked_items_summary(config);
    if verified { Ok("locked".to_string()) } else { Ok("locked_unverified".to_string()) }
}

#[tauri::command]
fn cmd_remove_locked_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    require_valid_session(&state)?;
    logger::log("UNLOCK", &format!("unlock_folder start: {}", path));
    match file_locker::unlock_folder(&path) {
        Ok(()) => logger::log("UNLOCK", &format!("unlock_folder ok: {}", path)),
        Err(e) => {
            logger::log("UNLOCK", &format!("unlock_folder failed: {} err={}", path, e));
            return Err(e);
        }
    }
    let verified = file_locker::verify_lock(&path).unwrap_or(true);
    logger::log("UNLOCK", &format!("unlock_folder verify={} path={}", verified, path));
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    service_client::notify_unlock_item(&path, password);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_folders.retain(|f| f != &path);
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    save_locked_items_summary(config);
    if !verified { Ok("unlocked".to_string()) } else { Ok("unlock_failed".to_string()) }
}

#[tauri::command]
fn cmd_rescue_unlock(path: String) -> Result<String, String> {
    logger::log("RESCUE", &format!("rescue_unlock start: {}", path));
    if !std::path::Path::new(&path).exists() {
        logger::log("RESCUE", &format!("rescue_unlock path not found: {}", path));
        return Err(format!("Path does not exist: {}", path));
    }

    let verify_result = file_locker::verify_lock(&path);
    let has_deny = match &verify_result {
        Ok(true) => true,
        // ACCESS_DENIED means the lock is so strict we can't even read — definitely locked
        Err(e) if e.contains("err=5") || e.contains("err = 5") || e.contains("ACCESS_DENIED") => true,
        Ok(false) => false,
        Err(_) => false,
    };
    logger::log("RESCUE", &format!("rescue_unlock verify_lock={:?} has_deny={} path={}", verify_result, has_deny, path));
    if !has_deny {
        return Ok("not_locked".to_string());
    }

    match file_locker::unlock_file(&path) {
        Ok(()) => logger::log("RESCUE", &format!("rescue_unlock unlock ok: {}", path)),
        Err(e) => {
            logger::log("RESCUE", &format!("rescue_unlock unlock failed: {} err={}", path, e));
            return Err(e);
        }
    }
    let still_locked = file_locker::verify_lock(&path).unwrap_or(false);
    logger::log("RESCUE", &format!("rescue_unlock still_locked={} path={}", still_locked, path));
    if still_locked {
        return Ok("failed".to_string());
    }
    Ok("rescued".to_string())
}

#[tauri::command]
fn cmd_toggle_locked_app(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if let Some(app) = config.locked_apps.iter_mut().find(|a| a.name == name) {
        app.enabled = enabled;
        if enabled {
            let _ = file_locker::lock_file(&app.path);
            service_client::notify_lock_file(&app.path);
        } else {
            let _ = file_locker::unlock_file(&app.path);
            let password_guard = state.password.lock().map_err(|e| e.to_string())?;
            let password = password_guard.as_ref().ok_or("No password in session")?;
            service_client::notify_unlock_item(&app.path, password);
        }
    } else {
        return Err(format!("App not found: {}", name));
    }
    let apps = config.locked_apps.clone();
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    process_guard::update_locked_apps(apps);
    save_locked_items_summary(config);
    Ok(())
}

#[tauri::command]
fn cmd_add_locked_app(
    state: State<'_, AppState>,
    name: String,
    path: String,
    _sha256: String,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let _ = file_locker::lock_file(&path);
    service_client::notify_lock_file(&path);
    let sha256 = process_guard::compute_file_sha256(&path).unwrap_or_default();
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if !config.locked_apps.iter().any(|a| a.name == name) {
        config.locked_apps.push(LockedApp {
            name,
            path,
            sha256,
            enabled: true,
        });
    }
    let apps = config.locked_apps.clone();
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    process_guard::update_locked_apps(apps);
    save_locked_items_summary(config);
    Ok(())
}

#[tauri::command]
fn cmd_remove_locked_app(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if let Some(app) = config.locked_apps.iter().find(|a| a.name == name) {
        let _ = file_locker::unlock_file(&app.path);
        let password_guard = state.password.lock().map_err(|e| e.to_string())?;
        let password = password_guard.as_ref().ok_or("No password in session")?;
        service_client::notify_unlock_item(&app.path, password);
    }
    config.locked_apps.retain(|a| a.name != name);
    let apps = config.locked_apps.clone();
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    process_guard::update_locked_apps(apps);
    save_locked_items_summary(config);
    Ok(())
}

#[tauri::command]
fn cmd_generate_totp() -> Result<String, String> {
    Ok(totp::generate_totp_secret())
}

#[tauri::command]
fn cmd_generate_totp_qr(secret: String) -> Result<String, String> {
    totp::generate_qr_data_uri(&secret)
}

#[tauri::command]
fn cmd_enable_2fa(
    state: State<'_, AppState>,
    secret: String,
    code: String,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let valid = totp::verify_totp_code(&secret, &code)?;
    if !valid {
        return Err("Invalid TOTP code. Make sure you scanned the QR code correctly.".to_string());
    }
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.totp_enabled = true;
    config.totp_secret = secret.clone();
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    vault::save_vault_meta(true)?;
    Ok(())
}

#[tauri::command]
fn cmd_disable_2fa(
    state: State<'_, AppState>,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.totp_enabled = false;
    config.totp_secret = String::new();
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    vault::save_vault_meta(false)?;
    Ok(())
}

#[tauri::command]
fn cmd_list_drives() -> Vec<String> {
    drive_locker::list_available_drives()
}

#[tauri::command]
fn cmd_list_processes() -> Vec<(String, String, String)> {
    process_guard::enumerate_processes()
}

#[tauri::command]
fn cmd_list_installed_apps() -> Vec<(String, String, String)> {
    process_guard::enumerate_installed_apps()
}

#[tauri::command]
fn cmd_set_auto_lock(
    state: State<'_, AppState>,
    minutes: u32,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.auto_lock_minutes = minutes;
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    auto_lock::set_auto_lock_minutes(minutes);
    Ok(())
}

#[tauri::command]
fn cmd_get_security_question() -> Result<String, String> {
    let recovery = vault::load_vault_recovery()?;
    Ok(recovery.security_question)
}

#[tauri::command]
fn cmd_get_recovery_key(state: State<'_, AppState>) -> Result<String, String> {
    require_valid_session(&state)?;
    let config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_ref().ok_or("Session not unlocked")?;
    Ok(config.recovery_key.clone())
}

#[tauri::command]
fn cmd_recover_with_key(
    new_password: String,
    recovery_key: String,
) -> Result<(), String> {
    vault::reset_password_with_key(&new_password, &recovery_key)
}

#[tauri::command]
fn cmd_list_usb_drives() -> Vec<usb_key::UsbDriveInfo> {
    usb_key::list_removable_drives()
}

#[tauri::command]
fn cmd_enroll_usb_key(
    state: State<'_, AppState>,
    drive_letter: String,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_ref().ok_or("Session not unlocked")?;
    let recovery_key = config.recovery_key.clone();
    let drive_info = usb_key::write_key_to_drive(&drive_letter, &recovery_key)?;
    drop(config_guard);
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.usb_key_enabled = true;
    config.usb_key_drive_serial = drive_info.serial;
    config.usb_key_drive_label = drive_info.label;
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_remove_usb_key(
    state: State<'_, AppState>,
) -> Result<(), String> {
    require_valid_session(&state)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.usb_key_enabled = false;
    config.usb_key_drive_serial = 0;
    config.usb_key_drive_label = String::new();
    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn cmd_detect_usb_key() -> Result<Option<String>, String> {
    match usb_key::detect_usb_key(None) {
        Some((_drive, key)) => Ok(Some(key)),
        None => Ok(None),
    }
}

#[tauri::command]
fn cmd_recover_with_usb_key(new_password: String) -> Result<(), String> {
    let (_drive, recovery_key) = usb_key::detect_usb_key(None)
        .ok_or("No OmniLock USB key detected. Insert your enrolled pendrive.")?;
    vault::reset_password_with_key(&new_password, &recovery_key)
}

#[tauri::command]
fn cmd_reset_password(
    new_password: String,
    answer: String,
) -> Result<(), String> {
    vault::reset_password(&new_password, &answer)
}

#[tauri::command]
fn cmd_get_watchdog_status() -> WatchdogStatusDto {
    watchdog::get_watchdog_status()
}

#[tauri::command]
fn cmd_get_system_info() -> SystemInfoDto {
    watchdog::get_system_info()
}

#[tauri::command]
fn cmd_show_widget(
    app: tauri::AppHandle,
    target_type: String,
    target_id: String,
    display_name: String,
) -> Result<(), String> {
    let target = UnlockTarget {
        target_type,
        target_id,
        display_name,
    };

    {
        let target_guard = UNLOCK_TARGET.get_or_init(|| Mutex::new(None));
        *target_guard.lock().map_err(|e| e.to_string())? = Some(target.clone());
    }

    if let Some(widget) = app.get_webview_window("widget") {
        widget.show().map_err(|e| e.to_string())?;
        widget.set_focus().map_err(|e| e.to_string())?;
        widget.emit("unlock-target", &target).map_err(|e| e.to_string())?;
    }

    Ok(())
}

#[tauri::command]
fn cmd_hide_widget(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(widget) = app.get_webview_window("widget") {
        widget.hide().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn cmd_widget_unlock(
    state: State<'_, AppState>,
    password: String,
) -> Result<(), String> {
    vault::decrypt_vault(&password).map_err(|_| "Incorrect password")?;

    let target = {
        let target_guard = UNLOCK_TARGET.get_or_init(|| Mutex::new(None));
        target_guard.lock().map_err(|e| e.to_string())?.clone()
            .ok_or("No unlock target")?
    };

    let mut config = vault::decrypt_vault(&password)?;

    match target.target_type.as_str() {
        "file" => {
            file_locker::unlock_file(&target.target_id)?;
            let verified = file_locker::verify_lock(&target.target_id).unwrap_or(true);
            if verified { return Err("Unlock failed - ACL still present".to_string()); }
            service_client::notify_unlock_item(&target.target_id, &password);
        }
        "folder" => {
            file_locker::unlock_folder(&target.target_id)?;
            let verified = file_locker::verify_lock(&target.target_id).unwrap_or(true);
            if verified { return Err("Unlock failed - ACL still present".to_string()); }
            service_client::notify_unlock_item(&target.target_id, &password);
        }
        "app" => {
            file_locker::unlock_file(&target.target_id)?;
            let verified = file_locker::verify_lock(&target.target_id).unwrap_or(true);
            if verified { return Err("Unlock failed - ACL still present".to_string()); }
            service_client::notify_unlock_item(&target.target_id, &password);
            config.locked_apps.retain(|a| a.path != target.target_id);
        }
        "drive" => {
            let remaining: Vec<String> = config.locked_drives.iter()
                .filter(|d| d.as_str() != target.target_id.as_str())
                .cloned()
                .collect();
            drive_locker::unlock_drive(&target.target_id, &remaining)?;
            service_client::notify_unlock_item(&format!("{}:\\", target.target_id), &password);
            config.locked_drives.retain(|d| d != &target.target_id);
        }
        _ => return Err(format!("Unknown target type: {}", target.target_type)),
    }

    vault::encrypt_vault(&config, &password)?;
    let apps = config.locked_apps.clone();
    process_guard::update_locked_apps(apps);
    save_locked_items_summary(&config);

    {
        let mut target_guard = UNLOCK_TARGET.get_or_init(|| Mutex::new(None)).lock().map_err(|e| e.to_string())?;
        *target_guard = None;
    }

    let mut session_guard = state.session_token.lock().map_err(|e| e.to_string())?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    *session_guard = Some(SessionToken {
        token: password.clone(),
        expires_at: now + 3600,
    });
    drop(session_guard);

    let mut password_guard = state.password.lock().map_err(|e| e.to_string())?;
    *password_guard = Some(password);

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    *config_guard = Some(config);

    Ok(())
}

#[tauri::command]
fn cmd_widget_list_locked() -> Result<Vec<UnlockTarget>, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let summary_path = std::path::PathBuf::from(appdata)
        .join("InnologyBD\\OmniLock\\locked_items.json");
    
    if !summary_path.exists() {
        return Ok(Vec::new());
    }
    
    let data = std::fs::read_to_string(&summary_path)
        .map_err(|e| format!("Failed to read locked items: {}", e))?;
    let targets: Vec<UnlockTarget> = serde_json::from_str(&data)
        .map_err(|e| format!("Failed to parse locked items: {}", e))?;
    Ok(targets)
}

#[tauri::command]
fn cmd_get_service_status() -> bool {
    service_client::is_service_running()
}

#[tauri::command]
async fn cmd_github_start_device_flow() -> Result<models::GitHubDeviceFlowDto, String> {
    let resp = github_sync::start_device_flow().await?;
    Ok(models::GitHubDeviceFlowDto {
        device_code: resp.device_code,
        user_code: resp.user_code,
        verification_uri: resp.verification_uri,
        expires_in: resp.expires_in,
        interval: resp.interval,
    })
}

#[tauri::command]
async fn cmd_github_poll_token(
    state: State<'_, AppState>,
    device_code: String,
    interval: u64,
    expires_in: u64,
) -> Result<models::GitHubSyncStatusDto, String> {
    let _token = github_sync::poll_for_token(&device_code, interval, expires_in).await?;
    let status = github_sync::get_sync_status();
    if status.connected {
        if let Ok(mut config_guard) = state.vault_config.lock() {
            if let Some(config) = config_guard.as_mut() {
                config.cloud_sync_enabled = true;
            }
        }
        if let Ok(password_guard) = state.password.lock() {
            if let Some(password) = password_guard.as_ref() {
                if let Ok(config_guard) = state.vault_config.lock() {
                    if let Some(config) = config_guard.as_ref() {
                        let _ = vault::encrypt_vault(config, password);
                    }
                }
            }
        }
    }
    Ok(models::GitHubSyncStatusDto {
        connected: status.connected,
        github_user: status.github_user,
        avatar_url: status.avatar_url,
        last_sync: status.last_sync,
        device_id: status.device_id,
    })
}

#[tauri::command]
fn cmd_github_get_status() -> models::GitHubSyncStatusDto {
    let status = github_sync::get_sync_status();
    models::GitHubSyncStatusDto {
        connected: status.connected,
        github_user: status.github_user,
        avatar_url: status.avatar_url,
        last_sync: status.last_sync,
        device_id: status.device_id,
    }
}

#[tauri::command]
async fn cmd_github_connect_token(
    state: State<'_, AppState>,
    token: String,
) -> Result<models::GitHubSyncStatusDto, String> {
    let user = github_sync::verify_github_token(&token).await?;
    let status = github_sync::connect_with_token(token, user)?;
    if status.connected {
        if let Ok(mut config_guard) = state.vault_config.lock() {
            if let Some(config) = config_guard.as_mut() {
                config.cloud_sync_enabled = true;
            }
        }
        if let Ok(password_guard) = state.password.lock() {
            if let Some(password) = password_guard.as_ref() {
                if let Ok(config_guard) = state.vault_config.lock() {
                    if let Some(config) = config_guard.as_ref() {
                        let _ = vault::encrypt_vault(config, password);
                    }
                }
            }
        }
    }
    Ok(models::GitHubSyncStatusDto {
        connected: status.connected,
        github_user: status.github_user,
        avatar_url: status.avatar_url,
        last_sync: status.last_sync,
        device_id: status.device_id,
    })
}

#[tauri::command]
fn cmd_github_disconnect() -> Result<(), String> {
    github_sync::disconnect_github()
}

#[tauri::command]
fn cmd_backup_vault(dest_dir: String) -> Result<String, String> {
    let dir = vault::vault_dir();
    let dest = std::path::PathBuf::from(&dest_dir);
    std::fs::create_dir_all(&dest).map_err(|e| format!("Cannot create backup folder: {}", e))?;

    let mut files_backed_up = Vec::new();
    let files_to_backup = ["vault.enc", "vault.recovery"];

    for fname in &files_to_backup {
        let src = dir.join(fname);
        if src.exists() {
            let dst = dest.join(fname);
            std::fs::copy(&src, &dst).map_err(|e| format!("Failed to copy {}: {}", fname, e))?;
            files_backed_up.push(fname.to_string());
        }
    }

    let meta_src = dir.join("vault.meta");
    if meta_src.exists() {
        let meta_dst = dest.join("vault.meta");
        std::fs::copy(&meta_src, &meta_dst).map_err(|e| format!("Failed to copy vault.meta: {}", e))?;
        files_backed_up.push("vault.meta".to_string());
    }

    let items_src = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
        + "\\InnologyBD\\OmniLock\\locked_items.json";
    let items_path = std::path::PathBuf::from(&items_src);
    if items_path.exists() {
        let items_dst = dest.join("locked_items.json");
        std::fs::copy(&items_path, &items_dst).map_err(|e| format!("Failed to copy locked_items.json: {}", e))?;
        files_backed_up.push("locked_items.json".to_string());
    }

    Ok(format!("Backed up {} files to: {}", files_backed_up.len(), dest_dir))
}

#[tauri::command]
fn cmd_restore_vault(src_dir: String) -> Result<String, String> {
    let src = std::path::PathBuf::from(&src_dir);
    if !src.exists() {
        return Err("Source folder does not exist".to_string());
    }

    let vault_dir = vault::vault_dir();
    let mut files_restored = Vec::new();
    let files_to_restore = ["vault.enc", "vault.recovery", "vault.meta"];

    for fname in &files_to_restore {
        let src_file = src.join(fname);
        if src_file.exists() {
            let dst = vault_dir.join(fname);
            std::fs::copy(&src_file, &dst).map_err(|e| format!("Failed to copy {}: {}", fname, e))?;
            files_restored.push(fname.to_string());
        }
    }

    let items_dst = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string())
        + "\\InnologyBD\\OmniLock\\locked_items.json";
    let items_src = src.join("locked_items.json");
    if items_src.exists() {
        std::fs::create_dir_all(std::path::PathBuf::from(&items_dst).parent().unwrap()).ok();
        std::fs::copy(&items_src, &items_dst).map_err(|e| format!("Failed to copy locked_items.json: {}", e))?;
        files_restored.push("locked_items.json".to_string());
    }

    if files_restored.is_empty() {
        return Err("No backup files found in the selected folder".to_string());
    }

    Ok(format!("Restored {} files. Please restart OmniLock to apply.", files_restored.len()))
}

#[tauri::command]
async fn cmd_github_sync_to_cloud(
    state: State<'_, AppState>,
) -> Result<models::GitHubSyncStatusDto, String> {
    let vault_data = {
        let password_guard = state.password.lock().map_err(|e| e.to_string())?;
        let password = password_guard.as_ref().ok_or("Session not unlocked")?;
        let config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
        let config = config_guard.as_ref().ok_or("Session not unlocked")?;
        let encrypted = vault::encrypt_vault_to_bytes(config, password)?;
        encrypted
    };
    let status = github_sync::sync_to_cloud(&vault_data).await?;
    {
        let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
        if let Some(config) = config_guard.as_mut() {
            config.cloud_sync_enabled = true;
        }
        let password_guard = state.password.lock().map_err(|e| e.to_string())?;
        let password = password_guard.as_ref().ok_or("No password in session")?;
        let config = config_guard.as_ref().unwrap();
        vault::encrypt_vault(config, password)?;
    }
    Ok(models::GitHubSyncStatusDto {
        connected: status.connected,
        github_user: status.github_user,
        avatar_url: status.avatar_url,
        last_sync: status.last_sync,
        device_id: status.device_id,
    })
}

#[tauri::command]
async fn cmd_github_sync_from_cloud(
    state: State<'_, AppState>,
) -> Result<(), String> {
    let vault_data = github_sync::sync_from_cloud().await?;
    let password = {
        let password_guard = state.password.lock().map_err(|e| e.to_string())?;
        password_guard.as_ref().ok_or("Session not unlocked")?.clone()
    };
    let config = vault::decrypt_vault_from_bytes(&vault_data, &password)?;
    vault::encrypt_vault(&config, &password)?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    *config_guard = Some(config.clone());
    let mut password_guard = state.password.lock().map_err(|e| e.to_string())?;
    *password_guard = Some(password);
    process_guard::update_locked_apps(config.locked_apps.clone());
    save_locked_items_summary(&config);
    Ok(())
}

#[tauri::command]
fn cmd_open_external_url(url: String) -> Result<(), String> {
    open::that(&url).map_err(|e| format!("Failed to open URL: {}", e))
}

#[tauri::command]
async fn cmd_get_system_stats() -> system_monitor::SystemStats {
    system_monitor::get_system_stats_async().await
}

#[tauri::command]
async fn cmd_get_weather(location: Option<String>) -> Result<system_monitor::WeatherData, String> {
    system_monitor::get_weather(location).await
}

#[tauri::command]
fn cmd_check_biometric() -> biometric::BiometricStatus {
    biometric::check_biometric_available()
}

#[tauri::command]
async fn cmd_authenticate_biometric(message: String) -> Result<bool, String> {
    logger::log("BIOMETRIC", "authenticate start");
    let result = biometric::authenticate_biometric(message).await;
    match &result {
        Ok(true) => logger::log("BIOMETRIC", "authenticate ok"),
        Ok(false) => logger::log("BIOMETRIC", "authenticate false"),
        Err(e) => logger::log("BIOMETRIC", &format!("authenticate failed: {}", e)),
    }
    result
}

#[tauri::command]
fn cmd_toggle_biometric(state: State<'_, AppState>, enabled: bool, password: Option<String>) -> Result<(), String> {
    let pw = if let Some(p) = &password {
        p.clone()
    } else {
        let guard = state.password.lock().map_err(|e| e.to_string())?;
        guard.as_ref().ok_or("Session not unlocked")?.clone()
    };

    logger::log("BIOMETRIC", &format!("toggle enabled={}", enabled));
    if enabled {
        match biometric::save_biometric_token(&pw) {
            Ok(()) => logger::log("BIOMETRIC", "save_biometric_token ok"),
            Err(e) => {
                logger::log("BIOMETRIC", &format!("save_biometric_token failed: {}", e));
                return Err(e);
            }
        }
    } else {
        let _ = biometric::remove_biometric_token();
        logger::log("BIOMETRIC", "remove_biometric_token");
    }

    let mut config = vault::decrypt_vault(&pw).map_err(|e| e.to_string())?;
    config.biometric_enabled = enabled;
    vault::encrypt_vault(&config, &pw).map_err(|e| e.to_string())?;
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    *config_guard = Some(config.into());
    Ok(())
}

#[tauri::command]
async fn cmd_biometric_login(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    logger::log("BIOMETRIC", "biometric_login start");
    let password = match biometric::load_biometric_token() {
        Ok(pw) => {
            logger::log("BIOMETRIC", "load_biometric_token ok");
            pw
        }
        Err(e) => {
            logger::log("BIOMETRIC", &format!("load_biometric_token failed: {}", e));
            return Err(e);
        }
    };
    let config = vault::decrypt_vault(&password).map_err(|e| format!("Wrong password: {}", e))?;
    logger::log("BIOMETRIC", "decrypt_vault ok");
    let session_token = auth::create_session_token().map_err(|e| e.to_string())?;
    {
        let mut token_guard = state.session_token.lock().map_err(|e| e.to_string())?;
        *token_guard = Some(session_token);
    }
    {
        let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
        *config_guard = Some(config.clone().into());
    }
    {
        let mut password_guard = state.password.lock().map_err(|e| e.to_string())?;
        *password_guard = Some(password);
    }
    process_guard::update_locked_apps(config.locked_apps.clone());
    process_guard::start_process_monitor(app.clone());
    auto_lock::start_auto_lock_monitor();
    logger::log("BIOMETRIC", "biometric_login ok");
    Ok(())
}

#[tauri::command]
fn cmd_has_biometric_token() -> bool {
    biometric::has_biometric_token()
}

#[tauri::command]
fn cmd_get_diagnostics() -> diagnostics::Diagnostics {
    diagnostics::collect_diagnostics()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    logger::init();

    let app_state = AppState {
        session_token: Mutex::new(None),
        vault_config: Mutex::new(None),
        password: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            cmd_get_vault_status,
            cmd_get_vault_config,
            cmd_setup_vault,
            cmd_unlock_session,
            cmd_toggle_system_preset,
            cmd_toggle_installer_guard,
            cmd_lock_now,
            cmd_add_locked_drive,
            cmd_remove_locked_drive,
            cmd_add_locked_file,
            cmd_remove_locked_file,
            cmd_add_locked_folder,
            cmd_remove_locked_folder,
            cmd_toggle_locked_app,
            cmd_add_locked_app,
            cmd_remove_locked_app,
            cmd_rescue_unlock,
            cmd_generate_totp,
            cmd_generate_totp_qr,
            cmd_enable_2fa,
            cmd_disable_2fa,
            cmd_list_drives,
            cmd_list_processes,
            cmd_list_installed_apps,
            cmd_set_auto_lock,
            cmd_get_security_question,
            cmd_get_recovery_key,
            cmd_recover_with_key,
            cmd_list_usb_drives,
            cmd_enroll_usb_key,
            cmd_remove_usb_key,
            cmd_detect_usb_key,
            cmd_recover_with_usb_key,
            cmd_reset_password,
            cmd_get_watchdog_status,
            cmd_get_system_info,
            cmd_show_widget,
            cmd_hide_widget,
            cmd_widget_unlock,
            cmd_widget_list_locked,
            cmd_get_service_status,
            cmd_github_start_device_flow,
            cmd_github_poll_token,
            cmd_github_get_status,
            cmd_github_connect_token,
            cmd_github_disconnect,
            cmd_github_sync_to_cloud,
            cmd_github_sync_from_cloud,
            cmd_open_external_url,
            cmd_backup_vault,
            cmd_restore_vault,
            cmd_get_system_stats,
            cmd_get_weather,
            cmd_check_biometric,
            cmd_authenticate_biometric,
            cmd_toggle_biometric,
            cmd_biometric_login,
            cmd_has_biometric_token,
            cmd_get_diagnostics,
        ])
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            logger::log("SETUP", "setup hook started");

            use tauri::tray::{TrayIconBuilder, MouseButton, MouseButtonState, TrayIconEvent};
            use tauri::menu::{Menu, MenuItem};

            let show_item = match MenuItem::with_id(app, "show", "Show OmniLock", true, None::<&str>) {
                Ok(item) => item,
                Err(e) => { logger::log("SETUP", &format!("menu item error: {}", e)); return Ok(()); }
            };
            let quit_item = match MenuItem::with_id(app, "quit", "Quit", true, None::<&str>) {
                Ok(item) => item,
                Err(e) => { logger::log("SETUP", &format!("quit item error: {}", e)); return Ok(()); }
            };
            let menu = match Menu::with_items(app, &[&show_item, &quit_item]) {
                Ok(m) => m,
                Err(e) => { logger::log("SETUP", &format!("menu error: {}", e)); return Ok(()); }
            };

            let icon = app.default_window_icon().cloned();
            let tray_builder = TrayIconBuilder::new()
                .tooltip("OmniLock - Enterprise Desktop Security")
                .menu(&menu);

            let tray_builder = if let Some(ic) = &icon {
                tray_builder.icon(ic.clone())
            } else {
                logger::log("SETUP", "no default icon, creating tray without icon");
                tray_builder
            };

            let _tray = match tray_builder
                .on_menu_event(|app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = app.get_webview_window("main") {
                                let _ = w.show();
                                let _ = w.set_focus();
                            }
                        }
                        "quit" => {
                            app.exit(0);
                        }
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)
            {
                Ok(t) => t,
                Err(e) => { logger::log("SETUP", &format!("tray build error: {}", e)); return Ok(()); }
            };

            let _widget = tauri::WebviewWindowBuilder::new(
                app,
                "widget",
                tauri::WebviewUrl::App("index.html?widget".into()),
            )
            .inner_size(420.0, 340.0)
            .center()
            .decorations(false)
            .resizable(false)
            .always_on_top(true)
            .visible(false)
            .build();

            // Auto-start service daemon if not running
            if !service_client::is_service_running() {
                logger::log("SERVICE", "service not running, attempting to start it");
                // Search multiple locations for the service binary
                let exe_dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.to_path_buf()));
                let search_paths = exe_dir.iter().flat_map(|dir| {
                    vec![
                        dir.join("omnilock-svc.exe"),
                        dir.join("resources").join("omnilock-svc.exe"),
                        dir.join("_resources").join("omnilock-svc.exe"),
                    ]
                });
                let svc_path = search_paths.filter(|p| p.exists()).next();
                if let Some(path) = svc_path {
                    match std::process::Command::new(&path)
                        .arg("--standalone")
                        .spawn()
                    {
                        Ok(_) => logger::log("SERVICE", &format!("service started from: {}", path.display())),
                        Err(e) => logger::log("SERVICE", &format!("failed to start service: {}", e)),
                    }
                } else {
                    logger::log("SERVICE", "omnilock-svc.exe not found, skipping service start");
                }
            } else {
                logger::log("SERVICE", "service already running");
            }

            panic_hotkey::start_hotkey_listener();
            watchdog::start_watchdog();
            logger::log("SETUP", "setup hook complete");
            Ok(())
        })
        .on_window_event(|window, event| {
            match window.label() {
                "main" => {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        window.hide().ok();
                        api.prevent_close();
                    }
                }
                "widget" => {
                    if let tauri::WindowEvent::Focused(false) = event {
                        // Don't hide if there's a pending unlock target
                        let has_target = UNLOCK_TARGET.get_or_init(|| Mutex::new(None))
                            .lock().map(|g| g.is_some()).unwrap_or(false);
                        if !has_target {
                            window.hide().ok();
                        }
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running OmniLock");
}
