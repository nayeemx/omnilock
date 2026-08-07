use sysinfo::System;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock, atomic::{AtomicBool, Ordering}};
use std::time::Instant;
use tauri::Emitter;
use tauri::Manager;
use windows_sys::Win32::System::Threading::{
    OpenProcess, TerminateProcess, PROCESS_TERMINATE,
};

use crate::models::{LockedApp, UnlockTarget};

static LOCKED_APPS: OnceLock<Mutex<Vec<LockedApp>>> = OnceLock::new();
static LOCKED_FOLDERS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static UNLOCKED_FOLDERS: OnceLock<Mutex<Vec<UnlockedFolderState>>> = OnceLock::new();
static PROMPTED_FOLDERS: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
static MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);
static FOLDER_MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

#[derive(Debug, Clone)]
#[allow(dead_code)]
struct UnlockedFolderState {
    path: String,
    unlocked_at: Instant,
}

fn locked_apps_store() -> &'static Mutex<Vec<LockedApp>> {
    LOCKED_APPS.get_or_init(|| Mutex::new(Vec::new()))
}

pub fn update_locked_apps(apps: Vec<LockedApp>) {
    if let Ok(mut guard) = locked_apps_store().lock() {
        *guard = apps;
    }
}

pub fn update_locked_folders(folders: Vec<String>) {
    if let Ok(mut guard) = LOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        *guard = folders;
    }
}

pub fn notify_folder_unlocked(path: &str) {
    if let Ok(mut guard) = UNLOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        if !guard.iter().any(|u| u.path == path) {
            guard.push(UnlockedFolderState {
                path: path.to_string(),
                unlocked_at: Instant::now(),
            });
        }
    }
    clear_prompted_folder(path);
}

fn clear_prompted_folder(path: &str) {
    if let Ok(mut guard) = PROMPTED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        let norm = normalize_path(path);
        guard.retain(|p| normalize_path(p) != norm);
    }
}

pub fn notify_folder_locked(path: &str) {
    if let Ok(mut guard) = UNLOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        guard.retain(|u| u.path != path);
    }
}

pub fn relock_all_unlocked_folders() {
    let paths: Vec<String> = UNLOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new()))
        .lock()
        .map(|g| g.iter().map(|u| u.path.clone()).collect())
        .unwrap_or_default();

    let key = crate::get_active_file_key();

    for path in &paths {
        if let Some(ref k) = key {
            let _ = crate::file_locker::lock_folder(path, k);
        }
    }

    if let Ok(mut guard) = UNLOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
        guard.clear();
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

fn get_explorer_paths() -> Vec<String> {
    crate::shell_access::get_explorer_paths()
}

fn normalize_path(p: &str) -> String {
    p.trim_end_matches('\\').trim_end_matches('/').to_lowercase()
}

pub fn start_file_access_monitor(app_handle: tauri::AppHandle) {
    if FOLDER_MONITOR_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(5));

        while FOLDER_MONITOR_RUNNING.load(Ordering::SeqCst) {
            let folders = LOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new()))
                .lock()
                .map(|g| g.clone())
                .unwrap_or_default();

            let explorer_paths: Vec<String> = if folders.is_empty() {
                Vec::new()
            } else {
                get_explorer_paths()
            };

            for folder in &folders {
                let norm_folder = normalize_path(folder);

                let is_in_explorer = explorer_paths.iter().any(|p| {
                    let norm_p = normalize_path(p);
                    norm_p == norm_folder || norm_p.starts_with(&format!("{}\\", norm_folder))
                });

                let already_unlocked = UNLOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new()))
                    .lock()
                    .map(|g| g.iter().any(|u| normalize_path(&u.path) == norm_folder))
                    .unwrap_or(false);

                let already_prompted = PROMPTED_FOLDERS.get_or_init(|| Mutex::new(Vec::new()))
                    .lock()
                    .map(|g| g.iter().any(|p| normalize_path(p) == norm_folder))
                    .unwrap_or(false);

                if is_in_explorer && !already_unlocked && !already_prompted {
                    let display_name = Path::new(folder)
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| folder.clone());

                    let target = UnlockTarget {
                        target_type: "folder".to_string(),
                        target_id: folder.clone(),
                        display_name,
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

                    // Only prompt once per folder, so the widget does not steal
                    // focus on every 2s poll while the user works elsewhere.
                    if let Ok(mut guard) = PROMPTED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
                        if !guard.iter().any(|p| normalize_path(p) == norm_folder) {
                            guard.push(folder.clone());
                        }
                    }
                }
            }

            // Re-lock folders: on Explorer close OR time-based auto re-lock
            {
                let mut unlocked_guard = match UNLOCKED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
                    Ok(g) => g,
                    Err(_) => {
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        continue;
                    }
                };

                let auto_lock_min = crate::auto_lock::get_auto_lock_minutes();
                let elapsed_threshold = if auto_lock_min > 0 {
                    Some(std::time::Duration::from_secs(auto_lock_min as u64 * 60))
                } else {
                    None
                };

                let mut to_relock: Vec<usize> = Vec::new();
                for (i, uf) in unlocked_guard.iter().enumerate() {
                    let norm_path = normalize_path(&uf.path);
                    let still_open = explorer_paths.iter().any(|p| {
                        let norm_p = normalize_path(p);
                        norm_p == norm_path || norm_p.starts_with(&format!("{}\\", norm_path))
                    });

                    let time_expired = elapsed_threshold
                        .map(|threshold| uf.unlocked_at.elapsed() >= threshold)
                        .unwrap_or(false);

                    if !still_open || time_expired {
                        to_relock.push(i);
                    }
                }

                for i in to_relock.into_iter().rev() {
                    let uf = unlocked_guard.remove(i);
                    clear_prompted_folder(&uf.path);
                    if let Some(ref k) = crate::get_active_file_key() {
                        let _ = crate::file_locker::lock_folder(&uf.path, k);
                    }
                }
            }

            // Forget prompt state for folders no longer open in Explorer, so
            // they can prompt again the next time the user opens them.
            if let Ok(mut guard) = PROMPTED_FOLDERS.get_or_init(|| Mutex::new(Vec::new())).lock() {
                guard.retain(|p| {
                    let norm = normalize_path(p);
                    explorer_paths.iter().any(|e| {
                        let ne = normalize_path(e);
                        ne == norm || ne.starts_with(&format!("{}\\", norm))
                    })
                });
            }

            std::thread::sleep(std::time::Duration::from_secs(2));
        }

        FOLDER_MONITOR_RUNNING.store(false, Ordering::SeqCst);
    });
}

pub fn stop_file_access_monitor() {
    FOLDER_MONITOR_RUNNING.store(false, Ordering::SeqCst);
}
