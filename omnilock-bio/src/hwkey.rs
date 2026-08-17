use windows_sys::Win32::System::SystemInformation::GetSystemFirmwareTable;

const RSMB: u32 = 0x52534d42; // "RSMB"

fn smbios_raw() -> Option<Vec<u8>> {
    let size = unsafe { GetSystemFirmwareTable(RSMB, 0, std::ptr::null_mut(), 0) };
    if size == 0 {
        return None;
    }
    let mut buf = vec![0u8; size as usize];
    let n = unsafe { GetSystemFirmwareTable(RSMB, 0, buf.as_mut_ptr() as _, size) };
    if n == 0 {
        return None;
    }
    buf.truncate(n as usize);
    Some(buf)
}

/// Extract a 1-indexed string from an SMBIOS string section.
fn smbios_string(strings: &[u8], idx: u8) -> Option<String> {
    if idx == 0 {
        return None;
    }
    let mut cur: u8 = 1;
    for part in strings.split(|&b| b == 0) {
        if part.is_empty() {
            continue;
        }
        if cur == idx {
            return Some(
                String::from_utf8_lossy(part)
                    .trim_end_matches(' ')
                    .to_string(),
            );
        }
        cur += 1;
    }
    None
}

/// SMBIOS Type 1 (System Information): Product Name + Serial Number.
/// These form the hw_key used to derive the pairing PSK (GWK).
pub struct SystemInfo {
    pub product_name: String,
    pub serial_number: String,
}

impl SystemInfo {
    pub fn read() -> Option<SystemInfo> {
        let raw = smbios_raw()?;
        // RawSMBIOSData: 4-byte signature ("RSMB") + major + minor + revision + u32 length,
        // then the table data.
        if raw.len() < 8 {
            return None;
        }
        let mut off = 8usize;
        let mut product_name = String::new();
        let mut serial_number = String::new();
        while off + 4 <= raw.len() {
            let ty = raw[off];
            let len = raw[off + 1] as usize;
            if ty == 127 {
                break; // end-of-table marker
            }
            if len < 4 || off + len > raw.len() {
                break;
            }
            if ty == 1 {
                let strs_start = off + len;
                let mut end = strs_start;
                while end + 1 < raw.len() && !(raw[end] == 0 && raw[end + 1] == 0) {
                    end += 1;
                }
                let strings = &raw[strs_start..=end];
                product_name = smbios_string(strings, raw[off + 5]).unwrap_or_default();
                serial_number = smbios_string(strings, raw[off + 7]).unwrap_or_default();
                if !product_name.is_empty() && !serial_number.is_empty() {
                    break;
                }
            }
            // skip the string section
            let mut p = off + len;
            while p + 1 < raw.len() && !(raw[p] == 0 && raw[p + 1] == 0) {
                p += 1;
            }
            off = p + 2;
        }
        if product_name.is_empty() {
            return None;
        }
        Some(SystemInfo {
            product_name,
            serial_number,
        })
    }
}

/// hw_key = product_name + '\0' + serial_number + '\0'
pub fn hw_key_bytes(info: &SystemInfo) -> Vec<u8> {
    let mut k = Vec::new();
    k.extend_from_slice(info.product_name.as_bytes());
    k.push(0);
    k.extend_from_slice(info.serial_number.as_bytes());
    k.push(0);
    k
}