use totp_rs::{Algorithm, Secret, TOTP};

pub fn generate_totp_secret() -> String {
    let mut secret_bytes = [0u8; 20];
    getrandom::getrandom(&mut secret_bytes).expect("Failed to generate TOTP secret");
    base32::encode(base32::Alphabet::RFC4648 { padding: true }, &secret_bytes)
}

fn secret_from_b32(b32_secret: &str) -> Result<Vec<u8>, String> {
    Secret::Encoded(b32_secret.to_string())
        .to_bytes()
        .map_err(|e| format!("Invalid TOTP secret: {}", e))
}

pub fn create_totp(b32_secret: &str) -> Result<TOTP, String> {
    let secret_bytes = secret_from_b32(b32_secret)?;
    TOTP::new(
        Algorithm::SHA1,
        6,
        30,
        1,
        secret_bytes,
        Some("InnologyBD".to_string()),
        "OmniLock".to_string(),
    )
    .map_err(|e| e.to_string())
}

pub fn generate_current_code(b32_secret: &str) -> Result<String, String> {
    let totp = create_totp(b32_secret)?;
    totp.generate_current().map_err(|e| e.to_string())
}

pub fn verify_totp_code(b32_secret: &str, code: &str) -> Result<bool, String> {
    let totp = create_totp(b32_secret)?;
    Ok(totp.check_current(code).unwrap_or(false))
}

pub fn generate_qr_data_uri(b32_secret: &str) -> Result<String, String> {
    let totp = create_totp(b32_secret)?;
    let base64_str = totp.get_qr_base64().map_err(|e| format!("QR generation failed: {}", e))?;
    Ok(format!("data:image/png;base64,{}", base64_str))
}

pub fn get_totp_url(b32_secret: &str) -> Result<String, String> {
    let totp = create_totp(b32_secret)?;
    Ok(totp.get_url())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_roundtrip() {
        let secret = generate_totp_secret();
        let code = generate_current_code(&secret).unwrap();
        assert_eq!(code.len(), 6);
        assert!(verify_totp_code(&secret, &code).unwrap());
    }

    #[test]
    fn test_secret_is_base32() {
        let secret = generate_totp_secret();
        assert!(base32::decode(base32::Alphabet::RFC4648 { padding: true }, &secret).is_some());
    }
}
