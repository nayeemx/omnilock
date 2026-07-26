use std::process::Command;
use crate::file_locker;

pub fn lock_drive(drive_letter: &str) -> Result<(), String> {
    let root = format!("{}:\\", drive_letter);
    file_locker::lock_file(&root)?;

    let reg_cmd = format!(
        r"reg add HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer /v NoDrives /t REG_DWORD /d {} /f",
        calculate_nodrives_value(drive_letter)
    );

    Command::new("cmd")
        .args(["/C", &reg_cmd])
        .output()
        .map_err(|e| e.to_string())?;

    Ok(())
}

pub fn unlock_drive(drive_letter: &str) -> Result<(), String> {
    let root = format!("{}:\\", drive_letter);
    file_locker::unlock_file(&root)?;

    let reg_cmd = r"reg delete HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Policies\Explorer /v NoDrives /f";

    Command::new("cmd")
        .args(["/C", reg_cmd])
        .output()
        .map_err(|e| e.to_string())?;

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
