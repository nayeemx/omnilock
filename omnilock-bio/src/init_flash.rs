use p256::ecdsa::signature::hazmat::PrehashSigner;
use p256::ecdsa::SigningKey as EcdsaSigningKey;
use p256::elliptic_curve::sec1::ToEncodedPoint;
use p256::PublicKey as P256PublicKey;
use rand::rngs::OsRng;

use crate::crypto::{aes256_cbc_encrypt, hmac_sha256};
use crate::error::{Error, Result};
use crate::flash::{
    erase_flash, get_flash_info, read_tls_flash, write_flash, FlashInfo, PartitionInfo,
};
use crate::hwkey::SystemInfo;
use crate::sensor::{identify_sensor, reboot, read_hw_reg32, write_hw_reg32};
use crate::tls::{hs_key, rev32, Tls, CRT_HARDCODED};
use crate::usb::{Device, PID, VID};

pub const RESET_BLOB_D51: &[u8] = include_bytes!("../resources/reset_blob_d51.bin");
pub const INIT_HARDCODED: &[u8] = include_bytes!("../resources/init_hardcoded.bin");
pub const INIT_HARDCODED_CLEAN_SLATE: &[u8] =
    include_bytes!("../resources/init_hardcoded_clean_slate.bin");

pub const FLASH_LAYOUT_HARDCODED: &[PartitionInfo] = &[
    PartitionInfo::new(1, 4, 7, 0x00001000, 0x00001000), // cert store
    PartitionInfo::new(2, 1, 2, 0x00002000, 0x0003e000), // xpfwext
    PartitionInfo::new(5, 5, 3, 0x00040000, 0x00008000),
    PartitionInfo::new(6, 6, 3, 0x00048000, 0x00008000), // calibration data
    PartitionInfo::new(4, 3, 5, 0x00050000, 0x00080000), // template database
];

pub const PARTITION_SIGNATURE: &[u8] = &[
    0x1d, 0xb0, 0x2a, 0x88, 0x6b, 0x00, 0x7e, 0x2b, 0x47, 0x26, 0x3b, 0xb8, 0xfe, 0x30, 0xbd,
    0x64, 0xa1, 0xf5, 0x8b, 0xea, 0x7b, 0x25, 0xf1, 0xe1, 0xba, 0x9a, 0xe0, 0x9a, 0xdd, 0x7e,
    0xcf, 0xf3, 0x63, 0x33, 0xf8, 0x19, 0x83, 0x39, 0xcd, 0xd7, 0x13, 0xf0, 0x43, 0x63, 0x37,
    0x10, 0xa1, 0x7b, 0xc7, 0xb3, 0xf4, 0x18, 0xf1, 0xd8, 0xff, 0x43, 0x5a, 0x1b, 0xf4, 0x7f,
    0x06, 0x5d, 0xff, 0xca, 0x72, 0x71, 0x09, 0x15, 0x22, 0x17, 0xfc, 0xe7, 0x3b, 0xf2, 0xbf,
    0x8e, 0x01, 0xa1, 0x64, 0x1f, 0x6a, 0x24, 0xb0, 0xc4, 0x92, 0xa6, 0xa3, 0xf1, 0x01, 0x14,
    0x05, 0x72, 0x75, 0x84, 0x68, 0x42, 0xb1, 0xc8, 0xb6, 0x6b, 0xd6, 0x70, 0x07, 0x38, 0x52,
    0x4d, 0x44, 0x71, 0xbc, 0xa3, 0x31, 0x5b, 0xa2, 0x3b, 0xb8, 0x32, 0x74, 0x32, 0x20, 0xad,
    0x19, 0x5b, 0x60, 0x55, 0x8a, 0xa7, 0x9a, 0x3e, 0xde, 0xb2, 0x60, 0x48, 0x34, 0xe2, 0xbb,
    0x62, 0xe8, 0x90, 0xb0, 0xce, 0x40, 0x5b, 0x3b, 0x8e, 0xf2, 0xfe, 0xc2, 0xaa, 0xb3, 0xe2,
    0x2b, 0xff, 0x23, 0xf8, 0x9a, 0x58, 0xff, 0x0d, 0xc0, 0x15, 0xfe, 0xce, 0x5d, 0x3e, 0xd3,
    0xf5, 0x49, 0x6a, 0xce, 0x87, 0x9a, 0x92, 0x98, 0x0a, 0xec, 0x9d, 0x85, 0xeb, 0x7e, 0x9d,
    0xf2, 0x45, 0xea, 0xe0, 0x3a, 0x41, 0xac, 0xfd, 0x4e, 0x7d, 0x1c, 0xb1, 0xdb, 0xd0, 0xdf,
    0x42, 0xd5, 0x34, 0x90, 0x4d, 0xe0, 0x0b, 0x63, 0x89, 0xf6, 0x88, 0x67, 0x64, 0x6e, 0x9d,
    0x7c, 0x3d, 0x0b, 0x1d, 0xff, 0xd7, 0x40, 0x70, 0xb2, 0xd0, 0xf2, 0x04, 0x9b, 0x9f, 0x1d,
    0xc7, 0xb0, 0xc9, 0x65, 0x1c, 0x59, 0xbe, 0x3e, 0xa8, 0x91, 0x67, 0x47, 0x25, 0xe1, 0xf2,
    0xf7, 0xa4, 0x84, 0xa9, 0x41, 0x61, 0x5b, 0x80, 0x21, 0x11, 0x05, 0x97, 0x83, 0x69, 0xcf,
    0x71,
];

