use p256::ecdh::{EphemeralSecret, SharedSecret};
use p256::ecdsa::signature::hazmat::{PrehashSigner, PrehashVerifier};
use p256::ecdsa::{Signature as EcdsaSignature, SigningKey as EcdsaSigningKey, VerifyingKey};
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::elliptic_curve::PrimeField;
use p256::{NonZeroScalar, PublicKey as P256PublicKey};
use rand::rngs::OsRng;
use sha2::{Digest, Sha256};

use crate::crypto::{aes256_cbc_decrypt, aes256_cbc_encrypt, hmac_sha256, prf, unpad};
use crate::error::{Error, Result};
use crate::usb::Device;

pub const PASSWORD_HARDCODED: &[u8] = include_bytes!("../resources/password_hardcoded.bin");
pub const GWK_SIGN_HARDCODED: &[u8] = include_bytes!("../resources/gwk_sign_hardcoded.bin");
pub const CRT_HARDCODED: &[u8] = include_bytes!("../resources/crt_hardcoded.bin");

/// Firmware public key used to verify the ECDH blob (hardcoded per fw revision
/// in synaWudfBioUsb.dll; only a genuine Synaptics device knows the private key).
const FWPUB_X: [u8; 32] = [
    0xf7, 0x27, 0x65, 0x3b, 0x4e, 0x16, 0xce, 0x06, 0x65, 0xa6, 0x89, 0x4d, 0x7f, 0x3a, 0x30,
    0xd7, 0xd0, 0xa0, 0xbe, 0x31, 0x0d, 0x12, 0x92, 0xa7, 0x43, 0x67, 0x1f, 0xdf, 0x69, 0xf6,
    0xa8, 0xd3,
];
const FWPUB_Y: [u8; 32] = [
    0xa8, 0x55, 0x38, 0xf8, 0xb6, 0xbe, 0xc5, 0x0d, 0x6e, 0xef, 0x8b, 0xd5, 0xf4, 0xd0, 0x7a,
    0x88, 0x62, 0x43, 0xc5, 0x8b, 0x23, 0x93, 0x94, 0x8d, 0xf7, 0x61, 0xa8, 0x47, 0x21, 0xa6,
    0xca, 0x94,
];

pub fn hs_key() -> NonZeroScalar {
    // key = password[:16], seed = password[16:] + aa aa
    let mut seed = Vec::with_capacity(0x12);
    seed.extend_from_slice(&PASSWORD_HARDCODED[0x10..]);
    seed.extend_from_slice(&[0xaa, 0xaa]);
    let key = &PASSWORD_HARDCODED[..0x10];
    let hk = prf(key, b"HS_KEY_PAIR_GEN", 0x20);
    let hk: Vec<u8> = hk.into_iter().rev().collect(); // python: int(hs_key[::-1].hex(), 16)
    let hk: [u8; 32] = hk.try_into().expect("hs_key length");
    let scalar = p256::Scalar::from_repr(hk.into()).unwrap();
    NonZeroScalar::new(scalar).expect("hs_key is zero")
}

pub fn rev32(b: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    for i in 0..32 {
        out[i] = b[31 - i];
    }
    out
}

fn point_to_wire(p: &P256PublicKey) -> ([u8; 32], [u8; 32]) {
    let aff = p.as_affine();
    let enc = aff.to_encoded_point(false);
    let (x, y) = match enc.coordinates() {
        p256::elliptic_curve::sec1::Coordinates::Uncompressed { x, y } => (x, y),
        _ => unreachable!("sec1 point has no coordinates"),
    };
    (rev32(x.as_slice()), rev32(y.as_slice()))
}

pub struct Tls {
    pub usb: Device,
    pub secure_rx: bool,
    pub secure_tx: bool,

    pub psk_encryption_key: Vec<u8>,
    pub psk_validation_key: Vec<u8>,

