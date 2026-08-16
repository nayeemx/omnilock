// Vault storage — private file storage inside the encrypted vault.
//
// Files are encrypted at rest with the vault's file_encryption_key (the same
// key used for lock blobs) and stored under %APPDATA%\InnologyBD\OmniLock\storage\
// under random names, so nothing about the stored files is visible on disk.
//
// Stored blob layout (file "<id>.vaultfile"):
//   [4]  magic "OVLF"
//   [1]  version = 1
//   [12] AES-GCM nonce
//   [4]  original file name length (u32 LE)
//   [N]  original file name (UTF-8)
//   [8]  original file size (u64 LE)
//   [*]  AES-256-GCM ciphertext + 16-byte tag
//
// Manifest (%APPDATA%\InnologyBD\OmniLock\vault_files.enc) is itself AES-GCM
// encrypted with the same key so stored file names are never leaked in clear:
//   [4]  magic "OVMF"
//   [1]  version = 1
//   [12] AES-GCM nonce
//   [*]  ciphertext of JSON: [{id, name, size, added_at}]
//
// The original file is deleted only after its encrypted blob is on disk.
// Extracting refuses to overwrite an existing destination file.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::file_locker;

const BLOB_MAGIC: &[u8; 4] = b"OVLF";
const MANIFEST_MAGIC: &[u8; 4] = b"OVMF";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFileInfo {
    pub id: String,
    pub name: String,
    pub size: u64,
    pub added_at: u64,
}

fn storage_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(appdata).join("InnologyBD\\OmniLock\\storage");
    fs::create_dir_all(&dir).ok();
    dir
}

fn manifest_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(appdata).join("InnologyBD\\OmniLock\\vault_files.enc")
}

fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

fn write_manifest(files: &[VaultFileInfo], key: &[u8]) -> Result<(), String> {
    let json = serde_json::to_vec(files).map_err(|e| e.to_string())?;
    let (ciphertext, nonce) = file_locker::do_encrypt(&json, key)?;
    let mut blob = Vec::with_capacity(4 + 1 + 12 + ciphertext.len());
    blob.extend_from_slice(MANIFEST_MAGIC);
    blob.push(1u8);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&ciphertext);
    fs::write(manifest_path(), &blob).map_err(|e| format!("Cannot write manifest: {e}"))
}

fn read_manifest(key: &[u8]) -> Result<Vec<VaultFileInfo>, String> {
    let path = manifest_path();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let blob = fs::read(&path).map_err(|e| format!("Cannot read manifest: {e}"))?;
    let min_len = 4 + 1 + 12;
    if blob.len() < min_len || &blob[..4] != MANIFEST_MAGIC {
        return Ok(Vec::new());
    }
    let nonce: [u8; 12] = blob[5..17].try_into().map_err(|_| "Bad manifest nonce")?;
    let plaintext = file_locker::do_decrypt(&blob[17..], key, &nonce)
        .map_err(|_| "Cannot decrypt file manifest — wrong key or corrupted vault")?;
    serde_json::from_slice(&plaintext).map_err(|e| format!("Invalid manifest: {e}"))
}

fn write_blob(id: &str, name: &str, size: u64, data: &[u8], key: &[u8]) -> Result<(), String> {
    let (ciphertext, nonce) = file_locker::do_encrypt(data, key)?;
    let name_bytes = name.as_bytes();
    let mut blob = Vec::with_capacity(4 + 1 + 12 + 4 + name_bytes.len() + 8 + ciphertext.len());
    blob.extend_from_slice(BLOB_MAGIC);
    blob.push(1u8);
    blob.extend_from_slice(&nonce);
    blob.extend_from_slice(&(name_bytes.len() as u32).to_le_bytes());
    blob.extend_from_slice(name_bytes);
    blob.extend_from_slice(&size.to_le_bytes());
    blob.extend_from_slice(&ciphertext);
    let path = storage_dir().join(format!("{id}.vaultfile"));
    fs::write(&path, &blob).map_err(|e| format!("Cannot write stored file: {e}"))
}

