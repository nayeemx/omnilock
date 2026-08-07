// File and folder locking via AES-256-GCM encryption.
//
// When a file is locked it is encrypted in-place:
//   original.txt  →  original.txt.omnilock   (encrypted blob)
//   original.txt  is deleted
//
// Encrypted blob layout:
//   [4]  magic "OLCK"
//   [1]  version = 1
//   [12] AES-GCM nonce
//   [4]  original path length (u32 LE)
//   [N]  original path (UTF-8)
//   [*]  AES-256-GCM ciphertext + 16-byte tag
//
// Recovery functions (safe_recover_acl, force_unlock, scan_acl_damage,
// bulk_recover_acl) are kept to fix files that were damaged by the old ACL
// approach.

use aes_gcm::{aead::{Aead, KeyInit}, Aes256Gcm, Key, Nonce};
use std::fs;
use std::path::{Path, PathBuf};

use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::Security::Authorization::*;
use windows_sys::Win32::System::Threading::*;

// ── helpers ───────────────────────────────────────────────────────────────────

fn to_wide(s: &str) -> Vec<u16> {
    use std::ffi::OsStr;
    use std::os::windows::ffi::OsStrExt;
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

const MAGIC: &[u8; 4] = b"OLCK";

/// Returns the path of the encrypted blob for a given original path.
pub fn encrypted_path_for(path: &str) -> String {
    format!("{}.omnilock", path)
}

/// True when the file is locked (encrypted blob exists, original gone).
pub fn is_file_locked(path: &str) -> bool {
    let enc = encrypted_path_for(path);
    Path::new(&enc).exists() && !Path::new(path).exists()
}

// ── encryption helpers ────────────────────────────────────────────────────────

fn do_encrypt(data: &[u8], key_material: &[u8]) -> Result<(Vec<u8>, [u8; 12]), String> {
    if key_material.len() != 32 {
        return Err("Invalid file encryption key length (expected 32 bytes)".to_string());
    }
    let key = Key::<Aes256Gcm>::from_slice(key_material);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| format!("RNG error: {e}"))?;
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher.encrypt(nonce, data).map_err(|e| format!("Encrypt error: {e}"))?;
    Ok((ct, nonce_bytes))
}

fn do_decrypt(ciphertext: &[u8], key_material: &[u8], nonce_bytes: &[u8]) -> Result<Vec<u8>, String> {
    if key_material.len() != 32 {
        return Err("Invalid file encryption key length (expected 32 bytes)".to_string());
    }
    let key = Key::<Aes256Gcm>::from_slice(key_material);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce_bytes);
    cipher.decrypt(nonce, ciphertext)
        .map_err(|_| "Decryption failed — wrong key or corrupted file".to_string())
}

