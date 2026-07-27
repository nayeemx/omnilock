use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;

use crate::models::{EncryptedVault, VaultConfig};
use serde::{Deserialize, Serialize};

const HEADER_MAGIC: &[u8; 4] = b"OMNI";
const VAULT_DIR: &str = "InnologyBD\\OmniLock";
const VAULT_FILE: &str = "vault.enc";
const RECOVERY_FILE: &str = "vault.recovery";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultRecoveryData {
    pub security_question: String,
    pub security_answer_hash: Vec<u8>,
    pub encrypted_password: Vec<u8>,
    pub password_nonce: Vec<u8>,
    #[serde(default)]
    pub encrypted_password_with_key: Vec<u8>,
    #[serde(default)]
    pub key_nonce: Vec<u8>,
}

fn vault_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(appdata).join(VAULT_DIR);
    fs::create_dir_all(&dir).ok();
    dir.join(VAULT_FILE)
}

pub fn vault_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = PathBuf::from(appdata).join(VAULT_DIR);
    fs::create_dir_all(&dir).ok();
    dir
}

fn recovery_path() -> PathBuf {
    vault_dir().join(RECOVERY_FILE)
}

pub fn hash_password(password: &str, salt: &[u8]) -> Result<Vec<u8>, String> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
    let mut output = vec![0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|e| e.to_string())?;
    Ok(output)
}

pub fn generate_salt() -> Vec<u8> {
    let mut salt = vec![0u8; 16];
    getrandom::getrandom(&mut salt).expect("Failed to generate random salt");
    salt
}

pub fn generate_recovery_key() -> String {
    let mut key_bytes = vec![0u8; 32];
    getrandom::getrandom(&mut key_bytes).expect("Failed to generate recovery key");
    let key = B64.encode(&key_bytes);
    key
}

pub fn encrypt_vault(config: &VaultConfig, password: &str) -> Result<(), String> {
    let json = serde_json::to_vec(config).map_err(|e| e.to_string())?;

    let key_material = hash_password(password, &config.password_salt)?;

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_material);
    let cipher = Aes256Gcm::new(key);

    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).expect("Failed to generate nonce");
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, json.as_ref())
        .map_err(|e| e.to_string())?;

    let encrypted = EncryptedVault {
        header: *HEADER_MAGIC,
        version: 1,
        salt: config.password_salt.clone(),
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        tag: Vec::new(),
    };

    let serialized = serde_json::to_vec(&encrypted).map_err(|e| e.to_string())?;
    fs::write(vault_path(), serialized).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn decrypt_vault(password: &str) -> Result<VaultConfig, String> {
    let data = fs::read(vault_path()).map_err(|e| format!("Vault not found: {}", e))?;

    let encrypted: EncryptedVault =
        serde_json::from_slice(&data).map_err(|e| format!("Invalid vault format: {}", e))?;

    if encrypted.header != *HEADER_MAGIC {
        return Err("Invalid vault header".to_string());
    }

    if encrypted.salt.is_empty() {
        return Err("Vault missing salt — corrupted or incompatible format".to_string());
    }

    let key_material = hash_password(password, &encrypted.salt)?;

    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_material);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&encrypted.nonce);

    let plaintext = cipher
        .decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|_| "Decryption failed - wrong password or corrupted vault".to_string())?;

    let config: VaultConfig =
        serde_json::from_slice(&plaintext).map_err(|e| format!("Invalid config: {}", e))?;
    Ok(config)
}

pub fn vault_exists() -> bool {
    vault_path().exists()
}

pub fn save_vault_meta(totp_enabled: bool) -> Result<(), String> {
    save_vault_meta_full(totp_enabled, 1)
}

pub fn save_vault_meta_full(totp_enabled: bool, vault_version: u32) -> Result<(), String> {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let dir = std::path::PathBuf::from(appdata).join(VAULT_DIR);
    let meta_path = dir.join("vault.meta");
    let meta = serde_json::json!({ "totp_enabled": totp_enabled, "vault_version": vault_version });
    std::fs::write(meta_path, serde_json::to_string(&meta).unwrap()).map_err(|e| e.to_string())
}

pub fn load_vault_meta() -> bool {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let meta_path = std::path::PathBuf::from(appdata)
        .join(VAULT_DIR)
        .join("vault.meta");
    if let Ok(data) = std::fs::read_to_string(meta_path) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&data) {
            return meta["totp_enabled"].as_bool().unwrap_or(false);
        }
    }
    false
}

pub fn load_vault_version() -> u32 {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let meta_path = std::path::PathBuf::from(appdata)
        .join(VAULT_DIR)
        .join("vault.meta");
    if let Ok(data) = std::fs::read_to_string(meta_path) {
        if let Ok(meta) = serde_json::from_str::<serde_json::Value>(&data) {
            return meta["vault_version"].as_u64().unwrap_or(1) as u32;
        }
    }
    1
}

