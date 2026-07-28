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

pub fn check_biometric_available() -> BiometricStatus {
    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "$ErrorActionPreference = 'SilentlyContinue'; \
             Add-Type -AssemblyName System.Runtime.WindowsRuntime; \
             $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object { $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' })[0]; \
             $asTask = $asTaskGeneric.MakeGenericMethod([Windows.Security.Credentials.UI.UserConsentVerifierAvailability]); \
             $asTask.Invoke($null, @([Windows.Security.Credentials.UI.UserConsentVerifier]::CheckAvailabilityAsync())) | Out-Null; \
             $result = $asTask.Invoke($null, @([Windows.Security.Credentials.UI.UserConsentVerifier]::CheckAvailabilityAsync())).Result; \
             $result.ToString()"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout.contains("Available") && !stdout.contains("DeviceNotPresent") && !stdout.contains("NotConfigured") {
                return BiometricStatus { available: true, reason: "Windows Hello is available".to_string() };
            }
        }
        Err(_) => {}
    }

    let pin_check = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "$ErrorActionPreference = 'SilentlyContinue'; \
             $kp = Get-ItemProperty 'HKLM:\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Credential Providers\\{D6886603-9D4F-4D48-A512-AD2EDE1AC5B1}' -ErrorAction SilentlyContinue; \
             if ($kp) { 'HasPINProvider' } else { 'NoPINProvider' }"])
        .output();

    if let Ok(out) = pin_check {
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if stdout.contains("HasPINProvider") {
            return BiometricStatus { available: true, reason: "Windows Hello credential provider found".to_string() };
        }
    }

    let service_check = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command",
            "Get-Service -Name 'WbioSrvc' -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Status"])
        .output();

    match service_check {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if stdout == "Running" {
                return BiometricStatus { available: true, reason: "Windows Biometric Service is running".to_string() };
            }
            BiometricStatus { available: false, reason: format!("Biometric service status: {}. Set up a PIN or fingerprint in Windows Settings > Accounts > Sign-in options.", stdout) }
        }
        Err(e) => BiometricStatus { available: false, reason: format!("Failed to check biometric service: {}", e) },
    }
}

pub async fn authenticate_biometric(message: String) -> Result<bool, String> {
    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         Add-Type -AssemblyName System.Runtime.WindowsRuntime; \
         $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ $_.Name -eq 'AsTask' -and $_.GetParameters().Count -eq 1 -and $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' }})[0]; \
         $asTask = $asTaskGeneric.MakeGenericMethod([Windows.Security.Credentials.UI.UserConsentVerifierResult]); \
         $op = [Windows.Security.Credentials.UI.UserConsentVerifier]::RequestVerificationAsync('{}'); \
         $result = $asTask.Invoke($null, @($op)).Result; \
         $result.ToString()",
        message.replace('\'', "''")
    );

    let output = tokio::task::spawn_blocking(move || {
        std::process::Command::new("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match stdout.as_str() {
        "Verified" => Ok(true),
        "Canceled" => Err("Authentication cancelled".to_string()),
        "DeviceNotPresent" => Err("No biometric device found".to_string()),
        "NotConfiguredForUser" => Err("Windows Hello not set up".to_string()),
        "DisabledByPolicy" => Err("Disabled by group policy".to_string()),
        "DeviceBusy" => Err("Device is busy".to_string()),
        _ => {
            let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
            if !stderr.is_empty() {
                Err(format!("Biometric auth failed: {}", stderr))
            } else if stdout.is_empty() {
                Err("No response from Windows Hello".to_string())
            } else {
                Err(format!("Unexpected response: {}", stdout))
            }
        }
    }
}

pub fn save_biometric_token(password: &str) -> Result<(), String> {
    let path = biometric_token_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir: {}", e))?;
    }

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!(
            "$bytes = [System.Text.Encoding]::UTF8.GetBytes('{}'); $protected = [System.Security.Cryptography.ProtectedData]::Protect($bytes, $null, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); [System.IO.File]::WriteAllBytes('{}', $protected)",
            password.replace('\'', "''"),
            path.to_string_lossy().replace('\'', "''")
        )])
        .output()
        .map_err(|e| format!("Failed to run DPAPI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("DPAPI protect failed: {}", stderr));
    }

    Ok(())
}

pub fn load_biometric_token() -> Result<String, String> {
    let path = biometric_token_path();
    if !path.exists() {
        return Err("No biometric token found".to_string());
    }

    let output = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", &format!(
            "$protected = [System.IO.File]::ReadAllBytes('{}'); $bytes = [System.Security.Cryptography.ProtectedData]::Unprotect($protected, $null, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); [System.Text.Encoding]::UTF8.GetString($bytes)",
            path.to_string_lossy().replace('\'', "''")
        )])
        .output()
        .map_err(|e| format!("Failed to run DPAPI: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("DPAPI unprotect failed: {}", stderr));
    }

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if stdout.is_empty() {
        return Err("Empty password from DPAPI".to_string());
    }

    Ok(stdout)
}

pub fn remove_biometric_token() -> Result<(), String> {
    let path = biometric_token_path();
    if path.exists() {
        std::fs::remove_file(&path).map_err(|e| format!("Failed to remove token: {}", e))?;
    }
    Ok(())
}

pub fn has_biometric_token() -> bool {
    biometric_token_path().exists()
}