fn read_blob(id: &str, key: &[u8]) -> Result<(String, u64, Vec<u8>), String> {
    let path = storage_dir().join(format!("{id}.vaultfile"));
    let blob = fs::read(&path).map_err(|e| format!("Stored file not found ({e})"))?;
    let min_len = 4 + 1 + 12 + 4 + 8;
    if blob.len() < min_len || &blob[..4] != BLOB_MAGIC {
        return Err("Not a valid OmniLock stored file (bad magic)".to_string());
    }
    let nonce: [u8; 12] = blob[5..17].try_into().map_err(|_| "Bad nonce".to_string())?;
    let name_len = u32::from_le_bytes([blob[17], blob[18], blob[19], blob[20]]) as usize;
    let name_start = 21;
    let name_end = name_start + name_len;
    let size_end = name_end + 8;
    if blob.len() < size_end {
        return Err("Stored file header truncated".to_string());
    }
    let name = String::from_utf8(blob[name_start..name_end].to_vec())
        .map_err(|_| "Invalid stored file name".to_string())?;
    let size = u64::from_le_bytes(blob[name_end..size_end].try_into().unwrap());
    let plaintext = file_locker::do_decrypt(&blob[size_end..], key, &nonce)?;
    Ok((name, size, plaintext))
}

// ── public API ────────────────────────────────────────────────────────────────

/// Encrypt a file into vault storage and delete the original.
/// Returns the stored file info. Refuses protected paths (vault dir, drive roots).
pub fn store_file(path: &str, key: &[u8]) -> Result<VaultFileInfo, String> {
    file_locker::check_protected_path(path)?;
    let p = Path::new(path);
    if !p.exists() || !p.is_file() {
        return Err(format!("File does not exist: {path}"));
    }
    if p.symlink_metadata().map(|m| m.file_type().is_symlink()).unwrap_or(false) {
        return Err("Refusing to store a symbolic link".to_string());
    }
    let name = p.file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or("Invalid file name")?;
    let data = fs::read(p).map_err(|e| format!("Cannot read file: {e}"))?;
    let size = data.len() as u64;

    let mut id_bytes = [0u8; 16];
    getrandom::getrandom(&mut id_bytes).map_err(|e| format!("RNG error: {e}"))?;
    let id = id_bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();

    write_blob(&id, &name, size, &data, key)?;
    // The blob is on disk — only now remove the original.
    fs::remove_file(p).map_err(|e| format!("Cannot remove original after encrypt: {e}"))?;

    let mut files = read_manifest(key)?;
    let info = VaultFileInfo {
        id,
        name: name.clone(),
        size,
        added_at: now_secs(),
    };
    files.push(info.clone());
    write_manifest(&files, key)?;
    crate::logger::log("VAULT_STORE", &format!("stored {name} ({size} bytes)"));
    Ok(info)
}

/// List the files currently stored in the vault.
pub fn list_files(key: &[u8]) -> Result<Vec<VaultFileInfo>, String> {
    let mut files = read_manifest(key)?;
    files.sort_by(|a, b| b.added_at.cmp(&a.added_at));
    Ok(files)
}

/// Decrypt a stored file to `dest_dir/<original name>`, then remove it from storage.
/// Refuses to overwrite an existing destination file.
pub fn extract_file(id: &str, dest_dir: &str, key: &[u8]) -> Result<String, String> {
    let (name, size, plaintext) = read_blob(id, key)?;
    if plaintext.len() as u64 != size {
        return Err(format!(
            "Stored file size mismatch (expected {size}, got {}). Corrupted storage?",
            plaintext.len()
        ));
    }
    let dest = PathBuf::from(dest_dir).join(&name);
    if dest.exists() {
        return Err(format!(
            "Refusing to overwrite existing file: {}",
            dest.to_string_lossy()
        ));
    }
    fs::create_dir_all(Path::new(dest_dir)).map_err(|e| format!("Cannot create folder: {e}"))?;
    fs::write(&dest, plaintext).map_err(|e| format!("Cannot write extracted file: {e}"))?;

    // Remove the blob and manifest entry only after the file is safely written.
    let blob_path = storage_dir().join(format!("{id}.vaultfile"));
    let _ = fs::remove_file(&blob_path);
    let mut files = read_manifest(key)?;
    files.retain(|f| f.id != id);
    write_manifest(&files, key)?;
    crate::logger::log("VAULT_STORE", &format!("extracted {name} -> {}", dest.display()));
    Ok(dest.to_string_lossy().to_string())
}

/// Delete a stored file (blob + manifest entry).
pub fn delete_file(id: &str, key: &[u8]) -> Result<(), String> {
    let blob_path = storage_dir().join(format!("{id}.vaultfile"));
    if blob_path.exists() {
        fs::remove_file(&blob_path).map_err(|e| format!("Cannot delete stored file: {e}"))?;
    }
    let mut files = read_manifest(key)?;
    let before = files.len();
    files.retain(|f| f.id != id);
    if files.len() == before {
        return Err("Stored file not found".to_string());
    }
    write_manifest(&files, key)?;
    crate::logger::log("VAULT_STORE", &format!("deleted {id}"));
    Ok(())
}