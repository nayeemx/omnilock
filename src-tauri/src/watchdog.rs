use std::process::Command;
use sysinfo::System;

const MAIN_NAME: &str = "omnilock.exe";

pub fn start_watchdog() {
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

pub fn is_guard_process() -> bool {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.file_name().map(|n| n.to_string_lossy().eq_ignore_ascii_case("omnilock-guard.exe")))
        .unwrap_or(false)
}
