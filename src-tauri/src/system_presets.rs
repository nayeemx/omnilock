use sysinfo::System;
use std::process::Command;

use crate::models::SystemPresets;

pub fn kill_process_by_name(name: &str) -> Result<(), String> {
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

pub fn block_process_launch(name: &str) -> bool {
    let blocked_names = [
        "taskmgr.exe",
        "control.exe",
        "regedit.exe",
        "powershell.exe",
        "cmd.exe",
        "rstrui.exe",
    ];
    blocked_names.iter().any(|n| n.eq_ignore_ascii_case(name))
}

pub fn apply_system_presets(presets: &SystemPresets) -> Result<(), String> {
    if presets.task_manager {
        kill_process_by_name("taskmgr.exe")?;
        set_registry_lock("TaskMgr", true)?;
    } else {
        set_registry_lock("TaskMgr", false)?;
    }

    if presets.control_panel {
        set_registry_lock("ControlPanel", true)?;
    } else {
        set_registry_lock("ControlPanel", false)?;
    }

    if presets.registry_editor {
        kill_process_by_name("regedit.exe")?;
        set_registry_lock("Regedit", true)?;
    } else {
        set_registry_lock("Regedit", false)?;
    }

    if presets.powershell {
        kill_process_by_name("powershell.exe")?;
        set_registry_lock("PowerShell", true)?;
    } else {
        set_registry_lock("PowerShell", false)?;
    }

    if presets.cmd {
        kill_process_by_name("cmd.exe")?;
        set_registry_lock("CMD", true)?;
    } else {
        set_registry_lock("CMD", false)?;
    }

    if presets.system_restore {
        kill_process_by_name("rstrui.exe")?;
        set_registry_lock("SystemRestore", true)?;
    } else {
        set_registry_lock("SystemRestore", false)?;
    }

    Ok(())
}

fn set_registry_lock(target: &str, lock: bool) -> Result<(), String> {
    let (key, value_name, disable_value) = match target {
        "TaskMgr" => (
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Policies\System",
            "DisableTaskMgr",
            "1",
        ),
        "ControlPanel" => (
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Policies\Explorer",
            "NoControlPanel",
            "1",
        ),
        "Regedit" => (
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Policies\System",
            "DisableRegistryTools",
            "1",
        ),
        "PowerShell" => (
            r"HKCU\Software\Policies\Microsoft\Windows\PowerShell",
            "EnableScripts",
            "0",
        ),
        "CMD" => (
            r"HKCU\Software\Policies\Microsoft\Windows\System",
            "DisableCMD",
            "1",
        ),
        "SystemRestore" => (
            r"HKLM\Software\Policies\Microsoft\Windows NT\SystemRestore",
            "DisableSR",
            "1",
        ),
        _ => return Err(format!("Unknown target: {}", target)),
    };

    if lock {
        Command::new("reg")
            .args(["add", key, "/v", value_name, "/t", "REG_DWORD", "/d", disable_value, "/f"])
            .output()
            .map_err(|e| e.to_string())?;
    } else {
        Command::new("reg")
            .args(["delete", key, "/v", value_name, "/f"])
            .output()
            .map_err(|e| e.to_string())?;
    }

    Ok(())
}
