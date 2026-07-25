# OmniLock - Security Specification

**Document Version:** 2.0.0

---

## 1. Encryption

### Vault Encryption
- **Algorithm:** AES-256-GCM (authenticated encryption)
- **Key Derivation:** Argon2id (m=65536, t=3, p=1, output=32 bytes)
- **Key:** Argon2id(password, per-vault random salt) — NOT SHA-256(password)
- **Nonce:** Random 96-bit nonce generated per encryption
- **Header Magic:** `OMNI` (4 bytes) for format identification

### Recovery Data Encryption
- **Algorithm:** AES-256-GCM
- **Key:** SHA-256(security_answer_lowercased_trimmed) — used to encrypt/decrypt the master password
- **Storage:** Separate file (`vault.recovery`), not inside the vault

### Password Verification
- **Algorithm:** Argon2id (same params as encryption)
- **Storage:** `VaultConfig.password_hash` (32 bytes)
- **Salt:** `VaultConfig.password_salt` (16 bytes)

### TOTP
- **Algorithm:** RFC 6238 (TOTP)
- **Hash:** SHA-1
- **Digits:** 6
- **Period:** 30 seconds
- **Tolerance:** 1 time-step (past and future)

---

## 2. System Protection

### File/Folder Locking
- **Method:** Windows NT DACL via `icacls.exe`
- **ACE:** `Everyone:(OI)(CI)F` denied (no read/write/execute/delete)
- **Recursion:** `/T` flag applies to subdirectories at the ACL level

### Drive Locking
- **DACL:** Same as file/folder locking on drive root
- **Visibility:** `NoDrives` registry value hides drive in Windows Explorer
- **Bitmask:** Standard Windows drive bitmask (bit 0 = A:, bit 1 = B:, etc.)

### System Presets
| Preset | Registry Key | Value |
|:---|:---|:---|
| Task Manager | `HKCU\...\Policies\System` | `DisableTaskMgr=1` |
| Control Panel | `HKCU\...\Policies\Explorer` | `NoControlPanel=1` |
| Registry Editor | `HKCU\...\Policies\System` | `DisableRegistryTools=1` |
| PowerShell | `HKCU\...\Policies\Microsoft\Windows\PowerShell` | `EnableScripts=0` |
| CMD | `HKCU\...\Policies\Microsoft\Windows\System` | `DisableCMD=1` |
| System Restore | `HKLM\...\Policies\Microsoft\Windows NT\SystemRestore` | `DisableSR=1` |

---

## 3. Password Reset (2FA Wipe on Reset)

1. User answers security question
2. Answer hash used as key to decrypt `vault.recovery` → recovers old master password
3. Old master password decrypts `vault.enc` → full vault config loaded
4. Vault is re-encrypted with new Argon2id(password, new_salt) key
5. 2FA is **disabled** (totp_enabled = false, totp_secret cleared)
6. Recovery data is re-encrypted with same answer hash but new password
7. User must log in with new password — no 2FA until re-enabled

---

## 4. Panic Lock (Win+Alt+L)

1. Hotkey triggers `PANIC_ACTIVE` atomic flag in Rust backend
2. Frontend detects flag and blanks the screen content
3. Session vault config is cleared from memory
4. User must re-authenticate to restore access

---

## 5. Data Locations

| Data | Path | Encrypted? |
|:---|:---|:---|
| Vault | `%APPDATA%\InnologyBD\OmniLock\vault.enc` | Yes (AES-256-GCM) |
| Recovery | `%APPDATA%\InnologyBD\OmniLock\vault.recovery` | Yes (AES-256-GCM) |
| Meta | `%APPDATA%\InnologyBD\OmniLock\vault.meta` | No (plaintext) |

---

*End of SECURITY.md (OmniLock v2.0.0)*
