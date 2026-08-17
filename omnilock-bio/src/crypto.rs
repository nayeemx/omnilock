use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};

type Aes256Cbc = cbc::Encryptor<aes::Aes256>;
type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

pub fn sha256(data: &[u8]) -> [u8; 32] {
    Sha256::digest(data).into()
}

pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("hmac key");
    mac.update(data);
    mac.finalize().into_bytes().into()
}

/// TLS 1.2 P_hash (HMAC-SHA256 based PRF), as used by python-validity's `prf`.
pub fn prf(secret: &[u8], seed: &[u8], length: usize) -> Vec<u8> {
    let mut n = length.div_ceil(0x20);
    let mut res = Vec::with_capacity(length);
    let mut a = hmac_sha256(secret, seed);
    while n > 0 {
        let mut buf = Vec::with_capacity(a.len() + seed.len());
        buf.extend_from_slice(&a);
        buf.extend_from_slice(seed);
        let block = hmac_sha256(secret, &buf);
        res.extend_from_slice(&block);
        a = hmac_sha256(secret, &a);
        n -= 1;
    }
    res.truncate(length);
    res
}

/// Custom padding used by the sensor TLS: l = 16 - (len % 16), append l bytes of (l-1).
pub fn pad(b: &[u8]) -> Vec<u8> {
    let l = 16 - (b.len() % 16);
    let mut out = Vec::with_capacity(b.len() + l);
    out.extend_from_slice(b);
    out.extend(std::iter::repeat(l as u8 - 1).take(l));
    out
}

pub fn unpad(b: &[u8]) -> Result<Vec<u8>, String> {
    if b.is_empty() {
        return Err("unpad: empty".into());
    }
    let pad_len = b[b.len() - 1] as usize + 1;
    if pad_len > b.len() {
        return Err("unpad: bad padding".into());
    }
    Ok(b[..b.len() - pad_len].to_vec())
}

pub fn aes256_cbc_encrypt(key: &[u8], iv: &[u8; 16], plain: &[u8]) -> Vec<u8> {
    let cipher = Aes256Cbc::new_from_slices(key, iv).expect("aes key");
    cipher.encrypt_padded_vec_mut::<cbc::cipher::block_padding::NoPadding>(plain)
}

pub fn aes256_cbc_decrypt(key: &[u8], iv: &[u8; 16], cipher: &[u8]) -> Result<Vec<u8>, String> {
    if cipher.len() % 16 != 0 {
        return Err("aes ciphertext not block aligned".into());
    }
    let dec = Aes256CbcDec::new_from_slices(key, iv).expect("aes key");
    Ok(dec
        .decrypt_padded_vec_mut::<cbc::cipher::block_padding::NoPadding>(cipher)
        .map_err(|e| format!("aes decrypt: {:?}", e))?)
}