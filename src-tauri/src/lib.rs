#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

pub mod models;
pub mod vault;
pub mod auth;
pub mod totp;
pub mod process_guard;
pub mod system_presets;
pub mod installer_guard;
pub mod panic_hotkey;
pub mod file_locker;
pub mod drive_locker;
pub mod watchdog;
pub mod auto_lock;
pub mod usb_key;

use tauri::State;
use std::sync::Mutex;

use models::*;

struct AppState {
    session_token: Mutex<Option<SessionToken>>,
    vault_config: Mutex<Option<VaultConfig>>,
    password: Mutex<Option<String>>,
}

#[tauri::command]
fn cmd_get_vault_status(_state: State<'_, AppState>) -> VaultStatusDto {
    let totp_enabled = vault::load_vault_meta();

    VaultStatusDto {
        initialized: vault::vault_exists(),
        totp_enabled,
        publisher: "InnologyBD".to_string(),
        version: "0.0.4".to_string(),
    }
}

#[tauri::command]
fn cmd_get_vault_config(state: State<'_, AppState>) -> Result<VaultConfigDto, String> {
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

    Ok(token)
}

#[tauri::command]
fn cmd_toggle_system_preset(
    state: State<'_, AppState>,
    preset_id: String,
    enabled: bool,
) -> Result<(), String> {
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
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.installer_guard_enabled = enabled;

    if enabled {
        installer_guard::monitor_installer_guard(true);
    }

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_trigger_panic_lock() -> Result<(), String> {
    panic_hotkey::register_panic_hotkey()?;
    Ok(())
}

#[tauri::command]
fn cmd_add_locked_drive(
    state: State<'_, AppState>,
    drive_letter: String,
) -> Result<(), String> {
    drive_locker::lock_drive(&drive_letter)?;

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;

    if !config.locked_drives.contains(&drive_letter) {
        config.locked_drives.push(drive_letter);
    }

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_remove_locked_drive(
    state: State<'_, AppState>,
    drive_letter: String,
) -> Result<(), String> {
    drive_locker::unlock_drive(&drive_letter)?;

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_drives.retain(|d| d != &drive_letter);

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_add_locked_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    file_locker::lock_file(&path)?;

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if !config.locked_files.contains(&path) {
        config.locked_files.push(path);
    }

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_remove_locked_file(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    file_locker::unlock_file(&path)?;

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_files.retain(|f| f != &path);

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_add_locked_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    file_locker::lock_folder(&path)?;

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    if !config.locked_folders.contains(&path) {
        config.locked_folders.push(path);
    }

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_remove_locked_folder(
    state: State<'_, AppState>,
    path: String,
) -> Result<(), String> {
    file_locker::unlock_folder(&path)?;

    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_folders.retain(|f| f != &path);

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_toggle_locked_app(
    state: State<'_, AppState>,
    name: String,
    enabled: bool,
) -> Result<(), String> {
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;

    if let Some(app) = config.locked_apps.iter_mut().find(|a| a.name == name) {
        app.enabled = enabled;
    } else {
        return Err(format!("App not found: {}", name));
    }

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_add_locked_app(
    state: State<'_, AppState>,
    name: String,
    path: String,
    sha256: String,
) -> Result<(), String> {
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

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

    Ok(())
}

#[tauri::command]
fn cmd_remove_locked_app(
    state: State<'_, AppState>,
    name: String,
) -> Result<(), String> {
    let mut config_guard = state.vault_config.lock().map_err(|e| e.to_string())?;
    let config = config_guard.as_mut().ok_or("Session not unlocked")?;
    config.locked_apps.retain(|a| a.name != name);

    let password_guard = state.password.lock().map_err(|e| e.to_string())?;
    let password = password_guard.as_ref().ok_or("No password in session")?;
    vault::encrypt_vault(config, password).map_err(|e| e.to_string())?;

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
fn cmd_verify_totp(secret: String, code: String) -> Result<bool, String> {
    totp::verify_totp_code(&secret, &code)
}

#[tauri::command]
fn cmd_enable_2fa(
    state: State<'_, AppState>,
    secret: String,
    code: String,
) -> Result<(), String> {
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
fn cmd_set_auto_lock(
    state: State<'_, AppState>,
    minutes: u32,
) -> Result<(), String> {
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
fn cmd_reset_password(
    new_password: String,
    answer: String,
) -> Result<(), String> {
    vault::reset_password(&new_password, &answer)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let app_state = AppState {
        session_token: Mutex::new(None),
        vault_config: Mutex::new(None),
        password: Mutex::new(None),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            cmd_get_vault_status,
            cmd_get_vault_config,
            cmd_setup_vault,
            cmd_unlock_session,
            cmd_toggle_system_preset,
            cmd_toggle_installer_guard,
            cmd_trigger_panic_lock,
            cmd_add_locked_drive,
            cmd_remove_locked_drive,
            cmd_add_locked_file,
            cmd_remove_locked_file,
            cmd_add_locked_folder,
            cmd_remove_locked_folder,
            cmd_toggle_locked_app,
            cmd_add_locked_app,
            cmd_remove_locked_app,
            cmd_generate_totp,
            cmd_generate_totp_qr,
            cmd_verify_totp,
            cmd_enable_2fa,
            cmd_disable_2fa,
            cmd_list_drives,
            cmd_list_processes,
            cmd_set_auto_lock,
            cmd_get_security_question,
            cmd_get_recovery_key,
            cmd_recover_with_key,
            cmd_list_usb_drives,
            cmd_enroll_usb_key,
            cmd_remove_usb_key,
            cmd_detect_usb_key,
            cmd_reset_password,
        ])
        .setup(|_app| {
            #[cfg(desktop)]
            {
                _app.handle().plugin(tauri_plugin_updater::Builder::new().build()).ok();
            }
            panic_hotkey::start_hotkey_listener();
            watchdog::start_watchdog();
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running OmniLock");
}