fn write_blob(encrypted_path: &str, nonce: &[u8; 12], original_path: &str, ciphertext: &[u8]) -> Result<(), String> {
    let path_bytes = original_path.as_bytes();
    let mut blob = Vec::with_capacity(4 + 1 + 12 + 4 + path_bytes.len() + ciphertext.len());
    blob.extend_from_slice(MAGIC);
    blob.push(1u8); // version
    blob.extend_from_slice(nonce);
    blob.extend_from_slice(&(path_bytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(path_bytes);
    blob.extend_from_slice(ciphertext);
    fs::write(encrypted_path, &blob).map_err(|e| format!("Cannot write encrypted file: {e}"))
}

fn read_blob(encrypted_path: &str) -> Result<([u8; 12], String, Vec<u8>), String> {
    let blob = fs::read(encrypted_path).map_err(|e| format!("Cannot read encrypted file: {e}"))?;
    let min_len = 4 + 1 + 12 + 4;
    if blob.len() < min_len {
        return Err("Not a valid OmniLock encrypted file (too short)".to_string());
    }
    if &blob[..4] != MAGIC {
        return Err("Not an OmniLock encrypted file (bad magic)".to_string());
    }
    // version at [4], ignored for now
    let nonce: [u8; 12] = blob[5..17].try_into().map_err(|_| "Bad nonce".to_string())?;
    let path_len = u32::from_le_bytes([blob[17], blob[18], blob[19], blob[20]]) as usize;
    let path_end = 21 + path_len;
    if blob.len() < path_end {
        return Err("Encrypted file header truncated".to_string());
    }
    let original_path = String::from_utf8(blob[21..path_end].to_vec())
        .map_err(|_| "Invalid original path in encrypted file".to_string())?;
    let ciphertext = blob[path_end..].to_vec();
    Ok((nonce, original_path, ciphertext))
}

// ── public lock / unlock API ──────────────────────────────────────────────────

/// Encrypt a single file in-place. Returns the path of the .omnilock file.
/// The original file is deleted only after the encrypted blob is written.
pub fn lock_file(path: &str, key_material: &[u8]) -> Result<String, String> {
    check_protected_path(path)?;
    if !Path::new(path).exists() {
        return Err(format!("File does not exist: {path}"));
    }
    if is_file_locked(path) {
        return Ok(encrypted_path_for(path)); // already locked
    }
    let data = fs::read(path).map_err(|e| format!("Cannot read file: {e}"))?;
    let (ciphertext, nonce) = do_encrypt(&data, key_material)?;
    let enc_path = encrypted_path_for(path);
    write_blob(&enc_path, &nonce, path, &ciphertext)?;
    fs::remove_file(path).map_err(|e| format!("Cannot remove original after encrypt: {e}"))?;
    crate::logger::log("LOCK", &format!("lock_file encrypted: {path} -> {enc_path}"));
    Ok(enc_path)
}

/// Decrypt a locked file back to its original path. Returns the original path.
/// Pass the ORIGINAL path (not the .omnilock path).
pub fn unlock_file(path: &str, key_material: &[u8]) -> Result<String, String> {
    let enc_path = encrypted_path_for(path);

    // Support both: caller passes original path or the .omnilock path
    let blob_path = if Path::new(path).extension().map(|e| e == "omnilock").unwrap_or(false) && Path::new(path).exists() {
        path.to_string()
    } else if Path::new(&enc_path).exists() {
        enc_path.clone()
    } else {
        return Err(format!("No encrypted file found for: {path}"));
    };

    let (nonce, original_path, ciphertext) = read_blob(&blob_path)?;
    let plaintext = do_decrypt(&ciphertext, key_material, &nonce)?;

    // Ensure parent directory exists
    if let Some(parent) = Path::new(&original_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create parent dir: {e}"))?;
    }

    // Never silently overwrite a file the user (re)created while a blob existed.
    if Path::new(&original_path).exists() {
        return Err(format!(
            "Refusing to overwrite existing file: {original_path}. \
             Move or delete it first, then unlock again."
        ));
    }
    fs::write(&original_path, plaintext).map_err(|e| format!("Cannot write decrypted file: {e}"))?;
    fs::remove_file(&blob_path).map_err(|e| format!("Cannot remove encrypted blob: {e}"))?;
    crate::logger::log("UNLOCK", &format!("unlock_file decrypted: {blob_path} -> {original_path}"));
    Ok(original_path)
}

/// Lock all files inside a folder recursively. The folder itself stays visible.
/// Returns the number of files encrypted.
pub fn lock_folder(path: &str, key_material: &[u8]) -> Result<usize, String> {
    check_protected_path(path)?;
    let dir = Path::new(path);
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("Folder does not exist: {path}"));
    }
    let count = encrypt_dir_recursive(dir, key_material)?;
    crate::logger::log("LOCK", &format!("lock_folder encrypted {count} files: {path}"));
    Ok(count)
}

/// Decrypt all .omnilock files inside a folder recursively.
/// Returns the number of files decrypted.
pub fn unlock_folder(path: &str, key_material: &[u8]) -> Result<usize, String> {
    let dir = Path::new(path);
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("Folder does not exist: {path}"));
    }
    let count = decrypt_dir_recursive(dir, key_material)?;
    crate::logger::log("UNLOCK", &format!("unlock_folder decrypted {count} files: {path}"));
    Ok(count)
}

