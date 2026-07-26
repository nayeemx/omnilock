use hmac::{Hmac, Mac};
use sha1::Sha1;
use totp_rs::{Algorithm, Secret, TOTP};

type HmacSha1 = Hmac<Sha1>;

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

fn generate_code_for_step(secret: &[u8], time_step: u64) -> Result<String, String> {
    let time_bytes = time_step.to_be_bytes();
    let mut mac = HmacSha1::new_from_slice(secret)
        .map_err(|e| format!("HMAC init failed: {}", e))?;
    mac.update(&time_bytes);
    let result = mac.finalize().into_bytes();

    let offset = (result[19] & 0x0f) as usize;
    let code = ((result[offset] as u32 & 0x7f) << 24)
        | ((result[offset + 1] as u32) << 16)
        | ((result[offset + 2] as u32) << 8)
        | (result[offset + 3] as u32);

    Ok(format!("{:06}", code % 1_000_000))
}

pub fn generate_current_code(b32_secret: &str) -> Result<String, String> {
    let secret_bytes = secret_from_b32(b32_secret)?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();
    generate_code_for_step(&secret_bytes, now / 30)
}

pub fn verify_totp_code(b32_secret: &str, code: &str) -> Result<bool, String> {
    let secret_bytes = secret_from_b32(b32_secret)?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_secs();

    let time_step = now / 30;

    for offset in 0u64..=2 {
        if time_step >= offset {
            let expected = generate_code_for_step(&secret_bytes, time_step - offset)?;
            if expected == code {
                return Ok(true);
            }
        }
        let expected = generate_code_for_step(&secret_bytes, time_step + offset)?;
        if expected == code {
            return Ok(true);
        }
    }

    Ok(false)
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
