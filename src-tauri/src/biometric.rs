// Direct Windows Biometric Framework (WBF) integration.
// Uses windows-sys glob imports identically to file_locker.rs.
// WinBioOpenSession -> WinBioIdentify -> compare SID -> WinBioCloseSession.
// No PowerShell. No Windows Hello dialog. OmniLock owns the prompt.

use serde::Serialize;
use std::path::PathBuf;

use windows_sys::Win32::Devices::BiometricFramework::*;
use windows_sys::Win32::Foundation::*;
use windows_sys::Win32::Security::*;
use windows_sys::Win32::System::Threading::*;

// WBF constants — not always re-exported as named items in windows-sys 0.61,
// so we define them directly from the SDK header values.
const WINBIO_TYPE_FINGERPRINT: u32 = 0x00000008;
const WINBIO_POOL_SYSTEM: u32      = 0x00000001;
const WINBIO_FLAG_DEFAULT: u32     = 0x00000000;
const WINBIO_ID_TYPE_SID: u32      = 3; // WINBIO_IDENTITY_TYPE for account SID

#[derive(Serialize, Clone)]
pub struct BiometricStatus {
    pub available: bool,
    pub reason: String,
}

// ── token storage helpers ────────────────────────────────────────────────────

fn biometric_token_path() -> PathBuf {
    let app_data = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    PathBuf::from(app_data)
        .join("InnologyBD")
        .join("OmniLock")
        .join("biometric.token")
}

fn powershell51_path() -> String {
    let sys32 = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    let path = format!("{}\\System32\\WindowsPowerShell\\v1.0\\powershell.exe", sys32);
    if std::path::Path::new(&path).exists() { path } else { "powershell".to_string() }
}

// ── sensor availability check ─────────────────────────────────────────────────

pub fn check_biometric_available() -> BiometricStatus {
    match wbf_count_fingerprint_units() {
        Ok(n) if n > 0 => BiometricStatus {
            available: true,
            reason: format!("{n} fingerprint sensor(s) detected"),
        },
        Ok(_) => BiometricStatus {
            available: false,
            reason: "No fingerprint sensor found. Enroll a fingerprint in Windows \
                     Settings \u{2192} Accounts \u{2192} Sign-in options \u{2192} \
                     Fingerprint recognition.".to_string(),
        },
        Err(e) => BiometricStatus {
            available: false,
            reason: format!("Windows Biometric Framework unavailable: {e}"),
        },
    }
}

fn wbf_count_fingerprint_units() -> Result<usize, String> {
    unsafe {
        let mut schema_ptr: *mut WINBIO_UNIT_SCHEMA = std::ptr::null_mut();
        let mut count: usize = 0;
        let hr = WinBioEnumBiometricUnits(
            WINBIO_TYPE_FINGERPRINT,
            &mut schema_ptr,
            &mut count,
        );
        if hr < 0 {
            return Err(format!("WinBioEnumBiometricUnits HRESULT=0x{hr:08X}"));
        }
        if !schema_ptr.is_null() {
            WinBioFree(schema_ptr as *mut _);
        }
        Ok(count)
    }
}

// ── core authentication via WBF ───────────────────────────────────────────────

const SCAN_TIMEOUT_SECS: u64 = 30;

pub async fn authenticate_biometric(message: String) -> Result<bool, String> {
    crate::logger::log("BIOMETRIC", &format!("WBF scan start: {message}"));
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(SCAN_TIMEOUT_SECS),
        tokio::task::spawn_blocking(wbf_identify_current_user),
    )
    .await;
    match result {
        Err(_) => Err(format!(
            "Fingerprint scan timed out after {SCAN_TIMEOUT_SECS}s. Please touch the sensor."
        )),
        Ok(Err(e)) => Err(format!("Task join error: {e}")),
        Ok(Ok(r))  => r,
    }
}