/// True when the folder contains at least one .omnilock file (i.e. is locked).
pub fn is_folder_locked(path: &str) -> bool {
    Path::new(path).is_dir() && dir_has_omnilock(Path::new(path))
}

fn dir_has_omnilock(dir: &Path) -> bool {
    if let Ok(entries) = fs::read_dir(dir) {
        for e in entries.flatten() {
            let p = e.path();
            if p.extension().map(|x| x == "omnilock").unwrap_or(false) {
                return true;
            }
            if p.is_dir() && dir_has_omnilock(&p) {
                return true;
            }
        }
    }
    false
}

fn encrypt_dir_recursive(dir: &Path, key: &[u8]) -> Result<usize, String> {
    let mut count = 0;
    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {e}"))?;
    for entry in entries.flatten() {
        let p = entry.path();
        // Never follow symlinks/junctions — they could point OUTSIDE the locked tree.
        if p.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            continue;
        }
        let name = p.file_name().unwrap_or_default().to_string_lossy();
        // Skip already-encrypted blobs and OmniLock's own files
        if name.ends_with(".omnilock") || name == "desktop.ini" {
            continue;
        }
        if p.is_dir() {
            count += encrypt_dir_recursive(&p, key)?;
        } else if p.is_file() {
            match lock_file(&p.to_string_lossy(), key) {
                Ok(_) => count += 1,
                Err(e) => crate::logger::log("LOCK", &format!("encrypt_dir skip {}: {e}", p.display())),
            }
        }
    }
    Ok(count)
}

fn decrypt_dir_recursive(dir: &Path, key: &[u8]) -> Result<usize, String> {
    let mut count = 0;
    let entries = fs::read_dir(dir).map_err(|e| format!("Cannot read dir: {e}"))?;
    for entry in entries.flatten() {
        let p = entry.path();
        // Never follow symlinks/junctions — they could point OUTSIDE the locked tree.
        if p.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
            continue;
        }
        if p.is_dir() {
            count += decrypt_dir_recursive(&p, key)?;
        } else if p.extension().map(|x| x == "omnilock").unwrap_or(false) {
            let path_str = p.to_string_lossy().to_string();
            match unlock_file(&path_str, key) {
                Ok(_) => count += 1,
                Err(e) => crate::logger::log("UNLOCK", &format!("decrypt_dir skip {path_str}: {e}")),
            }
        }
    }
    Ok(count)
}

/// verify_lock: returns true if the path (or its encrypted counterpart) is locked.
pub fn verify_lock(path: &str) -> Result<bool, String> {
    // Encryption-based lock: .omnilock file exists, original gone
    if is_file_locked(path) {
        return Ok(true);
    }
    // Folder lock: contains .omnilock files
    if Path::new(path).is_dir() {
        return Ok(is_folder_locked(path));
    }
    // Caller passed the .omnilock path directly
    if path.ends_with(".omnilock") && Path::new(path).exists() {
        return Ok(true);
    }
    Ok(false)
}

// ── backup helpers (kept for restore functionality) ───────────────────────────

#[allow(dead_code)] // reserved: create_backup_before_lock was removed in the encryption rewrite
fn backup_path_for(path: &str) -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let safe_name = path.replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '_', "_");
    let now = std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let backup_dir = PathBuf::from(appdata)
        .join("InnologyBD")
        .join("OmniLock")
        .join("backups")
        .join(&safe_name);
    fs::create_dir_all(&backup_dir).map_err(|e| format!("Cannot create backup dir: {e}"))?;
    let name = Path::new(path).file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "unknown".to_string());
    Ok(backup_dir.join(format!("{}_{}.enc_backup", name, now)))
}

pub fn list_backups(path: &str) -> Result<Vec<(String, u64)>, String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let safe_name = path.replace(|c: char| !c.is_alphanumeric() && c != '.' && c != '_', "_");
    let backup_dir = PathBuf::from(appdata)
        .join("InnologyBD")
        .join("OmniLock")
        .join("backups")
        .join(&safe_name);
    if !backup_dir.exists() {
        return Ok(Vec::new());
    }
    let mut backups = Vec::new();
    if let Ok(entries) = fs::read_dir(&backup_dir) {
        for entry in entries.flatten() {
            let meta = entry.metadata().ok();
            let secs = meta.as_ref()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            backups.push((entry.path().to_string_lossy().to_string(), secs));
        }
    }
    backups.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(backups)
}

