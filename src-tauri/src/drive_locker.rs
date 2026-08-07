use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::logger;

// ACL enforcement is handled by the Windows service via named pipe.
// These functions handle the NoDrives registry visibility toggle + ACL on drive root.

pub fn lock_drive(drive_letter: &str) -> Result<(), String> {
    let root = format!("{}:\\", drive_letter);
    if !Path::new(&root).exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }

    let reg_cmd = format!(
        r"reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer /v NoDrives /t REG_DWORD /d {} /f",
        calculate_nodrives_value(drive_letter)
    );
    crate::hidden_cmd("cmd")
        .args(["/C", &reg_cmd])
        .output()
        .map_err(|e| e.to_string())?;

    if root != "C:\\" {
        // v0.0.35: NoDrives hides the drive from Explorer. Whole-drive recursive
        // encryption was REMOVED — it would encrypt installed apps and is blocked
        // by the drive-root guard in file_locker. True drive access blocking needs
        // a design decision (see AGENTS.md -> priority issues).
        logger::log("DRIVE", &format!("Drive hidden via NoDrives: {}", root));
    }

    Ok(())
}

pub fn unlock_drive(drive_letter: &str, remaining_locked: &[String]) -> Result<(), String> {
    let root = format!("{}:\\", drive_letter);
    if !Path::new(&root).exists() {
        return Err(format!("Drive {} does not exist", drive_letter));
    }

    if remaining_locked.is_empty() {
        let reg_cmd = r"reg delete HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer /v NoDrives /f";
        crate::hidden_cmd("cmd")
            .args(["/C", reg_cmd])
            .output()
            .map_err(|e| e.to_string())?;
    } else {
        let mut mask = 0u32;
        for drive in remaining_locked {
            mask |= calculate_nodrives_value(drive);
        }
        let reg_cmd = format!(
            r"reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer /v NoDrives /t REG_DWORD /d {} /f",
            mask
        );
        crate::hidden_cmd("cmd")
            .args(["/C", &reg_cmd])
            .output()
            .map_err(|e| e.to_string())?;
    }

    if root != "C:\\" {
        logger::log("DRIVE", &format!("Drive visibility restored: {}", root));
    }

    Ok(())
}

fn calculate_nodrives_value(drive_letter: &str) -> u32 {
    let letter = drive_letter.to_uppercase().chars().next().unwrap_or('C');
    let bit_position = letter as u32 - 'A' as u32;
    1u32 << bit_position
}

pub fn list_available_drives() -> Vec<String> {
    let mut drives = Vec::new();
    for letter in 'A'..='Z' {
        let path = format!("{}:\\", letter);
        if std::path::Path::new(&path).exists() {
            drives.push(letter.to_string());
        }
    }
    drives
}

pub fn list_removable_drives() -> Vec<String> {
    const DRIVE_REMOVABLE: u32 = 2;
    let mut drives = Vec::new();
    for letter in 'A'..='Z' {
        let path = format!("{}:\\", letter);
        if Path::new(&path).exists() {
            let wide: Vec<u16> = path.encode_utf16().chain(std::iter::once(0)).collect();
            unsafe {
                let drive_type = windows_sys::Win32::Storage::FileSystem::GetDriveTypeW(wide.as_ptr());
                if drive_type == DRIVE_REMOVABLE {
                    drives.push(letter.to_string());
                }
            }
        }
    }
    drives
}

pub fn is_drive_present(drive_letter: &str) -> bool {
    let path = format!("{}:\\", drive_letter);
    Path::new(&path).exists()
}

static USB_MONITOR_RUNNING: AtomicBool = AtomicBool::new(false);

fn monitored_locked_drives() -> &'static Mutex<Vec<String>> {
    static STORE: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(Vec::new()))
}

type RelockCallback = Arc<dyn Fn() + Send + Sync>;

fn usb_removed_callback() -> &'static Mutex<Option<RelockCallback>> {
    static STORE: OnceLock<Mutex<Option<RelockCallback>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(None))
}

pub fn start_usb_removal_monitor() {
    if USB_MONITOR_RUNNING.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        let mut known_drives: Vec<(String, bool)> = Vec::new();

        loop {
            if !USB_MONITOR_RUNNING.load(Ordering::SeqCst) {
                break;
            }

            let removable = list_removable_drives();

            for drive in &removable {
                let present = is_drive_present(drive);
                let known = known_drives.iter().any(|(d, _)| d == drive);

                if !known {
                    known_drives.push((drive.clone(), present));
                } else if let Some(entry) = known_drives.iter_mut().find(|(d, _)| d == drive) {
                    if entry.1 && !present {
                        logger::log("USB", &format!("USB drive removed: {}", drive));

                        let locked_drives = monitored_locked_drives()
                            .lock()
                            .map(|g| g.clone())
                            .unwrap_or_default();

                        if locked_drives.contains(drive) {
                            logger::log("USB", &format!("Locked drive {} was removed!", drive));
                            if let Ok(guard) = usb_removed_callback().lock() {
                                if let Some(ref cb) = *guard {
                                    cb();
                                }
                            }
                        }
                    }
                    entry.1 = present;
                }
            }

            known_drives.retain(|(d, _)| removable.contains(d));

            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        USB_MONITOR_RUNNING.store(false, Ordering::SeqCst);
    });
}

pub fn stop_usb_removal_monitor() {
    USB_MONITOR_RUNNING.store(false, Ordering::SeqCst);
}

pub fn set_usb_removal_callback(cb: RelockCallback) {
    if let Ok(mut guard) = usb_removed_callback().lock() {
        *guard = Some(cb);
    }
}

pub fn set_monitored_locked_drives(drives: Vec<String>) {
    if let Ok(mut guard) = monitored_locked_drives().lock() {
        *guard = drives;
    }
}