fn is_d51_reset_family() -> bool {
    (VID, PID) == (0x138a, 0x00ab) || (VID, PID) == (0x06cb, 0x00b7)
}

fn with_hdr(id: u16, buf: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(4 + buf.len());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&(buf.len() as u16).to_le_bytes());
    out.extend_from_slice(buf);
    out
}

/// Encrypt (x || y || d, all wire-reversed) with the pairing PSK.
pub fn encrypt_key(tls: &Tls, d: &[u8; 32], x: &[u8; 32], y: &[u8; 32]) -> Vec<u8> {
    let mut m = Vec::with_capacity(0x60 + 16);
    m.extend_from_slice(x);
    m.extend_from_slice(y);
    m.extend_from_slice(d);
    let l = 16 - (m.len() % 16);
    m.extend(std::iter::repeat(l as u8).take(l)); // value l, not l-1 (init_flash padding)

    let mut iv = [0u8; 16];
    use rand::RngCore;
    OsRng.fill_bytes(&mut iv);
    let c = aes256_cbc_encrypt(&tls.psk_encryption_key, &iv, &m);
    let mut out = Vec::with_capacity(1 + 16 + c.len() + 32);
    out.push(0x02);
    out.extend_from_slice(&iv);
    out.extend_from_slice(&c);
    let sig = hmac_sha256(&tls.psk_validation_key, &out[1..]);
    out.extend_from_slice(&sig);
    out
}

/// Sign msg with the hs_key-derived P-256 key; result: <L der> + der, padded to 444 bytes.
fn make_cert(client_public: &P256PublicKey) -> Vec<u8> {
    let aff = client_public.as_affine();
    let enc = aff.to_encoded_point(false);
    let (x, y) = match enc.coordinates() {
        p256::elliptic_curve::sec1::Coordinates::Uncompressed { x, y } => (x, y),
        _ => unreachable!("sec1 point has no coordinates"),
    };
    let mut msg = Vec::with_capacity(0xc0);
    msg.extend_from_slice(&0x17u32.to_le_bytes());
    msg.extend_from_slice(&0x20u32.to_le_bytes());
    msg.extend_from_slice(&rev32(x.as_slice()));
    msg.extend_from_slice(&[0u8; 0x24]);
    msg.extend_from_slice(&rev32(y.as_slice()));
    msg.extend_from_slice(&[0u8; 0x4c]);
    // msg is now 4+4+32+0x24+32+0x4c = 0xc0 bytes

let sk = EcdsaSigningKey::from(hs_key());
    let digest = crate::crypto::sha256(&msg);
    let sig: p256::ecdsa::Signature = sk
        .sign_prehash(&digest)
        .expect("sign make_cert msg");
    let der = sig.to_der().as_bytes().to_vec();

    let mut out = Vec::with_capacity(4 + der.len());
    out.extend_from_slice(&(der.len() as u32).to_le_bytes());
    out.extend_from_slice(&der);
    msg.extend_from_slice(&out);
    msg.resize(444, 0); // python pads with zeros to 444
    msg
}

fn serialize_flash_params(ic: &crate::hw_tables::FlashIcInfo) -> Vec<u8> {
    let mut b = Vec::with_capacity(12);
    b.extend_from_slice(&ic.size.to_le_bytes());
    b.extend_from_slice(&ic.sector_size.to_le_bytes());
    b.extend_from_slice(&[0, 0]);
    b.push(ic.sector_erase_cmd);
    b.push(0);
    b
}

/// cmd 4f â€” write the partition table + certs into the (clean) flash.
fn partition_flash(
    tls: &mut Tls,
    info: &FlashInfo,
    layout: &[PartitionInfo],
    signature: &[u8],
    client_public: &P256PublicKey,
) -> Result<()> {
    let mut cmd = vec![0x4f, 0x00, 0x00, 0x00, 0x00];

    cmd.extend_from_slice(&with_hdr(0, &serialize_flash_params(info.ic)));

    let mut parts = Vec::new();
    for p in layout {
        parts.extend_from_slice(&p.serialize());
    }
    parts.extend_from_slice(signature);
    cmd.extend_from_slice(&with_hdr(1, &parts));

    cmd.extend_from_slice(&with_hdr(5, &make_cert(client_public)));
    cmd.extend_from_slice(&with_hdr(3, CRT_HARDCODED));

    let rsp = tls.cmd(&cmd)?;
    crate::error::assert_status(&rsp)?;
    let rsp = &rsp[2..];
    let crt_len = u32::from_le_bytes(rsp[..4].try_into().unwrap()) as usize;
    tls.tls_cert = rsp[4..4 + crt_len].to_vec();
    Ok(())
}