pub fn restore_backup(backup_path: &str, target_path: &str) -> Result<(), String> {
    if !Path::new(backup_path).exists() {
        return Err("Backup file does not exist".to_string());
    }
    if let Some(parent) = Path::new(target_path).parent() {
        fs::create_dir_all(parent).map_err(|e| format!("Cannot create parent: {e}"))?;
    }
    fs::copy(backup_path, target_path).map_err(|e| format!("Cannot restore: {e}"))?;
    Ok(())
}

// ── critical-path guard ───────────────────────────────────────────────────────

/// Refuse to lock paths that would be catastrophic (drive roots, the vault
/// directory — which contains the key that unlocks everything — and the app's
/// own executable).
fn check_protected_path(path: &str) -> Result<(), String> {
    let trimmed = path.trim_end_matches(['\\', '/']);
    // Drive root like "D:" or "D:"
    if trimmed.len() == 2 && trimmed.as_bytes()[1] == b':' {
        return Err(format!(
            "Refusing to encrypt a whole drive root ({path}). \
             Lock drives via drive locking (NoDrives) instead."
        ));
    }
    // Vault directory — locking it would encrypt the key itself
    if let Ok(appdata) = std::env::var("APPDATA") {
        let vault_dir = PathBuf::from(appdata).join("InnologyBD");
        if path.to_lowercase().starts_with(&vault_dir.to_string_lossy().to_lowercase()) {
            return Err(format!(
                "Refusing to lock the OmniLock vault directory ({path}) — \
                 this would make locked files unrecoverable."
            ));
        }
    }
    // The running executable
    if let Ok(exe) = std::env::current_exe() {
        let exe_lower = exe.to_string_lossy().to_lowercase();
        let path_lower = path.to_lowercase();
        if path_lower == exe_lower
            || path_lower == format!("{}.omnilock", exe_lower)
        {
            return Err("Refusing to lock the OmniLock executable itself.".to_string());
        }
    }
    Ok(())
}

// ── ACL recovery (fixes damage from old lock mechanism) ───────────────────────

unsafe fn enable_privilege(name: &str) -> Result<(), String> {
    let mut h_token: HANDLE = std::ptr::null_mut();
    if OpenProcessToken(GetCurrentProcess(), TOKEN_ADJUST_PRIVILEGES | TOKEN_QUERY, &mut h_token) == 0 {
        return Err("OpenProcessToken failed".to_string());
    }
    let name_w = to_wide(name);
    let mut luid: LUID = std::mem::zeroed();
    if LookupPrivilegeValueW(std::ptr::null_mut(), name_w.as_ptr(), &mut luid) == 0 {
        CloseHandle(h_token);
        return Err(format!("LookupPrivilegeValue({name}) failed"));
    }
    let mut tp = TOKEN_PRIVILEGES {
        PrivilegeCount: 1,
        Privileges: [LUID_AND_ATTRIBUTES { Luid: luid, Attributes: SE_PRIVILEGE_ENABLED }],
    };
    AdjustTokenPrivileges(h_token, 0, &mut tp, std::mem::size_of::<TOKEN_PRIVILEGES>() as u32,
        std::ptr::null_mut(), std::ptr::null_mut());
    CloseHandle(h_token);
    Ok(())
}

fn current_user_sid_buf() -> Result<Vec<u8>, String> {
    unsafe {
        let mut h_token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h_token) == 0 {
            return Err("OpenProcessToken failed".to_string());
        }
        let mut len: u32 = 0;
        GetTokenInformation(h_token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u8; len as usize];
        if GetTokenInformation(h_token, TokenUser, buf.as_mut_ptr() as _, len, &mut len) == 0 {
            CloseHandle(h_token);
            return Err("GetTokenInformation failed".to_string());
        }
        CloseHandle(h_token);
        Ok(buf)
    }
}

