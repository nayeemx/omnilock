use sysinfo::System;
use sha2::{Digest, Sha256};
use std::fs;
use windows_sys::Win32::System::Threading::{
    OpenProcess, SuspendThread, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ, PROCESS_SUSPEND_RESUME,
};

use crate::models::LockedApp;

fn get_process_sha256(path: &str) -> Result<String, String> {
    let data = fs::read(path).map_err(|e| format!("Cannot read binary: {}", e))?;
    let mut hasher = Sha256::new();
    hasher.update(&data);
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn enumerate_processes() -> Vec<(String, String, String)> {
    let mut sys = System::new_all();
    sys.refresh_processes();

    let mut results = Vec::new();
    for (_pid, process) in sys.processes() {
        let name = process.name().to_string();
        let path = process.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let sha256 = get_process_sha256(&path).unwrap_or_default();
        results.push((name, path, sha256));
    }
    results
}

pub fn suspend_process_by_path(app: &LockedApp) -> Result<(), String> {
    let mut sys = System::new_all();
    sys.refresh_processes();

    for (pid, process) in sys.processes() {
        let process_path = process.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
        let process_name = process.name().to_string();

        let path_match = process_path.eq_ignore_ascii_case(&app.path);
        let name_match = process_name.eq_ignore_ascii_case(&app.name);

        if path_match || name_match {
            if !app.sha256.is_empty() {
                let current_hash = get_process_sha256(&process_path).unwrap_or_default();
                if current_hash != app.sha256 {
                    continue;
                }
            }

            unsafe {
                let handle = OpenProcess(
                    PROCESS_SUSPEND_RESUME | PROCESS_QUERY_INFORMATION | PROCESS_VM_READ,
                    0,
                    pid.as_u32(),
                );
                if handle != 0 {
                    let thread_id = windows_sys::Win32::System::Threading::GetProcessId(handle);
                    if thread_id != 0 {
                        let thandle = OpenProcess(
                            PROCESS_SUSPEND_RESUME,
                            0,
                            thread_id,
                        );
                        if thandle != 0 {
                            SuspendThread(thandle);
                            windows_sys::Win32::Foundation::CloseHandle(thandle);
                        }
                    }
                    windows_sys::Win32::Foundation::CloseHandle(handle);
                    return Ok(());
                }
            }
        }
    }
    Err("Process not found or cannot be suspended".to_string())
}

pub fn monitor_processes(locked_apps: Vec<LockedApp>) {
    std::thread::spawn(move || loop {
        let mut sys = System::new_all();
        sys.refresh_processes();

        for app in &locked_apps {
            if !app.enabled {
                continue;
            }

            for (_pid, process) in sys.processes() {
                let process_path = process.exe().map(|p| p.to_string_lossy().to_string()).unwrap_or_default();
                let process_name = process.name().to_string();

                if process_path.eq_ignore_ascii_case(&app.path)
                    || process_name.eq_ignore_ascii_case(&app.name)
                {
                    let _ = suspend_process_by_path(app);
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    });
}
