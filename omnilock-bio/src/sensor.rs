use crate::error::{assert_status, Error, Result};
use crate::hw_tables::{dev_info_lookup, DeviceInfo};
use crate::tls::Tls;

pub struct RomInfo {
    pub timestamp: u32,
    pub build: u32,
    pub major: u8,
    pub minor: u8,
    pub product: u8,
    pub u1: u8,
}

impl RomInfo {
    /// cmd 01
    pub fn get(tls: &mut Tls) -> Result<RomInfo> {
        let rsp = tls.cmd(&[0x01])?;
        assert_status(&rsp)?;
        let rsp = &rsp[2..];
        Ok(RomInfo {
            timestamp: u32::from_le_bytes(rsp[0..4].try_into().unwrap()),
            build: u32::from_le_bytes(rsp[4..8].try_into().unwrap()),
            major: rsp[8],
            minor: rsp[9],
            product: rsp[11],
            u1: rsp[15],
        })
    }
}

/// cmd 75 — sensor identity lookup (major, minor) -> DeviceInfo.
pub fn identify_sensor(tls: &mut Tls) -> Result<DeviceInfo> {
    let rsp = tls.cmd(&[0x75])?;
    assert_status(&rsp)?;
    let rsp = &rsp[2..];
    let zeroes = u32::from_le_bytes(rsp[0..4].try_into().unwrap());
    let minor = u16::from_le_bytes(rsp[4..6].try_into().unwrap());
    let major = u16::from_le_bytes(rsp[6..8].try_into().unwrap());
    if zeroes != 0 {
        return Err(Error::Other("identify_sensor: expected zeroes".into()));
    }
    dev_info_lookup(major, minor).ok_or_else(|| {
        Error::Other(format!(
            "identify_sensor: no device table entry for major=0x{:04x} minor=0x{:04x}",
            major, minor
        ))
    })
}

/// cmd 07 <B L B> — read 32-bit hardware register.
pub fn read_hw_reg32(tls: &mut Tls, addr: u32) -> Result<u32> {
    let mut cmd = vec![0x07];
    cmd.extend_from_slice(&addr.to_le_bytes());
    cmd.push(4);
    let rsp = tls.cmd(&cmd)?;
    assert_status(&rsp)?;
    Ok(u32::from_le_bytes(rsp[2..6].try_into().unwrap()))
}

/// cmd 08 <B L L B> — write 32-bit hardware register.
pub fn write_hw_reg32(tls: &mut Tls, addr: u32, val: u32) -> Result<()> {
    let mut cmd = vec![0x08];
    cmd.extend_from_slice(&addr.to_le_bytes());
    cmd.extend_from_slice(&val.to_le_bytes());
    cmd.push(4);
    let rsp = tls.cmd(&cmd)?;
    assert_status(&rsp)
}

/// cmd 05 02 00 — reboot; the device disconnects.
pub fn reboot(tls: &mut Tls) -> Result<()> {
    let rsp = tls.cmd(&[0x05, 0x02, 0x00])?;
    assert_status(&rsp)
}

/// cmd 51 00000000 — capture program status (0 or 7 = finished).
pub fn get_prg_status(tls: &mut Tls) -> Result<Vec<u8>> {
    tls.app(&[0x51, 0x00, 0x00, 0x00, 0x00])
}

/// cmd 51 00200000 — capture results.
pub fn get_prg_status2(tls: &mut Tls) -> Result<Vec<u8>> {
    tls.app(&[0x51, 0x00, 0x20, 0x00, 0x00])
}