fn make_safe_dacl() -> Result<(*mut ACL, Vec<u8>), String> {
    unsafe {
        let buf = current_user_sid_buf()?;
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);

        let mut admin_sid: PSID = std::ptr::null_mut();
        ConvertStringSidToSidW(to_wide("S-1-5-32-544").as_ptr(), &mut admin_sid);
        let mut system_sid: PSID = std::ptr::null_mut();
        ConvertStringSidToSidW(to_wide("S-1-5-18").as_ptr(), &mut system_sid);

        let mut entries: Vec<EXPLICIT_ACCESS_W> = Vec::new();

        let mut ea_user: EXPLICIT_ACCESS_W = std::mem::zeroed();
        ea_user.grfAccessPermissions = GENERIC_ALL;
        ea_user.grfAccessMode = GRANT_ACCESS;
        ea_user.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
        ea_user.Trustee.TrusteeForm = TRUSTEE_IS_SID;
        ea_user.Trustee.TrusteeType = TRUSTEE_IS_USER;
        ea_user.Trustee.ptstrName = token_user.User.Sid as *mut u16;
        entries.push(ea_user);

        if !admin_sid.is_null() {
            let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
            ea.grfAccessPermissions = GENERIC_ALL;
            ea.grfAccessMode = GRANT_ACCESS;
            ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
            ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
            ea.Trustee.TrusteeType = TRUSTEE_IS_GROUP;
            ea.Trustee.ptstrName = admin_sid as *mut u16;
            entries.push(ea);
        }
        if !system_sid.is_null() {
            let mut ea: EXPLICIT_ACCESS_W = std::mem::zeroed();
            ea.grfAccessPermissions = GENERIC_ALL;
            ea.grfAccessMode = GRANT_ACCESS;
            ea.grfInheritance = SUB_CONTAINERS_AND_OBJECTS_INHERIT;
            ea.Trustee.TrusteeForm = TRUSTEE_IS_SID;
            ea.Trustee.TrusteeType = TRUSTEE_IS_WELL_KNOWN_GROUP;
            ea.Trustee.ptstrName = system_sid as *mut u16;
            entries.push(ea);
        }

        let mut new_dacl: *mut ACL = std::ptr::null_mut();
        let ret = SetEntriesInAclW(entries.len() as u32, entries.as_mut_ptr(), std::ptr::null_mut(), &mut new_dacl);

        if !admin_sid.is_null() { LocalFree(admin_sid as _); }
        if !system_sid.is_null() { LocalFree(system_sid as _); }

        if ret != 0 { return Err(format!("SetEntriesInAclW failed: {ret}")); }
        Ok((new_dacl, buf))
    }
}

pub fn safe_recover_acl(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    unsafe {
        let path_wide = to_wide(path);
        enable_privilege("SeTakeOwnershipPrivilege")?;
        let (new_dacl, buf) = make_safe_dacl()?;
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let ret = SetNamedSecurityInfoW(path_wide.as_ptr(), SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION | DACL_SECURITY_INFORMATION,
            token_user.User.Sid, std::ptr::null_mut(), new_dacl, std::ptr::null_mut());
        LocalFree(new_dacl as *mut _);
        if ret != 0 { return Err(format!("SetNamedSecurityInfo failed: err={ret}")); }
        Ok(())
    }
}