pub const CURRENT_VAULT_VERSION: u32 = 1;

pub fn migrate_vault_if_needed(password: &str) -> Result<u32, String> {
    let data = fs::read(vault_path()).map_err(|e| format!("Vault not found: {}", e))?;
    let encrypted: EncryptedVault =
        serde_json::from_slice(&data).map_err(|e| format!("Invalid vault format: {}", e))?;

    if encrypted.version >= CURRENT_VAULT_VERSION {
        return Ok(encrypted.version);
    }

    let mut config = decrypt_vault(password)?;
    let old_version = encrypted.version;

    if old_version < 1 {
        config.totp_enabled = false;
        config.totp_secret = String::new();
    }

    encrypt_vault(&config, password)?;
    save_vault_meta_full(config.totp_enabled, CURRENT_VAULT_VERSION)?;

    Ok(CURRENT_VAULT_VERSION)
}

pub fn hash_answer(answer: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(answer.to_lowercase().trim().as_bytes());
    hasher.finalize().to_vec()
}

pub fn save_vault_recovery(
    question: &str,
    answer_hash: &[u8],
    encrypted_pw: &[u8],
    nonce: &[u8],
    encrypted_pw_with_key: &[u8],
    key_nonce: &[u8],
) -> Result<(), String> {
    let data = VaultRecoveryData {
        security_question: question.to_string(),
        security_answer_hash: answer_hash.to_vec(),
        encrypted_password: encrypted_pw.to_vec(),
        password_nonce: nonce.to_vec(),
        encrypted_password_with_key: encrypted_pw_with_key.to_vec(),
        key_nonce: key_nonce.to_vec(),
    };
    let json = serde_json::to_vec(&data).map_err(|e| e.to_string())?;
    fs::write(recovery_path(), json).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn load_vault_recovery() -> Result<VaultRecoveryData, String> {
    let data = fs::read(recovery_path()).map_err(|e| format!("No recovery data found: {}", e))?;
    serde_json::from_slice(&data).map_err(|e| format!("Invalid recovery data: {}", e))
}

pub fn recovery_exists() -> bool {
    recovery_path().exists()
}

pub fn encrypt_bytes(data: &[u8], key_material: &[u8]) -> Result<(Vec<u8>, Vec<u8>), String> {
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key_material);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).expect("Failed to generate nonce");
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, data)
        .map_err(|e| e.to_string())?;
    Ok((ciphertext, nonce_bytes.to_vec()))
}

fn decrypt_bytes(data: &[u8], key_material: &[u8], nonce: &[u8]) -> Result<Vec<u8>, String> {
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(key_material);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(nonce);
    cipher
        .decrypt(nonce, data)
        .map_err(|_| "Decryption failed".to_string())
}

pub fn reset_password(new_password: &str, answer: &str) -> Result<(), String> {
    let recovery = load_vault_recovery()?;

    let answer_hash = hash_answer(answer);
    if answer_hash.as_slice() != recovery.security_answer_hash.as_slice() {
        return Err("Incorrect security answer".to_string());
    }

    let old_pw_bytes = decrypt_bytes(
        &recovery.encrypted_password,
        &answer_hash,
        &recovery.password_nonce,
    )
    .map_err(|_| "Failed to recover credentials. Recovery data may be corrupted.".to_string())?;
    let old_password = String::from_utf8(old_pw_bytes)
        .map_err(|_| "Invalid recovery data format".to_string())?;

    if new_password == old_password {
        return Err("New password must be different from your current password.".to_string());
    }

    rekey_vault(&old_password, new_password, &answer)?;

    Ok(())
}

