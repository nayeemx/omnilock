use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

const KEY_FILENAME: &str = "omnilock.key";
const KEY_MAGIC: &str = "OMNILOCK-USB-KEY-V2";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbKeyFile {
    pub magic: String,
    pub version: u32,
    pub encrypted_key: String,
    pub nonce: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbDriveInfo {
    pub letter: String,
    pub label: String,
    pub serial: u32,
}

pub fn get_machine_fingerprint() -> String {
    unsafe {
        extern "system" {
            fn GetComputerNameA(name: *mut u8, size: *mut u32) -> i32;
            fn GetUserNameA(name: *mut u8, size: *mut u32) -> i32;
        }

        let mut comp_name = [0u8; 256];
        let mut comp_size: u32 = 256;
        GetComputerNameA(comp_name.as_mut_ptr(), &mut comp_size);

        let mut user_name = [0u8; 256];
        let mut user_size: u32 = 256;
        GetUserNameA(user_name.as_mut_ptr(), &mut user_size);

        let comp = String::from_utf8_lossy(&comp_name[..comp_size as usize]);
        let user = String::from_utf8_lossy(&user_name[..user_size as usize]);

        // Add volume serial of C: drive for additional uniqueness
        let vol_serial = get_volume_serial("C:\\");

        let material = format!("{}|{}|{}", comp, user, vol_serial);
        let mut hasher = Sha256::new();
        hasher.update(material.as_bytes());
        B64.encode(hasher.finalize())
    }
}

fn get_volume_serial(drive: &str) -> u32 {
    unsafe {
        extern "system" {
            fn GetVolumeInformationA(
                root: *const u8,
                vol: *mut u8,
                vol_size: u32,
                serial: *mut u32,
                max_comp: *mut u32,
                flags: *mut u32,
                fs: *mut u8,
                fs_size: u32,
            ) -> i32;
        }

        let root = format!("{}\0", drive);
        let mut serial: u32 = 0;
        let mut max_comp: u32 = 0;
        let mut flags: u32 = 0;
        let mut vol = [0u8; 256];
        let mut fs_name = [0u8; 256];

        GetVolumeInformationA(
            root.as_ptr(),
            vol.as_mut_ptr(),
            256,
            &mut serial,
            &mut max_comp,
            &mut flags,
            fs_name.as_mut_ptr(),
            256,
        );

        serial
    }
}

fn derive_key(material: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"OMNILOCK-USB-KEY-SALT\0");
    hasher.update(material);
    let result = hasher.finalize();
    let mut key = [0u8; 32];
    key.copy_from_slice(&result);
    key
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

pub fn write_key_to_drive(letter: &str, recovery_key: &str) -> Result<UsbDriveInfo, String> {
    let drives = list_removable_drives();
    let drive = drives.iter().find(|d| d.letter == letter)
        .ok_or_else(|| format!("Removable drive {} not found", letter))?;

    // Machine-bound encryption: key = SHA256(machine_fingerprint + usb_serial)
    let machine_fp = get_machine_fingerprint();
    let material = format!("{}|{}", machine_fp, drive.serial);
    let aes_key = derive_key(material.as_bytes());

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).expect("Failed to generate nonce");
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, recovery_key.as_bytes())
        .map_err(|e| e.to_string())?;

    let key_file = UsbKeyFile {
        magic: KEY_MAGIC.to_string(),
        version: 2,
        encrypted_key: B64.encode(&ciphertext),
        nonce: B64.encode(&nonce_bytes),
    };

    let json = serde_json::to_string_pretty(&key_file).map_err(|e| e.to_string())?;
    let key_path = format!("{}:\\{}", letter, KEY_FILENAME);
    fs::write(&key_path, json).map_err(|e| format!("Failed to write key to drive: {}", e))?;

    Ok(drive.clone())
}

pub fn detect_usb_key(expected_serial: Option<u32>) -> Option<(UsbDriveInfo, String)> {
    let drives = list_removable_drives();

    for drive in &drives {
        if let Some(expected) = expected_serial {
            if drive.serial != expected {
                continue;
            }
        }

        let key_path = PathBuf::from(format!("{}:\\{}", drive.letter, KEY_FILENAME));
        if !key_path.exists() {
            continue;
        }

        if let Ok(data) = fs::read_to_string(&key_path) {
            if let Ok(key_file) = serde_json::from_str::<UsbKeyFile>(&data) {
                if key_file.magic != KEY_MAGIC {
                    continue;
                }

                // Machine-bound decryption
                let machine_fp = get_machine_fingerprint();
                let material = format!("{}|{}", machine_fp, drive.serial);
                let aes_key = derive_key(material.as_bytes());

                let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key);
                let cipher = Aes256Gcm::new(key);

                let ciphertext = match B64.decode(&key_file.encrypted_key) {
                    Ok(c) => c,
                    Err(_) => continue,
                };
                let nonce_bytes = match B64.decode(&key_file.nonce) {
                    Ok(n) => n,
                    Err(_) => continue,
                };
                let nonce = Nonce::from_slice(&nonce_bytes);

                match cipher.decrypt(nonce, ciphertext.as_ref()) {
                    Ok(plaintext) => {
                        if let Ok(key_str) = String::from_utf8(plaintext) {
                            return Some((drive.clone(), key_str));
                        }
                    }
                    Err(_) => continue, // Wrong machine or corrupted
                }
            }
        }
    }

    None
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

    // Machine-bound decryption
    let drives = list_removable_drives();
    let drive = drives.iter().find(|d| d.letter == letter)
        .ok_or("Drive not found")?;

    let machine_fp = get_machine_fingerprint();
    let material = format!("{}|{}", machine_fp, drive.serial);
    let aes_key = derive_key(material.as_bytes());

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&aes_key);
    let cipher = Aes256Gcm::new(key);

    let ciphertext = B64.decode(&key_file.encrypted_key)
        .map_err(|_| "Invalid key file data")?;
    let nonce_bytes = B64.decode(&key_file.nonce)
        .map_err(|_| "Invalid key file data")?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = cipher.decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Key file is for a different machine or corrupted")?;

    String::from_utf8(plaintext).map_err(|_| "Invalid key data".to_string())
}
