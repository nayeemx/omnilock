use aes_gcm::{aead::Aead, Aes256Gcm, Key, Nonce};
use aes_gcm::aead::KeyInit;
use argon2::{Argon2, Params, Version};
use serde::{Deserialize, Serialize};
use std::fs;

const HEADER_MAGIC: [u8; 4] = [0x4F, 0x4D, 0x4E, 0x49]; // "OMNI"

#[derive(Debug, Serialize, Deserialize)]
pub struct EncryptedVault {
    pub header: [u8; 4],
    #[serde(default)]
    pub version: u32,
    pub salt: Vec<u8>,
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
    pub tag: Vec<u8>,
}

fn hash_password(password: &str, salt: &[u8]) -> Result<Vec<u8>, String> {
    let params = Params::new(65536, 3, 1, Some(32)).map_err(|e| e.to_string())?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, Version::V0x13, params);
    let mut output = vec![0u8; 32];
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut output)
        .map_err(|e| e.to_string())?;
    Ok(output)
}

pub fn verify_vault_password(password: &str) -> bool {
    let vault_path = super::state::vault_path_programdata();
    if !vault_path.exists() {
        return false;
    }

    let data = match fs::read(&vault_path) {
        Ok(d) => d,
        Err(_) => return false,
    };

    let encrypted: EncryptedVault = match serde_json::from_slice(&data) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if encrypted.header != HEADER_MAGIC {
        return false;
    }

    if encrypted.salt.is_empty() || encrypted.nonce.is_empty() || encrypted.ciphertext.is_empty() {
        return false;
    }

    let key_material = match hash_password(password, &encrypted.salt) {
        Ok(k) => k,
        Err(_) => return false,
    };

    let key = Key::<Aes256Gcm>::from_slice(&key_material);
    let cipher = Aes256Gcm::new(key);
    let nonce = Nonce::from_slice(&encrypted.nonce);

    // AES-GCM includes the tag in the ciphertext for aes-gcm crate
    let mut ciphertext_with_tag = encrypted.ciphertext.clone();
    ciphertext_with_tag.extend_from_slice(&encrypted.tag);

    cipher.decrypt(nonce, ciphertext_with_tag.as_ref()).is_ok()
}