pub fn reset_password_with_key(new_password: &str, recovery_key: &str) -> Result<(), String> {
    if new_password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    let has_upper = new_password.chars().any(|c| c.is_uppercase());
    let has_lower = new_password.chars().any(|c| c.is_lowercase());
    let has_digit = new_password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = new_password.chars().any(|c| !c.is_alphanumeric());

    if !has_upper || !has_lower || !has_digit || !has_symbol {
        return Err("Password must include uppercase, lowercase, numbers, and symbols".to_string());
    }

    let recovery = load_vault_recovery()?;

    let key_material = derive_recovery_key_material(recovery_key);

    if recovery.encrypted_password_with_key.is_empty() {
        return Err("This vault was created before recovery key support was available. Use security question recovery.".to_string());
    }

    let old_pw_bytes = decrypt_bytes(
        &recovery.encrypted_password_with_key,
        &key_material,
        &recovery.key_nonce,
    )
    .map_err(|_| "Invalid recovery key. Make sure you are using the correct key.".to_string())?;
    let old_password = String::from_utf8(old_pw_bytes)
        .map_err(|_| "Invalid recovery data format".to_string())?;

    if new_password == old_password {
        return Err("New password must be different from your current password.".to_string());
    }

    let mut config = decrypt_vault(&old_password)?;

    let salt = generate_salt();
    let password_hash = hash_password(new_password, &salt)?;
    config.password_hash = password_hash;
    config.password_salt = salt;
    config.totp_enabled = false;
    config.totp_secret = String::new();

    encrypt_vault(&config, new_password)?;

    let answer_hash = &recovery.security_answer_hash;
    let (new_encrypted_pw, new_nonce) =
        encrypt_bytes(new_password.as_bytes(), answer_hash)?;
    let (new_encrypted_pw_key, new_key_nonce) =
        encrypt_bytes(new_password.as_bytes(), &key_material)?;
    save_vault_recovery(
        &recovery.security_question,
        answer_hash,
        &new_encrypted_pw,
        &new_nonce,
        &new_encrypted_pw_key,
        &new_key_nonce,
    )?;
    save_vault_meta(false)?;

    Ok(())
}

fn rekey_vault(old_password: &str, new_password: &str, answer: &str) -> Result<(), String> {
    let mut config = decrypt_vault(old_password)?;

    let salt = generate_salt();
    let password_hash = hash_password(new_password, &salt)?;
    config.password_hash = password_hash;
    config.password_salt = salt;
    config.totp_enabled = false;
    config.totp_secret = String::new();

    encrypt_vault(&config, new_password)?;

    let recovery = load_vault_recovery()?;
    let new_answer_hash = hash_answer(answer);
    let (new_encrypted_pw, new_nonce) =
        encrypt_bytes(new_password.as_bytes(), &new_answer_hash)?;

    let (new_encrypted_pw_key, new_key_nonce) = if !recovery.encrypted_password_with_key.is_empty() {
        let key_material = derive_recovery_key_material(&config.recovery_key);
        encrypt_bytes(new_password.as_bytes(), &key_material)?
    } else {
        (Vec::new(), Vec::new())
    };

    save_vault_recovery(
        &recovery.security_question,
        &new_answer_hash,
        &new_encrypted_pw,
        &new_nonce,
        &new_encrypted_pw_key,
        &new_key_nonce,
    )?;
    save_vault_meta(false)?;

    Ok(())
}

pub fn derive_recovery_key_material(recovery_key: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(b"OMNILOCK-RECOVERY-KEY-DERIVE\0");
    hasher.update(recovery_key.as_bytes());
    hasher.finalize().to_vec()
}

pub fn encrypt_vault_to_bytes(config: &VaultConfig, password: &str) -> Result<Vec<u8>, String> {
    let json = serde_json::to_vec(config).map_err(|e| e.to_string())?;
    let key_material = hash_password(password, &config.password_salt)?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_material);
    let cipher = Aes256Gcm::new(key);
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut nonce_bytes).expect("Failed to generate nonce");
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, json.as_ref())
        .map_err(|e| e.to_string())?;
    let encrypted = EncryptedVault {
        header: *HEADER_MAGIC,
        version: 1,
        salt: config.password_salt.clone(),
        nonce: nonce_bytes.to_vec(),
        ciphertext,
        tag: Vec::new(),
    };
    serde_json::to_vec(&encrypted).map_err(|e| e.to_string())
}

