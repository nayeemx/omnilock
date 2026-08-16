// Windows Hello (UserConsentVerifier) integration.
// HARDWARE FINDING (proven on HP EliteBook 850 G5, Synaptics VFS7552, 2026-08-16):
// the WBF driver refuses to deliver fingerprint captures to background apps
// (WinBioIdentify blocks forever, sensor logs zero events). The ONLY working
// fingerprint path on this machine is Windows Hello's own verification prompt.
// So OmniLock asks Windows Hello to verify the fingerprint — the same pipeline
// that signs the user in at the lock screen. Windows sign-in stays untouched.

use serde::Serialize;
use std::path::PathBuf;

#[derive(Serialize, Clone)]
pub struct BiometricStatus {
    pub available: bool,
    pub reason: String,
}

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

/// True only if Windows Hello's fingerprint verification can actually run:
/// the Biometric Service must be up AND a biometric credential provider must be
/// configured (a fingerprint/PIN is enrolled). Registry + service checks only
/// (fast, non-blocking); the real gate is the prompt at scan time.
pub fn check_biometric_available() -> BiometricStatus {
    let service_check = crate::hidden_cmd("sc")
        .args(["query", "WbioSrvc"])
        .output();
    let service_running = match &service_check {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains("RUNNING"),
        Err(_) => false,
    };
    if !service_running {
        return BiometricStatus {
            available: false,
            reason: "Windows Biometric Service is not running".to_string(),
        };
    }

    let hello_check = crate::hidden_cmd("reg")
        .args(["query",
            "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\WindowsHello",
            "/v", "Enabled", "/t", "REG_DWORD"])
        .output();
    let hello_enabled = match &hello_check {
        Ok(out) => String::from_utf8_lossy(&out.stdout).contains("0x1"),
        Err(_) => false,
    };
    if hello_enabled {
        return BiometricStatus {
            available: true,
            reason: "Windows Hello fingerprint verification is available".to_string(),
        };
    }

    let bio_check = crate::hidden_cmd("reg")
        .args(["query",
            "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Bio\\Credential Provider",
            "/s"])
        .output();
    let has_bio = match &bio_check {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            !stdout.is_empty() && !stdout.contains("ERROR")
        }
        Err(_) => false,
    };
    if has_bio {
        return BiometricStatus {
            available: true,
            reason: "Biometric provider detected. Set up a PIN in Windows Settings \u{2192} \
                     Accounts \u{2192} Sign-in options."
                .to_string(),
        };
    }

    BiometricStatus {
        available: false,
        reason: "Windows Hello not configured. Set up a fingerprint or PIN in Windows \
                 Settings \u{2192} Accounts \u{2192} Sign-in options."
            .to_string(),
    }
}

// ── core authentication via Windows Hello ────────────────────────────────────
//
// NOTE: the result type is `UserConsentVerificationResult` — the old code used
// `UserConsentVerifierResult` which does NOT exist as a WinRT type and made the
// prompt fail on real hardware ("Unable to find type ..."). Verified working:
// the prompt appears, the touch is accepted, `Verified` is returned.

pub async fn authenticate_biometric(message: String) -> Result<bool, String> {
    crate::logger::log("BIOMETRIC", &format!("Windows Hello scan start: {message}"));
    let ps = powershell51_path();
    let output = tokio::task::spawn_blocking(move || {
        let script = format!(
            "$ErrorActionPreference = 'Stop'; \
             try {{ \
               Add-Type -AssemblyName System.Runtime.WindowsRuntime; \
               $null = [Windows.Security.Credentials.UI.UserConsentVerifier,Windows.Security.Credentials.UI,ContentType=WindowsRuntime]; \
               $null = [Windows.Security.Credentials.UI.UserConsentVerificationResult,Windows.Security.Credentials.UI,ContentType=WindowsRuntime]; \
               $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ \
                 $_.Name -eq 'AsTask' -and \
                 $_.GetParameters().Count -eq 1 -and \
                 $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' \
               }})[0]; \
               $asTask = $asTaskGeneric.MakeGenericMethod([Windows.Security.Credentials.UI.UserConsentVerificationResult]); \
               $op = [Windows.Security.Credentials.UI.UserConsentVerifier]::RequestVerificationAsync('{}'); \
               $result = $asTask.Invoke($null, @($op)).Result; \
               $result.ToString() \
             }} catch {{ \
               'Error: ' + $_.Exception.Message \
             }}",
            message.replace('\'', "''")
        );

        crate::hidden_cmd(&ps)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-Command", &script])
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {e}"))?
    .map_err(|e| format!("Failed to run PowerShell: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if stdout.starts_with("Error:") {
        return Err(stdout);
    }
    if !output.status.success() && !stderr.is_empty() {
        return Err(format!("Biometric auth failed: {stderr}"));
    }

    match stdout.as_str() {
        "Verified" => {
            crate::logger::log("BIOMETRIC", "Windows Hello verification OK");
            Ok(true)
        }
        "Canceled" => Err("Authentication cancelled".to_string()),
        "DeviceNotPresent" => Err("No biometric device found".to_string()),
        "NotConfiguredForUser" => Err(
            "Windows Hello not set up. Set up a PIN in Windows Settings \u{2192} Accounts \
             \u{2192} Sign-in options."
                .to_string(),
        ),
        "DisabledByPolicy" => Err("Disabled by group policy".to_string()),
        "DeviceBusy" => Err("Device is busy".to_string()),
        _ => {
            if stdout.is_empty() {
                Err("No response from Windows Hello. Make sure Windows Hello is set up in \
                     Settings \u{2192} Accounts \u{2192} Sign-in options."
                    .to_string())
            } else {
                Err(format!("Unexpected response: {stdout}"))
            }
        }
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