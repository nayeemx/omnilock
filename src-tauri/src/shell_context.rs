use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::logger;

static CONTEXT_MENU_INSTALLED: AtomicBool = AtomicBool::new(false);

pub fn is_installed() -> bool {
    CONTEXT_MENU_INSTALLED.load(Ordering::SeqCst)
}

pub fn install() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Cannot get exe path: {}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();

    install_for("*", &exe_str)?;
    install_for("Directory", &exe_str)?;
    install_for("Drive", &exe_str)?;
    register_omnilock_extension(&exe_str)?;

    CONTEXT_MENU_INSTALLED.store(true, Ordering::SeqCst);
    logger::log("MENU", &format!("Context menu installed for: {}", exe_str));
    Ok(())
}

/// Register only the `.omnilock` file association (no context-menu entries).
/// Called automatically at app startup so double-clicking a locked file opens
/// the unlock widget. Idempotent — safe to call on every launch.
pub fn register_extension_only() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Cannot get exe path: {}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();
    register_omnilock_extension(&exe_str)
}

/// Register the .omnilock file extension so double-clicking an encrypted file
/// opens OmniLock with --open-locked <path>, which shows the unlock widget.
fn register_omnilock_extension(exe_path: &str) -> Result<(), String> {
    // .omnilock → OmniLockFile
    let _ = std::process::Command::new("reg")
        .args(["add", r"HKEY_CLASSES_ROOT\.omnilock", "/ve", "/d", "OmniLockFile", "/f"])
        .output();

    // ProgID description
    let _ = std::process::Command::new("reg")
        .args(["add", r"HKEY_CLASSES_ROOT\OmniLockFile", "/ve", "/d", "OmniLock Encrypted File", "/f"])
        .output();

    // Icon
    let _ = std::process::Command::new("reg")
        .args(["add", r"HKEY_CLASSES_ROOT\OmniLockFile\DefaultIcon", "/ve", "/d", exe_path, "/f"])
        .output();

    // Open command → OmniLock.exe --open-locked "%1"
    let open_cmd = format!("\"{}\" --open-locked \"%1\"", exe_path);
    let _ = std::process::Command::new("reg")
        .args(["add", r"HKEY_CLASSES_ROOT\OmniLockFile\shell\open\command", "/ve", "/d", &open_cmd, "/f"])
        .output();

    // Notify shell
    let _ = std::process::Command::new("ie4uinit.exe").args(["-show"]).output();

    logger::log("MENU", "Registered .omnilock file extension");
    Ok(())
}

fn install_for(reg_type: &str, exe_path: &str) -> Result<(), String> {
    let base = format!(r"HKEY_CLASSES_ROOT\{}\shell\OmniLock", reg_type);

    let _ = std::process::Command::new("reg")
        .args(["add", &base, "/ve", "/d", "Lock with OmniLock", "/f"])
        .output()
        .map_err(|e| format!("reg add failed: {}", e))?;

    let _ = std::process::Command::new("reg")
        .args(["add", &base, "/v", "Icon", "/d", exe_path, "/f"])
        .output()
        .map_err(|e| format!("reg add icon failed: {}", e))?;

    let _ = std::process::Command::new("reg")
        .args(["add", &base, "/v", "MultiSelectModel", "/d", "Player", "/f"])
        .output()
        .ok();

    let cmd_base = format!(r"{}\command", base);
    let cmd_value = format!("\"{}\" --context-menu-lock \"%1\"", exe_path);
    let _ = std::process::Command::new("reg")
        .args(["add", &cmd_base, "/ve", "/d", &cmd_value, "/f"])
        .output()
        .map_err(|e| format!("reg add command failed: {}", e))?;

    let unlock_base = format!(r"HKEY_CLASSES_ROOT\{}\shell\OmniLockUnlock", reg_type);
    let _ = std::process::Command::new("reg")
        .args(["add", &unlock_base, "/ve", "/d", "Unlock with OmniLock", "/f"])
        .output()
        .map_err(|e| format!("reg add unlock base failed: {}", e))?;

    let _ = std::process::Command::new("reg")
        .args(["add", &unlock_base, "/v", "Icon", "/d", exe_path, "/f"])
        .output()
        .map_err(|e| format!("reg add unlock icon failed: {}", e))?;

    let cmd_unlock_base = format!(r"{}\command", unlock_base);
    let cmd_unlock_value = format!("\"{}\" --context-menu-unlock \"%1\"", exe_path);
    let _ = std::process::Command::new("reg")
        .args(["add", &cmd_unlock_base, "/ve", "/d", &cmd_unlock_value, "/f"])
        .output()
        .map_err(|e| format!("reg add unlock command failed: {}", e))?;

    Ok(())
}

pub fn uninstall() -> Result<(), String> {
    for reg_type in &["*", "Directory", "Drive"] {
        let base = format!(r"HKEY_CLASSES_ROOT\{}\shell\OmniLock", reg_type);
        let _ = std::process::Command::new("reg")
            .args(["delete", &base, "/f"])
            .output()
            .map_err(|e| format!("reg delete lock failed: {}", e))?;

        let unlock_base = format!(r"HKEY_CLASSES_ROOT\{}\shell\OmniLockUnlock", reg_type);
        let _ = std::process::Command::new("reg")
            .args(["delete", &unlock_base, "/f"])
            .output()
            .map_err(|e| format!("reg delete unlock failed: {}", e))?;
    }

    CONTEXT_MENU_INSTALLED.store(false, Ordering::SeqCst);
    logger::log("MENU", "Context menu uninstalled");
    Ok(())
}

pub fn handle_context_menu_lock(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    logger::log("MENU", &format!("Context menu lock: {}", path));

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Cannot get exe path: {}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();
    let pass_cmd = format!(
        "\"{}\" --lock-action \"{}\" \"{}\"",
        exe_str, "lock", path
    );

    std::process::Command::new("cmd")
        .args(["/C", "start", "", "\"\"", &pass_cmd])
        .spawn()
        .map_err(|e| format!("Cannot launch OmniLock for lock: {}", e))?;

    Ok("Lock action dispatched".to_string())
}

pub fn handle_context_menu_unlock(path: &str) -> Result<String, String> {
    let p = Path::new(path);
    if !p.exists() {
        return Err(format!("Path does not exist: {}", path));
    }

    logger::log("MENU", &format!("Context menu unlock: {}", path));

    let exe_path = std::env::current_exe()
        .map_err(|e| format!("Cannot get exe path: {}", e))?;
    let exe_str = exe_path.to_string_lossy().to_string();
    let pass_cmd = format!(
        "\"{}\" --lock-action \"{}\" \"{}\"",
        exe_str, "unlock", path
    );

    std::process::Command::new("cmd")
        .args(["/C", "start", "", "\"\"", &pass_cmd])
        .spawn()
        .map_err(|e| format!("Cannot launch OmniLock for unlock: {}", e))?;

    Ok("Unlock action dispatched".to_string())
}