    client_random: Vec<u8>,
    server_random: Vec<u8>,
    ecdh_q: P256PublicKey,
    handshake_hash: Sha256,

    sign_key: Vec<u8>,
    validation_key: Vec<u8>,
    encryption_key: Vec<u8>,
    decryption_key: Vec<u8>,
    master_secret: Vec<u8>,

    priv_key: EcdsaSigningKey,
    session_public: P256PublicKey,

    pub tls_cert: Vec<u8>,
    pub ecdh_blob: Vec<u8>,
    pub priv_blob: Vec<u8>,
    pub paired: bool,
}

fn dummy_pubkey() -> P256PublicKey {
    // point (0,0) does not lie on the curve; used only as a placeholder
    // before handle_ecdh() is called.
    P256PublicKey::from_sec1_bytes(&[0x02, 0x00, 0x00]).expect("placeholder")
}

impl Tls {
    pub fn new(usb: Device, hw_key: &[u8]) -> Tls {
        let mut seed = Vec::with_capacity(3 + hw_key.len());
        seed.extend_from_slice(b"GWK");
        seed.extend_from_slice(hw_key);
        let psk_encryption_key = prf(PASSWORD_HARDCODED, &seed, 0x20);

        let mut seed2 = Vec::with_capacity(8 + GWK_SIGN_HARDCODED.len());
        seed2.extend_from_slice(b"GWK_SIGN");
        seed2.extend_from_slice(GWK_SIGN_HARDCODED);
        let psk_validation_key = prf(&psk_encryption_key, &seed2, 0x20);

        Tls {
            usb,
            secure_rx: false,
            secure_tx: false,
            psk_encryption_key,
            psk_validation_key,
            client_random: Vec::new(),
            server_random: Vec::new(),
            ecdh_q: dummy_pubkey(),
            handshake_hash: Sha256::new(),
            sign_key: Vec::new(),
            validation_key: Vec::new(),
            encryption_key: Vec::new(),
            decryption_key: Vec::new(),
            master_secret: Vec::new(),
            priv_key: EcdsaSigningKey::from_bytes((&[1u8; 32]).into()).unwrap(),
            session_public: dummy_pubkey(),
            tls_cert: Vec::new(),
            ecdh_blob: Vec::new(),
            priv_blob: Vec::new(),
            paired: false,
        }
    }

    pub fn cmd(&mut self, cmd: &[u8]) -> Result<Vec<u8>> {
        if self.secure_rx && self.secure_tx {
            self.app(cmd)
        } else {
            self.usb.cmd(cmd)
        }
    }

    pub fn open(&mut self) -> Result<()> {
        self.secure_rx = false;
        self.secure_tx = false;
        self.handshake_hash = Sha256::new();

        // Flight 1: 44 00 00 00 + handshake(ClientHello)
        let hello = self.make_client_hello();
        let mut flight1 = vec![0x44, 0x00, 0x00, 0x00];
        flight1.extend_from_slice(&self.make_handshake(&hello));
        let rsp = self.usb.cmd(&flight1)?;
        self.parse_tls_response(&rsp)?;

        self.make_keys();

        // Flight 2: 44 00 00 00 + handshake(certs + client_kex + cert_verify) + CCS + handshake(finish)
        let mut flight2 = vec![0x44, 0x00, 0x00, 0x00];
        let mut hs = Vec::new();
        hs.extend_from_slice(&self.make_certs());
        hs.extend_from_slice(&self.make_client_kex());
        hs.extend_from_slice(&self.make_cert_verify()?);
        flight2.extend_from_slice(&self.make_handshake(&hs));
        flight2.extend_from_slice(&self.make_change_cipher_spec());
        let finish = self.make_finish();
        flight2.extend_from_slice(&self.make_handshake(&finish));
        let rsp = self.usb.cmd(&flight2)?;
        self.parse_tls_response(&rsp)?;
        Ok(())
    }

