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
    let service_check = crate::hidden_cmd("sc")
        .args(["query", "WbioSrvc"])
        .output();

    let service_running = match &service_check {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            stdout.contains("RUNNING")
        }
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
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            stdout.contains("0x1")
        }
        Err(_) => false,
    };

    if hello_enabled {
        return BiometricStatus {
            available: true,
            reason: "Windows Hello is available".to_string(),
        };
    }

    let pin_check = crate::hidden_cmd("reg")
        .args(["query",
            "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Authentication\\Bio\\Credential Provider",
            "/s"])
        .output();

    let has_bio = match &pin_check {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            !stdout.is_empty() && !stdout.contains("ERROR")
        }
        Err(_) => false,
    };

    if has_bio {
        return BiometricStatus {
            available: true,
            reason: "Biometric provider detected. Set up a PIN in Windows Settings > Accounts > Sign-in options.".to_string(),
        };
    }

    BiometricStatus {
        available: false,
        reason: "Windows Hello not configured. Set up a PIN or fingerprint in Windows Settings > Accounts > Sign-in options.".to_string(),
    }
}

pub async fn authenticate_biometric(message: String) -> Result<bool, String> {
    let output = tokio::task::spawn_blocking(move || {
        let script = format!(
            "$ErrorActionPreference = 'Stop'; \
             try {{ \
               Add-Type -AssemblyName System.Runtime.WindowsRuntime; \
               [void][System.WindowsRuntimeSystemExtensions]; \
               $asTaskGeneric = ([System.WindowsRuntimeSystemExtensions].GetMethods() | Where-Object {{ \
                 $_.Name -eq 'AsTask' -and \
                 $_.GetParameters().Count -eq 1 -and \
                 $_.GetParameters()[0].ParameterType.Name -eq 'IAsyncOperation`1' \
               }})[0]; \
               $asTask = $asTaskGeneric.MakeGenericMethod([Windows.Security.Credentials.UI.UserConsentVerifierResult]); \
               $op = [Windows.Security.Credentials.UI.UserConsentVerifier]::RequestVerificationAsync('{}'); \
               $result = $asTask.Invoke($null, @($op)).Result; \
               $result.ToString() \
             }} catch {{ \
               'Error: ' + $_.Exception.Message \
             }}",
            message.replace('\'', "''")
        );

        crate::hidden_cmd("powershell")
            .args(["-NoProfile", "-Command", &script])
            .output()
    })
    .await
    .map_err(|e| format!("Task join error: {}", e))?
    .map_err(|e| format!("Failed to run PowerShell: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();

    if stdout.starts_with("Error:") {
        return Err(stdout);
    }

    if !output.status.success() && !stderr.is_empty() {
        return Err(format!("Biometric auth failed: {}", stderr));
    }

    match stdout.as_str() {
        "Verified" => Ok(true),
        "Canceled" => Err("Authentication cancelled".to_string()),
        "DeviceNotPresent" => Err("No biometric device found".to_string()),
        "NotConfiguredForUser" => Err("Windows Hello not set up. Set up a PIN in Windows Settings > Accounts > Sign-in options.".to_string()),
        "DisabledByPolicy" => Err("Disabled by group policy".to_string()),
        "DeviceBusy" => Err("Device is busy".to_string()),
        _ => {
            if stdout.is_empty() {
                Err("No response from Windows Hello. Make sure Windows Hello is set up in Settings > Accounts > Sign-in options.".to_string())
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

    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         try {{ \
           Add-Type -AssemblyName System.Security; \
           $bytes = [System.Text.Encoding]::UTF8.GetBytes('{}'); \
           $entropy = [System.Text.Encoding]::UTF8.GetBytes('OmniLock2026Biometric'); \
           $protected = [System.Security.Cryptography.ProtectedData]::Protect($bytes, $entropy, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); \
           [System.IO.File]::WriteAllBytes('{}', $protected); \
           'OK' \
         }} catch {{ \
           Write-Error $_.Exception.Message; \
           exit 1 \
         }}",
        password.replace('\'', "''"),
        path.to_string_lossy().replace('\'', "''")
    );

    let output = crate::hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", &script])
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

    let script = format!(
        "$ErrorActionPreference = 'Stop'; \
         try {{ \
           Add-Type -AssemblyName System.Security; \
           $protected = [System.IO.File]::ReadAllBytes('{}'); \
           $entropy = [System.Text.Encoding]::UTF8.GetBytes('OmniLock2026Biometric'); \
           $bytes = [System.Security.Cryptography.ProtectedData]::Unprotect($protected, $entropy, [System.Security.Cryptography.DataProtectionScope]::CurrentUser); \
           [System.Text.Encoding]::UTF8.GetString($bytes) \
         }} catch {{ \
           Write-Error $_.Exception.Message; \
           exit 1 \
         }}",
        path.to_string_lossy().replace('\'', "''")
    );

    let output = crate::hidden_cmd("powershell")
        .args(["-NoProfile", "-Command", &script])
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
