use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const KEY_FILENAME: &str = "omnilock.key";
const KEY_MAGIC: &str = "OMNILOCK-USB-KEY";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbKeyFile {
    pub magic: String,
    pub version: u32,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDriveInfo {
    pub letter: String,
    pub label: String,
    pub serial: u32,
}

pub fn list_removable_drives() -> Vec<UsbDriveInfo> {
    unsafe {
        extern "system" {
            fn GetLogicalDrives() -> u32;
            fn GetDriveTypeA(root_path: *const u8) -> u32;
            fn GetVolumeInformationA(
                root_path: *const u8,
                volume_name: *mut u8,
                volume_name_size: u32,
                volume_serial: *mut u32,
                max_comp_len: *mut u32,
                fs_flags: *mut u32,
                fs_name: *mut u8,
                fs_name_size: u32,
            ) -> i32;
        }

        const DRIVE_REMOVABLE: u32 = 2;
        let drives = GetLogicalDrives();
        let mut result = Vec::new();

        for i in 0..26 {
            if (drives >> i) & 1 == 1 {
                let letter = (b'A' + i as u8) as char;
                let root = format!("{}:\\\0", letter);
                let drive_type = GetDriveTypeA(root.as_ptr());

                if drive_type == DRIVE_REMOVABLE {
                    let mut volume_name = [0u8; 256];
                    let mut serial: u32 = 0;
                    let mut max_comp: u32 = 0;
                    let mut fs_flags: u32 = 0;
                    let mut fs_name = [0u8; 256];

                    let ok = GetVolumeInformationA(
                        root.as_ptr(),
                        volume_name.as_mut_ptr(),
                        256,
                        &mut serial,
                        &mut max_comp,
                        &mut fs_flags,
                        fs_name.as_mut_ptr(),
                        256,
                    );

                    if ok != 0 {
                        let label = String::from_utf8_lossy(
                            &volume_name[..volume_name.iter().position(|&b| b == 0).unwrap_or(256)]
                        ).to_string();

                        result.push(UsbDriveInfo {
                            letter: letter.to_string(),
                            label,
                            serial,
                        });
                    }
                }
            }
        }

        result
    }
}

fn find_key_on_drive(letter: &str) -> Option<PathBuf> {
    let key_path = PathBuf::from(format!("{}:\\{}", letter, KEY_FILENAME));
    if key_path.exists() {
        Some(key_path)
    } else {
        None
    }
}

pub fn detect_usb_key(expected_serial: Option<u32>) -> Option<(UsbDriveInfo, String)> {
    let drives = list_removable_drives();

    for drive in &drives {
        if let Some(expected) = expected_serial {
            if drive.serial != expected {
                continue;
            }
        }

        if let Some(key_path) = find_key_on_drive(&drive.letter) {
            if let Ok(data) = fs::read_to_string(&key_path) {
                if let Ok(key_file) = serde_json::from_str::<UsbKeyFile>(&data) {
                    if key_file.magic == KEY_MAGIC && !key_file.key.is_empty() {
                        return Some((drive.clone(), key_file.key));
                    }
                }
            }
        }
    }

    None
}

pub fn write_key_to_drive(letter: &str, recovery_key: &str) -> Result<UsbDriveInfo, String> {
    let drives = list_removable_drives();
    let drive = drives.iter().find(|d| d.letter == letter)
        .ok_or_else(|| format!("Removable drive {} not found", letter))?;

    let key_file = UsbKeyFile {
        magic: KEY_MAGIC.to_string(),
        version: 1,
        key: recovery_key.to_string(),
    };

    let json = serde_json::to_string_pretty(&key_file).map_err(|e| e.to_string())?;
    let key_path = format!("{}:\\{}", letter, KEY_FILENAME);
    fs::write(&key_path, json).map_err(|e| format!("Failed to write key to drive: {}", e))?;

    Ok(drive.clone())
}

pub fn verify_key_on_drive(letter: &str) -> Result<String, String> {
    let key_path = PathBuf::from(format!("{}:\\{}", letter, KEY_FILENAME));
    if !key_path.exists() {
        return Err(format!("No OmniLock key found on drive {}:", letter));
    }

    let data = fs::read_to_string(&key_path).map_err(|e| e.to_string())?;
    let key_file: UsbKeyFile = serde_json::from_str(&data).map_err(|e| format!("Invalid key file: {}", e))?;

    if key_file.magic != KEY_MAGIC {
        return Err("Not an OmniLock key file".to_string());
    }

    Ok(key_file.key)
}
