use crate::models::{AuthPayload, SetupPayload, SessionToken, VaultConfig};
use crate::{vault, totp as totp_mod};
use subtle::ConstantTimeEq;
use base64::{engine::general_purpose::STANDARD as B64, Engine};

use std::time::{SystemTime, UNIX_EPOCH};

pub fn setup_vault(payload: SetupPayload) -> Result<(), String> {
    if payload.master_password.len() < 8 {
        return Err("Password must be at least 8 characters".to_string());
    }

    let has_upper = payload.master_password.chars().any(|c| c.is_uppercase());
    let has_lower = payload.master_password.chars().any(|c| c.is_lowercase());
    let has_digit = payload.master_password.chars().any(|c| c.is_ascii_digit());
    let has_symbol = payload
        .master_password
        .chars()
        .any(|c| !c.is_alphanumeric());

    if !has_upper || !has_lower || !has_digit || !has_symbol {
        return Err("Password must include uppercase, lowercase, numbers, and symbols".to_string());
    }

    let salt = vault::generate_salt();
    let password_hash = vault::hash_password(&payload.master_password, &salt)?;
    let answer_hash = vault::hash_answer(&payload.security_answer);
    let recovery_key = vault::generate_recovery_key();
    let question = payload.security_question.clone();

    let totp_enabled = !payload.totp_secret.is_empty();

    let mut config = VaultConfig::default();
    config.password_hash = password_hash;
    config.password_salt = salt;
    config.security_question = question.clone();
    config.security_answer_hash = answer_hash.clone();
    config.recovery_key = recovery_key.clone();
    config.totp_enabled = totp_enabled;
    config.totp_secret = payload.totp_secret;

    vault::encrypt_vault(&config, &payload.master_password)?;
    vault::save_vault_meta(totp_enabled)?;

    let (encrypted_pw, pw_nonce) =
        vault::encrypt_bytes(payload.master_password.as_bytes(), &answer_hash)?;

    let key_material = vault::derive_recovery_key_material(&recovery_key);
    let (encrypted_pw_key, key_nonce) =
        vault::encrypt_bytes(payload.master_password.as_bytes(), &key_material)?;

    vault::save_vault_recovery(
        &question,
        &answer_hash,
        &encrypted_pw,
        &pw_nonce,
        &encrypted_pw_key,
        &key_nonce,
    )?;

    Ok(())
}

pub fn unlock_session(auth: AuthPayload) -> Result<SessionToken, String> {
    let config = vault::decrypt_vault(&auth.master_password)?;

    let computed_hash =
        vault::hash_password(&auth.master_password, &config.password_salt)?;

    let hash_eq: bool = computed_hash
        .as_slice()
        .ct_eq(config.password_hash.as_slice())
        .into();

    if !hash_eq {
        return Err("Invalid master password".to_string());
    }

    vault::migrate_vault_if_needed(&auth.master_password).ok();

    let config = vault::decrypt_vault(&auth.master_password)?;

    if config.totp_enabled {
        if auth.totp_code.is_empty() {
            return Err("Two-factor authentication code required".to_string());
        }
        let code_valid = totp_mod::verify_totp_code(&config.totp_secret, &auth.totp_code)?;
        if !code_valid {
            return Err("Invalid two-factor authentication code".to_string());
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let mut token_bytes = [0u8; 32];
    getrandom::getrandom(&mut token_bytes).expect("Failed to generate token");

    let token = SessionToken {
        token: B64.encode(&token_bytes),
        expires_at: now + 3600,
    };

    Ok(token)
}

pub fn verify_security_answer(answer: &str, stored_hash: &[u8]) -> bool {
    let computed = vault::hash_answer(answer);
    computed.as_slice().ct_eq(stored_hash).into()
}

pub fn is_session_expired(token: &SessionToken) -> bool {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    now > token.expires_at
}