pub fn force_unlock(path: &str) -> Result<(), String> {
    if !Path::new(path).exists() {
        return Err(format!("Path does not exist: {path}"));
    }
    unsafe {
        let path_wide = to_wide(path);
        enable_privilege("SeTakeOwnershipPrivilege")?;
        enable_privilege("SeRestorePrivilege")?;

        // Take ownership as SYSTEM first
        let mut system_sid: PSID = std::ptr::null_mut();
        ConvertStringSidToSidW(to_wide("S-1-5-18").as_ptr(), &mut system_sid);
        SetNamedSecurityInfoW(path_wide.as_ptr(), SE_FILE_OBJECT, OWNER_SECURITY_INFORMATION,
            system_sid, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        if !system_sid.is_null() { LocalFree(system_sid as _); }

        // Reset DACL
        let (new_dacl, buf) = make_safe_dacl()?;
        SetNamedSecurityInfoW(path_wide.as_ptr(), SE_FILE_OBJECT, DACL_SECURITY_INFORMATION,
            std::ptr::null_mut(), std::ptr::null_mut(), new_dacl, std::ptr::null_mut());
        LocalFree(new_dacl as *mut _);

        // Restore owner to current user
        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let ret = SetNamedSecurityInfoW(path_wide.as_ptr(), SE_FILE_OBJECT, OWNER_SECURITY_INFORMATION,
            token_user.User.Sid, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut());
        if ret != 0 { return Err(format!("Restore ownership failed: err={ret}")); }
        Ok(())
    }
}

// ── ACL damage scanner (for fixing old-format locks) ─────────────────────────

pub fn scan_acl_damage(root: &str) -> Vec<String> {
    let mut damaged: Vec<String> = Vec::new();
    scan_recursive(Path::new(root), &mut damaged);
    damaged
}

fn is_owned_by_system(path: &str) -> bool {
    unsafe {
        let path_wide = to_wide(path);
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        let ret = GetNamedSecurityInfoW(path_wide.as_ptr(), SE_FILE_OBJECT,
            OWNER_SECURITY_INFORMATION, std::ptr::null_mut(), std::ptr::null_mut(),
            std::ptr::null_mut(), std::ptr::null_mut(), &mut sd);
        if ret != 0 { return false; }

        let mut owner_sid: PSID = std::ptr::null_mut();
        let mut defaulted: i32 = 0;
        GetSecurityDescriptorOwner(sd, &mut owner_sid, &mut defaulted);

        let mut result = false;
        if !owner_sid.is_null() {
            let mut system_sid: PSID = std::ptr::null_mut();
            if ConvertStringSidToSidW(to_wide("S-1-5-18").as_ptr(), &mut system_sid) != 0 {
                result = EqualSid(owner_sid, system_sid) != 0;
                LocalFree(system_sid as *mut _);
            }
        }
        LocalFree(sd as *mut _);
        result
    }
}

fn scan_recursive(dir: &Path, out: &mut Vec<String>) {
    let dir_str = dir.to_string_lossy().to_string();
    if is_owned_by_system(&dir_str) {
        out.push(dir_str.clone());
    }
    unsafe {
        use windows_sys::Win32::Storage::FileSystem::{FindFirstFileW, FindNextFileW, FindClose, WIN32_FIND_DATAW, FILE_ATTRIBUTE_DIRECTORY};
        let pattern = format!(r"{}\*", dir_str);
        let pattern_w = to_wide(&pattern);
        let mut find_data: WIN32_FIND_DATAW = std::mem::zeroed();
        let handle = FindFirstFileW(pattern_w.as_ptr(), &mut find_data);
        if handle == INVALID_HANDLE_VALUE { return; }
        loop {
            let name = String::from_utf16_lossy(
                &find_data.cFileName[..find_data.cFileName.iter().position(|&c| c == 0).unwrap_or(find_data.cFileName.len())]
            );
            if name != "." && name != ".." {
                let child = dir.join(&name);
                let child_str = child.to_string_lossy().to_string();
                if find_data.dwFileAttributes & FILE_ATTRIBUTE_DIRECTORY != 0 {
                    scan_recursive(&child, out);
                } else if is_owned_by_system(&child_str) {
                    out.push(child_str);
                }
            }
            let mut next: WIN32_FIND_DATAW = std::mem::zeroed();
            if FindNextFileW(handle, &mut next) == 0 { break; }
            find_data = next;
        }
        FindClose(handle);
    }
}

pub fn bulk_recover_acl(paths: &[String]) -> Vec<(String, String)> {
    paths.iter().map(|p| {
        match force_unlock(p) {
            Ok(()) => (p.clone(), "ok".to_string()),
            Err(e) => (p.clone(), e),
        }
    }).collect()
}
