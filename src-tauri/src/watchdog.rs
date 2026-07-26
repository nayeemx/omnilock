use std::process::Command;
use std::sync::OnceLock;
use std::time::Instant;
use sysinfo::System;

use crate::models::{WatchdogStatusDto, SystemInfoDto};

const MAIN_NAME: &str = "omnilock.exe";

static START_TIME: OnceLock<Instant> = OnceLock::new();

fn init_start_time() {
    START_TIME.get_or_init(|| Instant::now());
}

pub fn get_uptime_secs() -> u64 {
    START_TIME.get()
        .map(|t| t.elapsed().as_secs())
        .unwrap_or(0)
}

pub fn start_watchdog() {
    init_start_time();

    std::thread::spawn(|| loop {
        let mut sys = System::new_all();
        sys.refresh_processes();

        let main_running = sys.processes().values().any(|p| {
            p.name().eq_ignore_ascii_case(MAIN_NAME)
        });

        if !main_running {
            let _ = restart_main();
        }

        std::thread::sleep(std::time::Duration::from_millis(500));
    });
}

fn restart_main() -> Result<(), String> {
    let exe_path = std::env::current_exe()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("Cannot determine exe directory")?
        .join(MAIN_NAME);

    if exe_path.exists() {
        Command::new(&exe_path)
            .spawn()
            .map_err(|e| format!("Failed to restart {}: {}", MAIN_NAME, e))?;
        return Ok(());
    }

    Err(format!("Executable not found: {:?}", exe_path))
}

pub fn get_watchdog_status() -> WatchdogStatusDto {
    let pid = std::process::id();
    let uptime_secs = get_uptime_secs();

    let mut sys = System::new_all();
    sys.refresh_processes();
    let process_count = sys.processes().len();

    WatchdogStatusDto {
        pid,
        uptime_secs,
        process_count,
        status: "Running".to_string(),
    }
}

pub fn get_system_info() -> SystemInfoDto {
    let os = System::name().unwrap_or_else(|| "Windows".to_string());
    let os_version = System::long_os_version()
        .unwrap_or_else(|| System::os_version().unwrap_or_else(|| "Unknown".to_string()));
    let full_os = format!("{} {}", os, os_version).trim().to_string();
    let arch = std::env::consts::ARCH.to_string();

    SystemInfoDto {
        os: full_os,
        arch,
    }
}

pub fn is_guard_process() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("omnilock-guard.exe")))
        .unwrap_or(false)
}
