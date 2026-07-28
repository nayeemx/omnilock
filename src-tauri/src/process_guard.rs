use sysinfo::System;
use sha2::{Digest, Sha256};
use std::fs;
use std::sync::{Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use tauri::Emitter;
use tauri::Manager;
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_TERMINATE,
};

use crate::models::{LockedApp, UnlockTarget};

static LOCKED_APPS: OnceLock<Mutex<Vec<LockedApp>>> = OnceLock::new();
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

fn locked_apps_store() -> &'static Mutex<Vec<LockedApp>> {
    LOCKED_APPS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn update_locked_apps(apps: Vec<LockedApp>) {
    if let Ok(mut guard) = locked_apps_store().lock() {
        *guard = apps;
    }
}

pub fn enumerate_processes() -> Vec<(String, String, String)> {
    let mut sys = System::new_all();
    sys.refresh_processes();

    let mut results = Vec::new();
    for (_pid, process) in sys.processes() {
        let name = process.name().to_string();
        let path = process.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        results.push((name, path, String::new()));
    }
    results
}

pub fn enumerate_installed_apps() -> Vec<(String, String, String)> {
    let mut results = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let hives: Vec<(&str, &str)> = vec![
        ("HKLM", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
        ("HKLM", r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall"),
        ("HKCU", r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall"),
    ];

    for (hive, subkey) in hives {
        let full_key = format!("{}\\{}", hive, subkey);
        if let Ok(output) = crate::hidden_cmd("reg")
            .args(["query", &full_key, "/s"])
            .output()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();
            let mut current_path = String::new();

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("DisplayName") {
                    if let Some(val) = trimmed.split_once("REG_SZ").map(|(_, v)| v.trim().to_string()) {
                        current_name = val;
                    }
                } else if trimmed.starts_with("InstallLocation") {
                    if let Some(val) = trimmed.split_once("REG_SZ").map(|(_, v)| v.trim().to_string()) {
                        current_path = val;
                    }
                } else if trimmed.starts_with("DisplayVersion") || trimmed.is_empty() || trimmed.starts_with("HKEY_") {
                    if !current_name.is_empty() && !seen.contains(&current_name) {
                        seen.insert(current_name.clone());
                        results.push((current_name.clone(), current_path.clone(), String::new()));
                    }
                    current_name = String::new();
                    current_path = String::new();
                }
            }
            if !current_name.is_empty() && !seen.contains(&current_name) {
                seen.insert(current_name.clone());
                results.push((current_name, current_path, String::new()));
            }
        }
    }

    results.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()));
    results
}

pub fn compute_file_sha256(path: &str) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("Cannot read binary: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn start_process_monitor(app_handle: tauri::AppHandle) {
    if MONITOR_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        while MONITOR_RUNNING.load(Ordering::SeqCst) {
            let apps = locked_apps_store().lock().map(|g| g.clone()).unwrap_or_default();

            if apps.is_empty() {
                // No locked apps — sleep longer to save CPU
                std::thread::sleep(std::time::Duration::from_secs(5));
                continue;
            }

            let mut sys = System::new_all();
            sys.refresh_processes();

            for app in &apps {
                if !app.enabled {
                    continue;
                }

                for (pid, process) in sys.processes() {
                    let process_path = process.exe()
                        .map(|p| p.to_string_lossy().to_string())
                        .unwrap_or_default();
                    let process_name = process.name().to_string();

                    let matches = process_path.eq_ignore_ascii_case(&app.path)
                        || process_name.eq_ignore_ascii_case(&app.name);

                    if matches {
                        if !app.sha256.is_empty() {
                            let current_hash = compute_file_sha256(&process_path)
                                .unwrap_or_default();
                            if !current_hash.is_empty() && current_hash != app.sha256 {
                                continue;
                            }
                        }

                        unsafe {
                            let handle = OpenProcess(PROCESS_TERMINATE, 0, pid.as_u32());
                            if handle != std::ptr::null_mut() {
                                TerminateProcess(handle, 1);
                                windows_sys::Win32::Foundation::CloseHandle(handle);
                            }
                        }

                        let target = UnlockTarget {
                            target_type: "app".to_string(),
                            target_id: app.path.clone(),
                            display_name: app.name.clone(),
                        };

                        if let Ok(mut guard) = crate::UNLOCK_TARGET.get_or_init(|| Mutex::new(None)).lock() {
                            *guard = Some(target.clone());
                        }

                        let _ = app_handle.emit("app-blocked", &target);

                        if let Some(widget) = app_handle.get_webview_window("widget") {
                            let _ = widget.show();
                            let _ = widget.set_focus();
                            let _ = widget.emit("unlock-target", &target);
                        }

                        break;
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(1000));
        }

        MONITOR_RUNNING.store(false, Ordering::SeqCst);
    });
}

pub fn stop_process_monitor() {
    MONITOR_RUNNING.store(false, Ordering::SeqCst);
}
