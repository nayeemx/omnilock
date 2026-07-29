use serde::Serialize;

#[derive(Serialize)]
pub struct Diagnostics {
    pub version: String,
    pub log_tail: String,
    pub locked_items_check: Vec<LockItemCheck>,
    pub biometric: BiometricCheck,
    pub service: ServiceCheck,
    pub drive_states: Vec<DriveCheck>,
}

#[derive(Serialize)]
pub struct LockItemCheck {
    pub path: String,
    pub kind: String,
    pub exists: bool,
    pub deny_ace_present: bool,
    pub check_error: String,
}

#[derive(Serialize)]
pub struct BiometricCheck {
    pub hardware_available: bool,
    pub token_exists: bool,
    pub token_load_ok: bool,
    pub reason: String,
    pub last_error: String,
}

#[derive(Serialize)]
pub struct ServiceCheck {
    pub running: bool,
}

#[derive(Serialize)]
pub struct DriveCheck {
    pub drive_letter: String,
    pub policy_active: bool,
}

pub fn collect_diagnostics() -> Diagnostics {
    let version = env!("CARGO_PKG_VERSION").to_string();
    let log_tail = crate::logger::read_log(16384);

    let locked_items_check = check_locked_items();
    let biometric = check_biometric();
    let service = check_service();
    let drive_states = check_drives();

    Diagnostics {
        version,
        log_tail,
        locked_items_check,
        biometric,
        service,
        drive_states,
    }
}

fn check_locked_items() -> Vec<LockItemCheck> {
    let mut checks = Vec::new();
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
    let items_path = std::path::PathBuf::from(&appdata)
        .join("InnologyBD\\OmniLock\\locked_items.json");
    if !items_path.exists() {
        return checks;
    }
    if let Ok(json) = std::fs::read_to_string(&items_path) {
        if let Ok(items) = serde_json::from_str::<Vec<crate::models::UnlockTarget>>(&json) {
            for item in items {
                let path = if item.target_type == "drive" {
                    format!("{}:", item.target_id)
                } else {
                    item.target_id.clone()
                };
                let exists = std::path::Path::new(&path).exists();
                let (deny_ace_present, check_error) = if !exists {
                    (false, "Path does not exist".to_string())
                } else {
                    match crate::file_locker::verify_lock(&path) {
                        Ok(true) => (true, String::new()),
                        Ok(false) => (false, "DENY ACE not found".to_string()),
                        Err(e) => (false, e),
                    }
                };

                checks.push(LockItemCheck {
                    path: item.target_id,
                    kind: item.target_type,
                    exists,
                    deny_ace_present,
                    check_error,
                });
            }
        }
    }
    checks
}

fn check_biometric() -> BiometricCheck {
    let hardware = crate::biometric::check_biometric_available();
    let token_exists = crate::biometric::has_biometric_token();
    let token_load_ok = if token_exists {
        matches!(crate::biometric::load_biometric_token(), Ok(ref s) if !s.is_empty())
    } else {
        false
    };
    let last_error = String::new();

    BiometricCheck {
        hardware_available: hardware.available,
        token_exists,
        token_load_ok,
        reason: hardware.reason,
        last_error,
    }
}

fn check_service() -> ServiceCheck {
    ServiceCheck {
        running: crate::service_client::is_service_running(),
    }
}

fn get_nodrives_mask() -> u32 {
    let out = crate::hidden_cmd("reg")
        .args(["query", "HKLM\\SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Policies\\Explorer", "/v", "NoDrives"])
        .output();
    match out {
        Ok(o) => {
            let s = String::from_utf8_lossy(&o.stdout);
            // Parse hex value from reg query output:
            //   "    NoDrives    REG_DWORD    0x123"
            if let Some(hex_part) = s.split("REG_DWORD").nth(1) {
                let trimmed = hex_part.trim();
                if let Some(rest) = trimmed.strip_prefix("0x") {
                    u32::from_str_radix(rest.trim(), 16).unwrap_or(0)
                } else {
                    0
                }
            } else {
                0
            }
        }
        Err(_) => 0,
    }
}

fn check_drives() -> Vec<DriveCheck> {
    let mut checks = Vec::new();
    let mask = get_nodrives_mask();
    for letter in 'C'..='Z' {
        let path = format!("{}:\\", letter);
        if std::path::Path::new(&path).exists() {
            let bit = 1u32 << (letter as u32 - 'A' as u32);
            let policy_active = (mask & bit) != 0;
            checks.push(DriveCheck {
                drive_letter: letter.to_string(),
                policy_active,
            });
        }
    }
    checks
}
