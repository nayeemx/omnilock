use std::path::Path;

// ACL enforcement is handled by the Windows service via named pipe.
// These functions only handle the NoDrives registry visibility toggle.

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