/// Put d51-family ROMs into the state expected by their reset payload
/// (cleartext preflight observed before packet 78 in the PR #256 capture).
fn prepare_clean_slate_reset(tls: &mut Tls) -> Result<()> {
    write_hw_reg32(tls, 0x8000205c, 7)?;
    let v = read_hw_reg32(tls, 0x80002080)?;
    if v != 2 && v != 3 {
        return Err(Error::Other("Unexpected register value during clean-slate reset".into()));
    }
    identify_sensor(tls)?;
    crate::flash::call_cleanups(tls)?;
    Ok(())
}

/// Initialize flash if it is not yet partitioned. For an already-partitioned
/// (paired) sensor this is a no-op.
pub fn init_flash(tls: &mut Tls) -> Result<()> {
    let info = get_flash_info(tls)?;

    if !info.partitions.is_empty() {
        println!(
            "Flash already initialized: {} partitions, IC {} ({} bytes)",
            info.partitions.len(),
            info.ic.name,
            info.ic.size
        );
        return Ok(());
    }
    println!("Flash was not initialized yet. Formatting...");

    if is_d51_reset_family() {
        prepare_clean_slate_reset(tls)?;
    }

    let rsp = tls.usb.cmd(RESET_BLOB_D51)?;
    crate::error::assert_status(&rsp)?;

    // Client pairing keypair
    let sk = p256::SecretKey::random(&mut OsRng);
    let client_private: [u8; 32] = sk.to_bytes().into();
    let client_public = sk.public_key();

    let layout = FLASH_LAYOUT_HARDCODED;
    let signature = PARTITION_SIGNATURE;

    partition_flash(tls, &info, layout, signature, &client_public)?;

    crate::sensor::RomInfo::get(tls)?;

    let rsp = {
        let r = tls.usb.cmd(&[0x50]);
        let _ = crate::flash::call_cleanups(tls);
        r?
    };
    crate::error::assert_status(&rsp)?;
    let rsp = &rsp[2..];
    let l = u32::from_le_bytes(rsp[..4].try_into().unwrap()) as usize;
    if rsp.len() != l {
        return Err(Error::Other("Length mismatch".into()));
    }
    let (zeroes, tail) = rsp.split_at(rsp.len() - 400);
    if zeroes[4..].iter().any(|&b| b != 0) {
        return Err(Error::Other("Expected zeroes".into()));
    }

    tls.handle_ecdh(tail)?;

    // client_public wire coords for the priv blob
    let aff = client_public.as_affine();
    let enc = aff.to_encoded_point(false);
    let (x, y) = match enc.coordinates() {
        p256::elliptic_curve::sec1::Coordinates::Uncompressed { x, y } => (x, y),
        _ => unreachable!("sec1 point has no coordinates"),
    };
    let xw = rev32(x.as_slice());
    let yw = rev32(y.as_slice());
    let priv_blob = encrypt_key(tls, &client_private, &xw, &yw);
    tls.handle_priv(&priv_blob)?;

    tls.open()?;

    // Wipe newly created partitions clean
    for p in [1u8, 2, 5, 6, 4] {
        erase_flash(tls, p)?;
    }

    // Persist certs and keys on the cert partition.
    write_flash(tls, 1, 0, &tls.make_tls_flash())?;

    println!("Pairing complete. Rebooting the sensor...");
    let _ = reboot(tls);
    Ok(())
}

/// Full open sequence: send_init + init_flash + TLS + firmware check.
pub fn open_common(dev: Device, info: &SystemInfo) -> Result<Tls> {
    let mut tls = Tls::new(dev, &crate::hwkey::hw_key_bytes(info));

    send_init(&mut tls)?;
    init_flash(&mut tls)?;
    let flash_bytes = read_tls_flash(&mut tls)?;
    tls.parse_tls_flash(&flash_bytes)?;
    tls.open()?;
    Ok(tls)
}

/// usb.send_init: cmd 01, cmd 19, cmd 43 02, init_hardcoded (+ clean-slate variant).
pub fn send_init(tls: &mut Tls) -> Result<()> {
    crate::error::assert_status(&tls.usb.cmd(&[0x01])?)?;
    crate::error::assert_status(&tls.usb.cmd(&[0x19])?)?;
    let rsp = tls.usb.cmd(&[0x43, 0x02])?; // get_fw_info(2)
    crate::error::assert_status(&tls.usb.cmd(INIT_HARDCODED)?)?;
    let err = u16::from_le_bytes([rsp[0], rsp[1]]);
    if err != 0 {
        println!("Clean slate: loading init_hardcoded_clean_slate");
        let _ = tls.usb.cmd(INIT_HARDCODED_CLEAN_SLATE)?;
    }
    Ok(())
}
