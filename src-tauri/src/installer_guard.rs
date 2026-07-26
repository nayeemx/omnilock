use sysinfo::System;

const BLOCKED_INSTALLERS: &[&str] = &[
    "msiexec.exe",
    "setup.exe",
    "install.exe",
    "installer.exe",
    "autorun.exe",
    "update.exe",
];

pub fn monitor_installer_guard(enabled: bool) {
    if !enabled {
        return;
    }

    std::thread::spawn(|| loop {
        let mut sys = System::new_all();
        sys.refresh_processes();

        for (pid, process) in sys.processes() {
            let name = process.name();
            if BLOCKED_INSTALLERS.iter().any(|n| n.eq_ignore_ascii_case(name)) {
                unsafe {
                    let handle = windows_sys::Win32::System::Threading::OpenProcess(
                        windows_sys::Win32::System::Threading::PROCESS_TERMINATE,
                        0,
                        pid.as_u32(),
                    );
                    if handle != 0 {
                        windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);
                        windows_sys::Win32::Foundation::CloseHandle(handle);
                    }
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(1000));
    });
}

pub fn is_installer_process(name: &str) -> bool {
    BLOCKED_INSTALLERS.iter().any(|n| n.eq_ignore_ascii_case(name))
}

pub fn block_installer(name: &str) -> Result<(), String> {
    let mut sys = System::new_all();
    sys.refresh_processes();

    for (pid, process) in sys.processes() {
        if process.name().eq_ignore_ascii_case(name) {
            unsafe {
                let handle = windows_sys::Win32::System::Threading::OpenProcess(
                    windows_sys::Win32::System::Threading::PROCESS_TERMINATE,
                    0,
                    pid.as_u32(),
                );
                if handle != 0 {
                    windows_sys::Win32::System::Threading::TerminateProcess(handle, 1);
                    windows_sys::Win32::Foundation::CloseHandle(handle);
                }
            }
        }
    }
    Ok(())
}
