use crate::crypto::sha256;
use crate::error::{assert_status, Error, Result};
use crate::hw_tables::{flash_ic_table_lookup, FlashIcInfo};
use crate::tls::Tls;

pub const DB_WRITE_ENABLE: &[u8] = include_bytes!("../resources/db_write_enable.bin");

pub struct PartitionInfo {
    pub id: u8,
    pub ty: u8,
    pub access_lvl: u16,
    pub offset: u32,
    pub size: u32,
}

impl PartitionInfo {
    pub const fn new(id: u8, ty: u8, access_lvl: u16, offset: u32, size: u32) -> PartitionInfo {
        PartitionInfo {
            id,
            ty,
            access_lvl,
            offset,
            size,
        }
    }

    /// `<BBHLL` + 4 zero bytes + sha256(header) — 48 bytes total.
    pub fn serialize(&self) -> Vec<u8> {
        let mut b = Vec::with_capacity(12);
        b.push(self.id);
        b.push(self.ty);
        b.extend_from_slice(&self.access_lvl.to_le_bytes());
        b.extend_from_slice(&self.offset.to_le_bytes());
        b.extend_from_slice(&self.size.to_le_bytes());
        let mut out = b.clone();
        out.extend_from_slice(&[0u8; 4]);
        out.extend_from_slice(&sha256(&b));
        out
    }
}

pub struct FlashInfo {
    pub ic: &'static FlashIcInfo,
    pub blocks: u16,
    pub unknown0: u16,
    pub blocksize: u16,
    pub unknown1: u16,
    pub partitions: Vec<PartitionInfo>,
}

pub fn get_flash_info(tls: &mut Tls) -> Result<FlashInfo> {
    let rsp = tls.cmd(&[0x3e])?;
    assert_status(&rsp)?;
    let rsp = &rsp[2..];
    let hdr = &rsp[..0xe];
    let jid0 = u16::from_le_bytes([hdr[0], hdr[1]]);
    let jid1 = u16::from_le_bytes([hdr[2], hdr[3]]);
    let blocks = u16::from_le_bytes([hdr[4], hdr[5]]);
    let unknown0 = u16::from_le_bytes([hdr[6], hdr[7]]);
    let blocksize = u16::from_le_bytes([hdr[8], hdr[9]]);
    let unknown1 = u16::from_le_bytes([hdr[10], hdr[11]]);
    let pcnt = u16::from_le_bytes([hdr[12], hdr[13]]) as usize;

    let ic = flash_ic_table_lookup(jid0, jid1, blocks as u32 * blocksize as u32).ok_or_else(|| {
        Error::Flash(format!(
            "Unknown flash IC. JEDEC id={:x}:{:x}, size={}x{}",
            jid0, jid1, blocks, blocksize
        ))
    })?;

    let mut partitions = Vec::with_capacity(pcnt);
    for i in 0..pcnt {
        let p = &rsp[0xe + i * 0xc..0xe + (i + 1) * 0xc];
        partitions.push(PartitionInfo {
            id: p[0],
            ty: p[1],
            access_lvl: u16::from_le_bytes([p[2], p[3]]),
            offset: u32::from_le_bytes([p[4], p[5], p[6], p[7]]),
            size: u32::from_le_bytes([p[8], p[9], p[10], p[11]]),
        });
    }

    Ok(FlashInfo {
        ic,
        blocks,
        unknown0,
        blocksize,
        unknown1,
        partitions,
    })
}

/// cmd 43 <partition> — firmware info; None when the partition is empty (b004).
pub fn get_fw_info(tls: &mut Tls, partition: u8) -> Result<Option<(u16, u16, u32)>> {
    let rsp = tls.cmd(&[0x43, partition])?;
    if rsp.len() == 2 && rsp[0] == 0xb0 && rsp[1] == 0x04 {
        return Ok(None);
    }
    assert_status(&rsp)?;
    let rsp = &rsp[2..];
    let major = u16::from_le_bytes([rsp[0], rsp[1]]);
    let minor = u16::from_le_bytes([rsp[2], rsp[3]]);
    let buildtime = u32::from_le_bytes([rsp[6], rsp[7], rsp[8], rsp[9]]);
    Ok(Some((major, minor, buildtime)))
}

pub fn write_enable(tls: &mut Tls) -> Result<()> {
    assert_status(&tls.cmd(DB_WRITE_ENABLE)?)
}

/// cmd 1a — commit pending flash operations; 0x0491 (nothing to commit) is fine.
pub fn call_cleanups(tls: &mut Tls) -> Result<()> {
    let rsp = tls.cmd(&[0x1a])?;
    let err = u16::from_le_bytes([rsp[0], rsp[1]]);
    if err == 0x0491 {
        return Ok(());
    }
    assert_status(&rsp)
}

pub fn erase_flash(tls: &mut Tls, partition: u8) -> Result<()> {
    write_enable(tls)?;
    let result = assert_status(&tls.cmd(&[0x3f, partition])?);
    let _ = call_cleanups(tls);
    result
}

/// cmd 40 <B B B H L L> — read `size` bytes from `addr` of `partition`.
pub fn read_flash(tls: &mut Tls, partition: u8, addr: u32, size: u32) -> Result<Vec<u8>> {
    let mut cmd = vec![0x40, partition, 1, 0, 0];
    cmd.extend_from_slice(&addr.to_le_bytes());
    cmd.extend_from_slice(&size.to_le_bytes());
    let rsp = tls.cmd(&cmd)?;
    assert_status(&rsp)?;
    let sz = u32::from_le_bytes(rsp[2..6].try_into().unwrap()) as usize;
    Ok(rsp[8..8 + sz].to_vec())
}

/// cmd 41 — write `buf` at `addr` of `partition`.
pub fn write_flash(tls: &mut Tls, partition: u8, addr: u32, buf: &[u8]) -> Result<()> {
    tls.cmd(DB_WRITE_ENABLE)?;
    let mut cmd = vec![0x41, partition, 1, 0, 0];
    cmd.extend_from_slice(&addr.to_le_bytes());
    cmd.extend_from_slice(&(buf.len() as u32).to_le_bytes());
    cmd.extend_from_slice(buf);
    let result = assert_status(&tls.cmd(&cmd)?);
    let _ = call_cleanups(tls);
    result
}

pub fn write_flash_all(tls: &mut Tls, partition: u8, ptr: u32, buf: &[u8]) -> Result<()> {
    let bs = 0x1000usize;
    let mut off = 0usize;
    while off < buf.len() {
        let chunk = &buf[off..(off + bs).min(buf.len())];
        write_flash(tls, partition, ptr + off as u32, chunk)?;
        off += chunk.len();
    }
    Ok(())
}

pub fn read_flash_all(tls: &mut Tls, partition: u8, start: u32, size: u32) -> Result<Vec<u8>> {
    let bs = 0x1000u32;
    let mut out = Vec::with_capacity(size as usize);
    let mut addr = start;
    while (addr as usize) < (start + size) as usize {
        let chunk = read_flash(tls, partition, addr, bs)?;
        out.extend_from_slice(&chunk);
        addr += bs;
    }
    out.truncate(size as usize);
    Ok(out)
}

pub fn read_tls_flash(tls: &mut Tls) -> Result<Vec<u8>> {
    read_flash(tls, 1, 0, 0x1000)
}