    pub fn app(&mut self, b: &[u8]) -> Result<Vec<u8>> {
        let rec = self.make_app_data(b);
        let rsp = self.usb.cmd(&rec)?;
        self.parse_tls_response(&rsp)
    }

    // ---------------- handshake building ----------------

    fn update_neg(&mut self, b: &[u8]) {
        self.handshake_hash.update(b);
    }

    fn with_neg_hdr(&mut self, t: u8, b: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + b.len());
        out.push(t);
        out.push((b.len() >> 16) as u8);
        out.push((b.len() >> 8) as u8);
        out.push(b.len() as u8);
        out.extend_from_slice(b);
        self.update_neg(&out);
        out
    }

    fn make_client_hello(&mut self) -> Vec<u8> {
        let mut h = Vec::new();
        h.extend_from_slice(&[0x03, 0x03]); // TLS 1.2
        self.client_random = {
            let mut r = [0u8; 32];
            use rand::RngCore;
            OsRng.fill_bytes(&mut r);
            r.to_vec()
        };
        h.extend_from_slice(&self.client_random);
        // session id: len 7 + 7 zero bytes
        h.extend_from_slice(&[0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
        // cipher suites: c005, 003d, 008d
        h.extend_from_slice(&[0x00, 0x06, 0xc0, 0x05, 0x00, 0x3d, 0x00, 0x8d]);
        h.push(0x00); // no compression
        // extensions: truncated_hmac(0x0004)=0x0017, EC points(0x000b)=uncompressed
        let mut exts = Vec::new();
        exts.extend_from_slice(&[0x00, 0x04, 0x00, 0x02, 0x00, 0x17]);
        exts.extend_from_slice(&[0x00, 0x0b, 0x00, 0x02, 0x01, 0x00]);
        // python quirk: len(exts) - 2 in the length field
        h.extend_from_slice(&((exts.len() - 2) as u16).to_be_bytes());
        h.extend_from_slice(&exts);
        self.with_neg_hdr(0x01, &h)
    }

    fn make_certs(&mut self) -> Vec<u8> {
        let cert_len = self.tls_cert.len() as u16;
        let mut cert = Vec::new();
        cert.extend_from_slice(&[0xac, 0x16]);
        cert.extend_from_slice(&self.tls_cert);
        let mut wrap = Vec::new();
        wrap.extend_from_slice(&[0x00, 0x00]);
        wrap.extend_from_slice(&cert_len.to_be_bytes());
        wrap.extend_from_slice(&cert);
        let mut wrap2 = Vec::new();
        wrap2.extend_from_slice(&[0x00, 0x00]);
        wrap2.extend_from_slice(&cert_len.to_be_bytes());
        wrap2.extend_from_slice(&wrap);
        self.with_neg_hdr(0x0b, &wrap2)
    }

    fn make_client_kex(&mut self) -> Vec<u8> {
        let (x, y) = point_to_wire(&self.session_public);
        let mut b = Vec::with_capacity(65);
        b.push(0x04);
        b.extend_from_slice(&x);
        b.extend_from_slice(&y);
        self.with_neg_hdr(0x10, &b)
    }

    fn make_cert_verify(&mut self) -> Result<Vec<u8>> {
        let digest = self.handshake_hash.clone().finalize();
        let sig: EcdsaSignature = self
            .priv_key
            .sign_prehash(&digest)
            .map_err(|e| Error::Crypto(format!("cert verify signing failed: {:?}", e)))?;
        let der = sig.to_der().as_bytes().to_vec();
        Ok(self.with_neg_hdr(0x0f, &der))
    }

    fn make_finish(&mut self) -> Vec<u8> {
        self.secure_tx = true;
        let hs_hash = self.handshake_hash.clone().finalize();
        let mut seed = Vec::with_capacity(16 + 32);
        seed.extend_from_slice(b"client finished");
        seed.extend_from_slice(&hs_hash);
        let verify_data = prf(&self.master_secret, &seed, 0xc);
        let mut out = Vec::with_capacity(4 + verify_data.len());
        out.push(0x14);
        out.push(0x00);
        out.push(0x00);
        out.push(verify_data.len() as u8);
        out.extend_from_slice(&verify_data);
        out
    }

    fn make_change_cipher_spec(&self) -> Vec<u8> {
        vec![0x14, 0x03, 0x03, 0x00, 0x01, 0x01]
    }

    fn make_handshake(&mut self, b: &[u8]) -> Vec<u8> {
        let payload = if self.secure_tx {
            self.encrypt(&self.sign(0x16, b))
        } else {
            b.to_vec()
        };
        let mut rec = vec![0x16, 0x03, 0x03];
        rec.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        rec.extend_from_slice(&payload);
        rec
    }

    fn make_app_data(&mut self, b: &[u8]) -> Vec<u8> {
        let payload = self.encrypt(&self.sign(0x17, b));
        let mut rec = vec![0x17, 0x03, 0x03];
        rec.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        rec.extend_from_slice(&payload);
        rec
    }

    // ---------------- key material ----------------

    fn make_keys(&mut self) -> Result<()> {
        // Session ECDH keypair
        let sk = EphemeralSecret::random(&mut OsRng);
        self.session_public = P256PublicKey::from(sk.public_key());
        let pre_master_secret: SharedSecret = sk.diffie_hellman(&self.ecdh_q);

        let mut seed = Vec::with_capacity(32 + 32);
        seed.extend_from_slice(&self.client_random);
        seed.extend_from_slice(&self.server_random);

        let mut s = Vec::with_capacity(13 + 64);
        s.extend_from_slice(b"master secret");
        s.extend_from_slice(&seed);
        self.master_secret = prf(pre_master_secret.raw_secret_bytes(), &s, 0x30);

        let mut s = Vec::with_capacity(14 + 64);
        s.extend_from_slice(b"key expansion");
        s.extend_from_slice(&seed);
        let key_block = prf(&self.master_secret, &s, 0x120);
        self.sign_key = key_block[0x00..0x20].to_vec();
        self.validation_key = key_block[0x20..0x40].to_vec();
        self.encryption_key = key_block[0x40..0x60].to_vec();
        self.decryption_key = key_block[0x60..0x80].to_vec();
        Ok(())
    }

    // ---------------- record protection ----------------

    fn encrypt(&self, b: &[u8]) -> Vec<u8> {
        let mut iv = [0u8; 16];
        use rand::RngCore;
        OsRng.fill_bytes(&mut iv);
        let mut iv_and_ct = iv.to_vec();
        iv_and_ct.extend_from_slice(&aes256_cbc_encrypt(
            &self.encryption_key,
            &iv,
            &crate::crypto::pad(b),
        ));
        iv_and_ct
    }

    fn decrypt(&self, c: &[u8]) -> Result<Vec<u8>> {
        if c.len() < 16 {
            return Err(Error::Crypto("decrypt: too short".into()));
        }
        let iv: [u8; 16] = c[..16].try_into().unwrap();
        let m = aes256_cbc_decrypt(&self.decryption_key, &iv, &c[16..])
            .map_err(Error::Crypto)?;
        unpad(&m).map_err(Error::Crypto)
    }

    fn sign(&self, t: u8, b: &[u8]) -> Vec<u8> {
        let mut hdr = Vec::with_capacity(5);
        hdr.push(t);
        hdr.push(3);
        hdr.push(3);
        hdr.extend_from_slice(&(b.len() as u16).to_be_bytes());
        let sig = hmac_sha256(&self.sign_key, &hdr);
        let mut out = Vec::with_capacity(b.len() + 32);
        out.extend_from_slice(b);
        out.extend_from_slice(&sig);
        out
    }

    fn validate(&self, t: u8, b: &[u8]) -> Result<Vec<u8>> {
        if b.len() < 32 {
            return Err(Error::Crypto("validate: too short".into()));
        }
        let (body, hs) = b.split_at(b.len() - 32);
        let mut hdr = Vec::with_capacity(5);
        hdr.push(t);
        hdr.push(3);
        hdr.push(3);
        hdr.extend_from_slice(&(body.len() as u16).to_be_bytes());
        let sig = hmac_sha256(&self.validation_key, &hdr);
        if sig != hs {
            return Err(Error::Other("Packet signature validation check failed".into()));
        }
        Ok(body.to_vec())
    }

    // ---------------- response parsing ----------------

    fn parse_tls_response(&mut self, rsp: &[u8]) -> Result<Vec<u8>> {
        let mut rsp = rsp.to_vec();
        let mut app_data = Vec::new();
        while !rsp.is_empty() {
            while rsp.len() < 5 {
                rsp.push(0);
            }
            let t = rsp[0];
            let mj = rsp[1];
            let mn = rsp[2];
            let sz = u16::from_be_bytes([rsp[3], rsp[4]]) as usize;
            let pkt = rsp[5..5 + sz].to_vec();
            rsp.drain(..5 + sz);

            if mj != 3 || mn != 3 {
                return Err(Error::Other(format!(
                    "Unexpected TLS version {} {}",
                    mj, mn
                )));
            }
            match t {
                0x16 => self.handle_handshake(&pkt)?,
                0x14 => {
                    if pkt != [0x01] {
                        return Err(Error::Other("Unexpected ChangeCipherSpec payload".into()));
                    }
                    self.secure_rx = true;
                }
                0x17 => {
                    let d = self.handle_app_data(&pkt)?;
                    app_data.extend_from_slice(&d);
                }
                _ => {
                    return Err(Error::Other(format!(
                        "Dont know how to handle message type {:02x}",
                        t
                    )))
                }
            }
        }
        Ok(app_data)
    }

    fn handle_handshake(&mut self, handshake: &[u8]) -> Result<()> {
        let mut hs: Vec<u8> = if self.secure_rx {
            let h = self.decrypt(handshake)?;
            self.validate(0x16, &h)?
        } else {
            handshake.to_vec()
        };
        while !hs.is_empty() {
            while hs.len() < 4 {
                hs.push(0);
            }
            let t = hs[0];
            let l = ((hs[1] as usize) << 16) | ((hs[2] as usize) << 8) | hs[3] as usize;
            let p = hs[4..4 + l].to_vec();
            hs.drain(..4 + l);

            match t {
                2 => self.handle_server_hello(&p)?,
                0x0d => self.handle_cert_req(&p)?,
                0x0e => self.handle_server_hello_done(&p)?,
                0x14 => self.handle_finish(&p)?,
                _ => {
                    return Err(Error::Other(format!(
                        "Unknown handshake packet {:02x}",
                        t
                    )))
                }
            }
            let mut hdrp = vec![t];
            hdrp.extend_from_slice(&[(l >> 16) as u8, (l >> 8) as u8, l as u8]);
            hdrp.extend_from_slice(&p);
            self.update_neg(&hdrp);
        }
        Ok(())
    }

    fn handle_server_hello(&mut self, p: &[u8]) -> Result<()> {
        if p.len() < 2 || p[0] != 3 || p[1] != 3 {
            return Err(Error::Other("unexpected TLS version".into()));
        }
        let p = &p[2..];
        self.server_random = p[..0x20].to_vec();
        let l = p[0x20] as usize;
        // skip session id
        let p = &p[0x21 + l..];
        let suite = u16::from_be_bytes([p[0], p[1]]);
        if suite != 0xc005 {
            return Err(Error::Other(format!(
                "Server accepted unsupported cipher suite {:04x}",
                suite
            )));
        }
        let p = &p[2..];
        if p[0] != 0 {
            return Err(Error::Other("Server selected compression".into()));
        }
        let p = &p[1..];
        if !p.is_empty() {
            return Err(Error::Other("Not expecting any more data".into()));
        }
        Ok(())
    }

    fn handle_cert_req(&self, p: &[u8]) -> Result<()> {
        if p.len() < 4 {
            return Err(Error::Other("short cert_req".into()));
        }
        let sign_and_hash_algo = u16::from_be_bytes([p[0], p[1]]);
        if sign_and_hash_algo != 0x140 {
            return Err(Error::Other(format!(
                "Server requested a cert with an unsupported sign and hash algo combination {:04x}",
                sign_and_hash_algo
            )));
        }
        let l = u16::from_be_bytes([p[2], p[3]]) as usize;
        if l != 0 {
            return Err(Error::Other(
                "Server requested a cert with non-empty list of CAs".into(),
            ));
        }
        if !p[4..].is_empty() {
            return Err(Error::Other("Not expecting any more data".into()));
        }
        Ok(())
    }

    fn handle_server_hello_done(&self, p: &[u8]) -> Result<()> {
        if !p.is_empty() {
            return Err(Error::Other(
                "Not expecting any body for server hello done".into(),
            ));
        }
        Ok(())
    }

    fn handle_finish(&mut self, b: &[u8]) -> Result<()> {
        let hs_hash = self.handshake_hash.clone().finalize();
        let mut seed = Vec::with_capacity(16 + 32);
        seed.extend_from_slice(b"server finished");
        seed.extend_from_slice(&hs_hash);
        let verify_data = prf(&self.master_secret, &seed, 0xc);
        if verify_data != b {
            return Err(Error::Other("Final handshake check failed".into()));
        }
        Ok(())
    }

    fn handle_app_data(&mut self, b: &[u8]) -> Result<Vec<u8>> {
        if !self.secure_rx {
            return Err(Error::Other(
                "App payload before secure connection established".into(),
            ));
        }
        let d = self.decrypt(b)?;
        self.validate(0x17, &d)
    }

    // ---------------- flash blocks (parse_tls_flash) ----------------

    pub fn parse_tls_flash(&mut self, reply: &[u8]) -> Result<()> {
        let mut reply = reply.to_vec();
        while !reply.is_empty() {
            if reply.len() < 4 + 0x20 {
                break;
            }
            let id = u16::from_le_bytes([reply[0], reply[1]]);
            let sz = u16::from_le_bytes([reply[2], reply[3]]) as usize;
            let hs = reply[4..4 + 0x20].to_vec();
            let body = reply[4 + 0x20..4 + 0x20 + sz].to_vec();
            reply.drain(..4 + 0x20 + sz);

            if id == 0xffff {
                break;
            }
            if crate::crypto::sha256(&body) != hs[..] {
                return Err(Error::Other("hash mismatch".into()));
            }
            match id {
                4 => self.handle_priv(&body)?,
                6 => self.handle_ecdh(&body)?,
                3 => self.tls_cert = body,
                0 | 1 | 2 => {
                    if body.iter().any(|&b| b != 0) {
                        return Err(Error::Other("Expected empty block".into()));
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }

    pub fn handle_priv(&mut self, body: &[u8]) -> Result<()> {
        self.priv_blob = body.to_vec();
        if body.is_empty() || body[0] != 2 {
            return Err(Error::Other(format!(
                "Unknown private key prefix {:02x}",
                body.first().copied().unwrap_or(0)
            )));
        }
        let body = &body[1..];
        let (c, hs) = body.split_at(body.len() - 0x20);
        let sig = hmac_sha256(&self.psk_validation_key, c);
        if sig != hs {
            return Err(Error::PairingFailed);
        }
        let iv: [u8; 16] = c[..16].try_into().unwrap();
        let m = aes256_cbc_decrypt(&self.psk_encryption_key, &iv, &c[16..])
            .map_err(Error::Crypto)?;
        // standard pad: remove m[-1] bytes
        let pad_len = *m.last().unwrap() as usize;
        let m = &m[..m.len() - pad_len];
        if m.len() < 0x60 {
            return Err(Error::Other("priv blob too short".into()));
        }
        let (x, m) = m.split_at(0x20);
        let (y, m) = m.split_at(0x20);
        let (d, _) = m.split_at(0x20);
        let _ = x;
        let _ = y; // may be zero with the latest Windows driver; only d matters
        let d_be = rev32(d);
        self.priv_key = EcdsaSigningKey::from_bytes((&d_be).into())
            .map_err(|e| Error::Crypto(format!("bad d in priv blob: {:?}", e)))?;
        self.paired = true;
        Ok(())
    }

    pub fn handle_ecdh(&mut self, body: &[u8]) -> Result<()> {
        self.ecdh_blob = body.to_vec();
        if body.len() < 0x90 + 4 {
            return Err(Error::Other("ecdh blob too short".into()));
        }
        let (key, signature) = body.split_at(0x90);
        let x = rev32(&key[0x08..0x28]);
        let y = rev32(&key[0x4c..0x6c]);
        let mut sec1 = vec![0x04];
        sec1.extend_from_slice(&x);
        sec1.extend_from_slice(&y);
        self.ecdh_q = P256PublicKey::from_sec1_bytes(&sec1)
            .map_err(|e| Error::Crypto(format!("pubkey not on curve: {:?}", e)))?;

        let l = u32::from_le_bytes(signature[..4].try_into().unwrap()) as usize;
        let sig_der = &signature[4..4 + l];
        if signature[4 + l..].iter().any(|&b| b != 0) {
            return Err(Error::Other("Zeroes expected".into()));
        }
        // Verify ECDSA(SHA256(key)) with the firmware's public key.
        let mut fw_sec1 = vec![0x04];
        fw_sec1.extend_from_slice(&FWPUB_X);
        fw_sec1.extend_from_slice(&FWPUB_Y);
        let fw = VerifyingKey::from(
            P256PublicKey::from_sec1_bytes(&fw_sec1)
                .map_err(|e| Error::Crypto(format!("fw pubkey: {:?}", e)))?,
        );
        let digest = crate::crypto::sha256(key);
        let sig = EcdsaSignature::from_der(sig_der)
            .map_err(|e| Error::Crypto(format!("sig der: {:?}", e)))?;
        fw.verify_prehash(&digest, &sig)
            .map_err(|e| Error::Crypto(format!("ECDH blob signature invalid: {:?}", e)))?;
        Ok(())
    }

    // ---------------- pairing helpers (init_flash) ----------------

    pub fn make_tls_flash_block(id: u16, body: &[u8]) -> Vec<u8> {
        let mut out = Vec::with_capacity(4 + 0x20 + body.len());
        out.extend_from_slice(&id.to_le_bytes());
        out.extend_from_slice(&(body.len() as u16).to_le_bytes());
        out.extend_from_slice(&crate::crypto::sha256(body));
        out.extend_from_slice(body);
        out
    }

    pub fn make_tls_flash(&self) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(&Self::make_tls_flash_block(0, &[0]));
        b.extend_from_slice(&Self::make_tls_flash_block(4, &self.priv_blob));
        b.extend_from_slice(&Self::make_tls_flash_block(3, &self.tls_cert));
        b.extend_from_slice(&Self::make_tls_flash_block(5, CRT_HARDCODED));
        b.extend_from_slice(&Self::make_tls_flash_block(1, &[0u8; 0x100]));
        b.extend_from_slice(&Self::make_tls_flash_block(2, &[0u8; 0x100]));
        b.extend_from_slice(&Self::make_tls_flash_block(6, &self.ecdh_blob));
        b.resize(0x1000, 0xff);
        b
    }
}
