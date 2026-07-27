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
            "try { $result = [Windows.Security.Credentials.UI.UserConsentVerifier, Windows.Security.Credentials.UI, ContentType = WindowsRuntime]::CheckAvailabilityAsync().GetAwaiter().GetResult(); $result.ToString() } catch { 'Unavailable' }"])
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            match stdout.as_str() {
                "Available" => BiometricStatus { available: true, reason: "Windows Hello is available".to_string() },
                "DeviceNotPresent" => BiometricStatus { available: false, reason: "No biometric device found".to_string() },
                "NotConfiguredForUser" => BiometricStatus { available: false, reason: "Windows Hello not set up for this user".to_string() },
                "DisabledByPolicy" => BiometricStatus { available: false, reason: "Disabled by group policy".to_string() },
                "DeviceBusy" => BiometricStatus { available: false, reason: "Biometric device is busy".to_string() },
                _ => BiometricStatus { available: false, reason: format!("Status: {}", stdout) },
            }
        }
        Err(e) => BiometricStatus { available: false, reason: format!("Failed to check: {}", e) },
    }
}

pub async fn authenticate_biometric(message: String) -> Result<bool, String> {
    let script = format!(
        "try {{ $result = [Windows.Security.Credentials.UI.UserConsentVerifier, Windows.Security.Credentials.UI, ContentType = WindowsRuntime]::RequestVerificationAsync('{}').GetAwaiter().GetResult(); $result.ToString() }} catch {{ 'Error: {{}}' }}",
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
            if stdout.starts_with("Error:") {
                Err(stdout)
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