fn wbf_identify_current_user() -> Result<bool, String> {
    unsafe {
        // 1. Get current user SID ─────────────────────────────────────────────
        let mut h_token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut h_token) == 0 {
            return Err("OpenProcessToken failed".to_string());
        }

        let mut len: u32 = 0;
        GetTokenInformation(h_token, TokenUser, std::ptr::null_mut(), 0, &mut len);
        let mut buf = vec![0u8; len as usize];
        if GetTokenInformation(
            h_token,
            TokenUser,
            buf.as_mut_ptr() as *mut _,
            len,
            &mut len,
        ) == 0
        {
            CloseHandle(h_token);
            return Err("GetTokenInformation failed".to_string());
        }
        CloseHandle(h_token);

        let token_user = &*(buf.as_ptr() as *const TOKEN_USER);
        let current_user_sid: PSID = token_user.User.Sid;

        // 2. Open WBF system fingerprint pool ─────────────────────────────────
        let mut session: u32 = 0;
        let hr_open = WinBioOpenSession(
            WINBIO_TYPE_FINGERPRINT,
            WINBIO_POOL_SYSTEM,
            WINBIO_FLAG_DEFAULT,
            std::ptr::null(),  // use all units in the pool
            0,                 // unitcount = 0 means "all"
            std::ptr::null(),  // default sensor database
            &mut session,
        );
        if hr_open < 0 {
            return Err(format!(
                "WinBioOpenSession failed (HRESULT=0x{hr_open:08X}). \
                 Enroll a fingerprint in Windows Settings \u{2192} Accounts \u{2192} \
                 Sign-in options \u{2192} Fingerprint recognition."
            ));
        }

        // 3. Block until a finger is placed ────────────────────────────────────
        let mut unit_id: u32 = 0;
        let mut identity: WINBIO_IDENTITY = std::mem::zeroed();
        let mut sub_factor: u8 = 0;
        let mut reject_detail: u32 = 0;

        let hr_id = WinBioIdentify(
            session,
            &mut unit_id,
            &mut identity,
            &mut sub_factor,
            &mut reject_detail,
        );
        WinBioCloseSession(session);

        if hr_id < 0 {
            return Err(format!(
                "WinBioIdentify failed (HRESULT=0x{hr_id:08X}). \
                 Make sure your finger is on the sensor and a fingerprint is enrolled."
            ));
        }

        // 4. Verify the identity is a SID (WINBIO_ID_TYPE_SID = 3) ─────────────
        if identity.Type != WINBIO_ID_TYPE_SID {
            return Err(
                "Fingerprint not enrolled for this Windows account. \
                 Enroll it in Settings \u{2192} Accounts \u{2192} Sign-in options."
                    .to_string(),
            );
        }

        // 5. Compare scanned SID with the current user ─────────────────────────
        // WINBIO_IDENTITY.Value.AccountSid.Data[68] holds the raw SID bytes.
        let scanned_sid: PSID = identity.Value.AccountSid.Data.as_ptr() as PSID;
        let matched = EqualSid(current_user_sid, scanned_sid) != 0;

        if !matched {
            return Err(
                "Fingerprint recognised but belongs to a different Windows user.".to_string(),
            );
        }

        Ok(true)
    }
}

// ── DPAPI token persistence ──────────────────────────────────────────────────

pub fn save_biometric_token(password: &str) -> Result<(), String> {
    let path = biometric_token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
    }
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         try {{ \
           Add-Type -AssemblyName System.Security; \
           $b=[System.Text.Encoding]::UTF8.GetBytes('{}'); \
           $ent=[System.Text.Encoding]::UTF8.GetBytes('OmniLock2026Biometric'); \
           $p=[System.Security.Cryptography.ProtectedData]::Protect($b,$ent,[System.Security.Cryptography.DataProtectionScope]::CurrentUser); \
           [System.IO.File]::WriteAllBytes('{}', $p); 'OK' \
         }} catch {{ Write-Error $_.Exception.Message; exit 1 }}",
        password.replace('\'', "''"),
        path.to_string_lossy().replace('\'', "''")
    );
    let ps = powershell51_path();
    let out = crate::hidden_cmd(&ps)
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|e| format!("DPAPI spawn failed: {e}"))?;
    if !out.status.success() {
        return Err(format!("DPAPI protect failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    Ok(())
}

pub fn load_biometric_token() -> Result<String, String> {
    let path = biometric_token_path();
    if !path.exists() {
        return Err("No biometric token found".to_string());
    }
    let script = format!(
        "$ErrorActionPreference='Stop'; \
         try {{ \
           Add-Type -AssemblyName System.Security; \
           $p=[System.IO.File]::ReadAllBytes('{}'); \
           $ent=[System.Text.Encoding]::UTF8.GetBytes('OmniLock2026Biometric'); \
           $b=[System.Security.Cryptography.ProtectedData]::Unprotect($p,$ent,[System.Security.Cryptography.DataProtectionScope]::CurrentUser); \
           [System.Text.Encoding]::UTF8.GetString($b) \
         }} catch {{ Write-Error $_.Exception.Message; exit 1 }}",
        path.to_string_lossy().replace('\'', "''")
    );
    let ps = powershell51_path();
    let out = crate::hidden_cmd(&ps)
        .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
        .output()
        .map_err(|e| format!("DPAPI spawn failed: {e}"))?;
    if !out.status.success() {
        return Err(format!("DPAPI unprotect failed: {}", String::from_utf8_lossy(&out.stderr)));
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() { Err("Empty password from DPAPI".to_string()) } else { Ok(s) }
}

pub fn remove_biometric_token() -> Result<(), String> {
    let path = biometric_token_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("remove token: {e}"))?;
    }
    Ok(())
}

pub fn has_biometric_token() -> bool {
    biometric_token_path().exists()
}