pub fn decrypt_vault_from_bytes(data: &[u8], password: &str) -> Result<VaultConfig, String> {
    let encrypted: EncryptedVault =
        serde_json::from_slice(data).map_err(|e| format!("Invalid vault format: {}", e))?;
    if encrypted.header != *HEADER_MAGIC {
        return Err("Invalid vault header".to_string());
    }
    if encrypted.salt.is_empty() {
        return Err("Vault missing salt".to_string());
    }
    let key_material = hash_password(password, &encrypted.salt)?;
    let key = aes_gcm::Key::<Aes256Gcm>::from_slice(&key_material);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&encrypted.nonce);
    let plaintext = cipher
        .decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|_| "Decryption failed - wrong password".to_string())?;
    let config: VaultConfig =
        serde_json::from_slice(&plaintext).map_err(|e| format!("Invalid config: {}", e))?;
    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::SystemPresets;

    fn test_config() -> VaultConfig {
        VaultConfig {
            password_hash: vec![1, 2, 3],
            password_salt: vec![0xAA; 16],
            security_question: "What is your pet's name?".into(),
            security_answer_hash: vec![7, 8, 9],
            recovery_key: "RECOVERY-KEY-123".into(),
            locked_apps: vec![],
            system_presets: SystemPresets::default(),
            installer_guard_enabled: false,
            locked_files: vec![],
            locked_folders: vec![],
            locked_drives: vec![],
            auto_lock_minutes: 5,
            ..Default::default()
        }
    }

    #[test]
    fn test_hash_password_deterministic() {
        let salt = vec![0xAA; 16];
        let h1 = hash_password("mypassword", &salt).unwrap();
        let h2 = hash_password("mypassword", &salt).unwrap();
        assert_eq!(h1, h2, "same input must produce same hash");
        assert_eq!(h1.len(), 32);
    }

    #[test]
    fn test_hash_password_different_passwords() {
        let salt = vec![0xBB; 16];
        let h1 = hash_password("password1", &salt).unwrap();
        let h2 = hash_password("password2", &salt).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_password_different_salts() {
        let h1 = hash_password("same", &[0x01; 16]).unwrap();
        let h2 = hash_password("same", &[0x02; 16]).unwrap();
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_generate_salt_length() {
        let salt = generate_salt();
        assert_eq!(salt.len(), 16);
    }

    #[test]
    fn test_generate_salt_random() {
        let s1 = generate_salt();
        let s2 = generate_salt();
        assert_ne!(s1, s2);
    }

    #[test]
    fn test_encrypt_decrypt_bytes_roundtrip() {
        let mut key_material = vec![0u8; 32];
        getrandom::getrandom(&mut key_material).unwrap();

        let plaintext = b"hello omnilock";
        let (ciphertext, nonce) = encrypt_bytes(plaintext, &key_material).unwrap();
        assert_ne!(ciphertext, plaintext);

        let decrypted = decrypt_bytes(&ciphertext, &key_material, &nonce).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let mut key1 = vec![0u8; 32];
        let mut key2 = vec![0u8; 32];
        getrandom::getrandom(&mut key1).unwrap();
        getrandom::getrandom(&mut key2).unwrap();

        let (ciphertext, nonce) = encrypt_bytes(b"secret", &key1).unwrap();
        let result = decrypt_bytes(&ciphertext, &key2, &nonce);
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_wrong_nonce_fails() {
        let mut key = vec![0u8; 32];
        getrandom::getrandom(&mut key).unwrap();
        let wrong_nonce = vec![0xFF; 12];

        let (ciphertext, _nonce) = encrypt_bytes(b"secret", &key).unwrap();
        let result = decrypt_bytes(&ciphertext, &key, &wrong_nonce);
        assert!(result.is_err());
    }

    #[test]
    fn test_vault_encrypt_decrypt_roundtrip() {
        let config = test_config();
        let password = "TestPassword123!";

        let encrypted_bytes = encrypt_vault_to_bytes(&config, password).unwrap();
        let decrypted_config = decrypt_vault_from_bytes(&encrypted_bytes, password).unwrap();

        assert_eq!(decrypted_config.security_question, config.security_question);
        assert_eq!(decrypted_config.recovery_key, config.recovery_key);
        assert_eq!(decrypted_config.auto_lock_minutes, config.auto_lock_minutes);
    }

    #[test]
    fn test_vault_wrong_password_fails() {
        let config = test_config();
        let encrypted = encrypt_vault_to_bytes(&config, "correct").unwrap();
        let result = decrypt_vault_from_bytes(&encrypted, "wrong");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("wrong password"));
    }

    #[test]
    fn test_vault_header_magic() {
        let config = test_config();
        let encrypted = encrypt_vault_to_bytes(&config, "pw").unwrap();
        let parsed: EncryptedVault = serde_json::from_slice(&encrypted).unwrap();
        assert_eq!(&parsed.header, b"OMNI");
        assert_eq!(parsed.version, 1);
        assert!(!parsed.salt.is_empty());
    }

    #[test]
    fn test_vault_invalid_header() {
        let config = test_config();
        let mut parsed: EncryptedVault =
            serde_json::from_slice(&encrypt_vault_to_bytes(&config, "pw").unwrap()).unwrap();
        parsed.header = *b"XXXX";
        let tampered = serde_json::to_vec(&parsed).unwrap();
        let result = decrypt_vault_from_bytes(&tampered, "pw");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Invalid vault header"));
    }

    #[test]
    fn test_derive_recovery_key_material_deterministic() {
        let k1 = derive_recovery_key_material("key-123");
        let k2 = derive_recovery_key_material("key-123");
        assert_eq!(k1, k2);
        assert_eq!(k1.len(), 32);
    }

    #[test]
    fn test_derive_recovery_key_material_different_keys() {
        let k1 = derive_recovery_key_material("key-a");
        let k2 = derive_recovery_key_material("key-b");
        assert_ne!(k1, k2);
    }

    #[test]
    fn test_vault_empty_data_fails() {
        let result = decrypt_vault_from_bytes(b"", "pw");
        assert!(result.is_err());
    }
}
