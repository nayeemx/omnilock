# OmniLock - Windows 11 App, Folder & File Locker
## System Architecture & Technical Specification

**Document Version:** 2.0.0
**Product Name:** OmniLock
**Developer / Publisher:** InnologyBD
**Target Platform:** Windows 10/11 x64
**Primary Stack:** Rust (Backend Guard & Daemon), Tauri v2 (IPC & Native Shell), React 18 / TypeScript (Frontend Dashboard)

---

## 1. Architectural Overview & System Topology

OmniLock uses a **single-process Tauri architecture** with a dual-threaded Rust backend. There is no separate guard binary — the watchdog thread monitors the main process from within.

```
+-----------------------------------------------------------------------------------+
|                                  WINDOWS 11 OS                                    |
+-----------------------------------------------------------------------------------+
                                           |
                                           v
               +----------------------------------------------+
               |  omnilock.exe (Tauri App Process)        |
               |                                              |
               |  ┌──────────────────────────────────────┐  |
               |  │  React 18 UI (Glassmorphism Dark)   │  |
               |  │  Sidebar │ TopBar │ 4 Page Tabs     │  |
               |  │  (split into 18 focused files)     │  |
               |  └────────────┬─────────────────────────┘  |
               |               │  Tauri IPC Bridge          │
               |               ▼                            │
               |  ┌──────────────────────────────────────┐  |
               |  │  Rust Backend (src-tauri/src/)      │  |
               |  │                                    │  |
               |  │  auth.rs       ← Setup + Unlock    │  |
               |  │  vault.rs      ← Encrypt + Recover │  |
               |  │  totp.rs       ← TOTP + QR         │  |
               |  │  process_guard.rs ← Enumerate+Stop │  |
               |  │  system_presets.rs    ← All 6      │  |
               |  │  installer_guard.rs   ← Kill MSI   │  |
               |  │  file_locker.rs      ← icacls ACL  │  |
               |  │  drive_locker.rs     ← ACL+Reg     │  |
               |  │  panic_hotkey.rs     ← Win+Alt+L   │  |
               |  │  watchdog.rs          ← Self-monitor│ │
               |  │  models.rs          ← All DTOs     │  |
               |  │  lib.rs             ← All commands │  |
               |  └──────────────────────────────────────┘  |
               +----------------------------------------------+
                                           |
                                           v
                   +------------------------------------------+
                   |  Vault Data Files (%APPDATA%)          |
                   |  InnologyBD\OmniLock\                  |
                   |    vault.enc    (Argon2id+AES-256-GCM)  |
                   |    vault.meta    (totp + vault_version) |
                   |    vault.recovery (answer-encrypted pw) |
                   +------------------------------------------+
```

---

## 2. Component Subsystems Architecture

### 2.1 Core Vault
- **Encrypted file:** `%APPDATA%\InnologyBD\OmniLock\vault.enc`
- **Recovery data:** `%APPDATA%\InnologyBD\OmniLock\vault.recovery` (separate file, answer-encrypted)
- **Meta info:** `%APPDATA%\InnologyBD\OmniLock\vault.meta` (TOTP status + vault version)
- **Encryption:** Argon2id KDF + AES-256-GCM authenticated encryption
- **Key derivation:** `Argon2id(password, salt)` where salt is per-vault random, stored in vault header
- **Header Magic:** `OMNI` (4 bytes)
- **Version:** `u32` in outer `EncryptedVault` struct, currently `1`

### 2.2 Application Interception
- Real-time Win32 process enumeration via `sysinfo` crate (`System::new_all()` + `refresh_processes()`)
- Triple Indexing per process: Binary Path + Process Name + SHA-256 Hash per binary
- Process suspension via `OpenProcess(PROCESS_SUSPEND_RESUME)` + `SuspendThread` Win32 API
- Polling interval: 500ms
- SHA-256 mismatch = skip (binary was renamed/modified since lock was applied)

### 2.3 System Presets & Installer Guard
Blocks the following via registry keys and process killing:
| Target | Method | Registry Key |
|:---|:---|:---|
| Task Manager | Kill process + registry | `HKCU\...\Policies\System\DisableTaskMgr=1` |
| Control Panel | Registry only | `HKCU\...\Policies\Explorer\NoControlPanel=1` |
| Registry Editor | Kill process + registry | `HKCU\...\Policies\System\DisableRegistryTools=1` |
| PowerShell | Kill process + registry | `HKCU\...\Policies\Microsoft\Windows\PowerShell\EnableScripts=0` |
| CMD | Kill process + registry | `HKCU\...\Policies\Microsoft\Windows\System\DisableCMD=1` |
| System Restore | Kill process + registry | `HKLM\...\Policies\Microsoft\Windows NT\SystemRestore\DisableSR=1` |
| Installers (MSI/setup) | Polling + kill | `msiexec.exe`, `setup.exe`, `install.exe`, etc. via `TerminateProcess` |

### 2.4 Panic Hotkey Engine
- **Hotkey:** Win + Alt + L
- **Implementation:** `RegisterHotKey` via raw FFI: `GetModuleHandleA("user32.dll")` → `GetProcAddress("RegisterHotKey")` → call via `std::mem::transmute`
- **Message loop:** Separate thread calls `GetMessageW`/`TranslateMessage`/`DispatchMessageW` in loop
- **Trigger:** On `WM_HOTKEY` (0x0312), sets `PANIC_ACTIVE` atomic flag
- **Frontend response:** Clears vault config from memory, locks screen

### 2.5 Self-Monitoring Watchdog
- Single thread polls every 500ms using `sysinfo`
- Checks if `omnilock.exe` is still running
- If main process died (unexpected crash/restart), attempts to restart itself from same directory
- No external `omnilock-guard.exe` binary needed

---

## 3. Tauri IPC Command Reference

All commands return `Result<T, String>` with human-readable error messages. Every command has user-visible feedback.

| Command | Args | Returns | Description |
|:---|:---|:---|:---|
| `cmd_get_vault_status` | None | `VaultStatusDto` | Vault init state, TOTP flag, publisher, version |
| `cmd_get_vault_config` | None | `Result<VaultConfigDto, String>` | Full config for unlocked session |
| `cmd_setup_vault` | `SetupPayload` | `Result<(), String>` | Create vault + save recovery data |
| `cmd_unlock_session` | `AuthPayload` | `Result<SessionToken, String>` | Password + optional TOTP auth + migration |
| `cmd_toggle_system_preset` | `preset_id, enabled` | `Result<(), String>` | Toggle system preset |
| `cmd_toggle_installer_guard` | `enabled` | `Result<(), String>` | Toggle installer blocking |
| `cmd_trigger_panic_lock` | None | `Result<(), String>` | Register panic hotkey |
| `cmd_add_locked_drive` | `drive_letter` | `Result<(), String>` | Lock drive (DACL + NoDrives) |
| `cmd_remove_locked_drive` | `drive_letter` | `Result<(), String>` | Unlock drive |
| `cmd_add_locked_file` | `path` | `Result<(), String>` | Lock file (icacls deny Everyone) |
| `cmd_remove_locked_file` | `path` | `Result<(), String>` | Unlock file |
| `cmd_add_locked_folder` | `path` | `Result<(), String>` | Lock folder (icacls deny) |
| `cmd_remove_locked_folder` | `path` | `Result<(), String>` | Unlock folder |
| `cmd_toggle_locked_app` | `name, enabled` | `Result<(), String>` | Toggle app in config |
| `cmd_add_locked_app` | `name, path, sha256` | `Result<(), String>` | Add app to lock list |
| `cmd_remove_locked_app` | `name` | `Result<(), String>` | Remove app from lock list |
| `cmd_generate_totp` | None | `Result<String, String>` | Generate base64 TOTP secret |
| `cmd_generate_totp_qr` | `secret` | `Result<String, String>` | Generate QR as data URI |
| `cmd_verify_totp` | `secret, code` | `Result<bool, String>` | Verify TOTP code |
| `cmd_enable_2fa` | `secret, code` | `Result<(), String>` | Enable TOTP in vault |
| `cmd_disable_2fa` | None | `Result<(), String>` | Disable TOTP in vault |
| `cmd_list_drives` | None | `Vec<String>` | Available drive letters A-Z |
| `cmd_list_processes` | None | `Vec<(String, String, String)>` | Process scan (name, path, sha256) |
| `cmd_set_auto_lock` | `minutes` | `Result<(), String>` | Set auto-lock timer |
| `cmd_get_security_question` | None | `Result<String, String>` | Get stored recovery question |
| `cmd_reset_password` | `new_password, answer` | `Result<(), String>` | Full password reset via security Q&A |

---

## 4. Frontend Component Structure

All components are under `src/components/`:

| Path | Lines | Purpose |
|:---|:---|:---|
| `types.ts` | ~27 | Shared types, constants (tabs, securityQuestions, presetMeta) |
| `auth/LoginScreen.tsx` | ~258 | Login form + password reset/recovery flow |
| `auth/SetupWizard.tsx` | ~182 | 3-step first-launch wizard |
| `layout/Sidebar.tsx` | ~51 | Navigation sidebar |
| `layout/TopBar.tsx` | ~32 | Header bar with status badges |
| `layout/Footer.tsx` | ~16 | Branding footer |
| `pages/AppLockerPage.tsx` | ~169 | Process scan/lock/toggle/remove |
| `pages/PresetsPage.tsx` | ~93 | System preset toggles + installer guard |
| `pages/VaultPage.tsx` | ~196 | Drive volumes + file/folder management |
| `pages/SecurityPage.tsx` | ~231 | 2FA config, auto-lock, recovery tiers |
| `shared/Field.tsx` | ~12 | Form field with label + icon |
| `shared/Toggle.tsx` | ~9 | Controlled toggle switch |
| `shared/SectionHeader.tsx` | ~9 | Page section header |
| `shared/Stat.tsx` | ~18 | Metric card |
| `shared/StatusPill.tsx` | ~12 | Protected/Unlocked badge |
| `App.tsx` | ~85 | Root routing only |

---

## 5. Versioning & Migration

### How Versioning Works
The `EncryptedVault` struct includes:
```rust
pub version: u32  // Current: 1
pub salt: Vec<u8>  // Per-vault random salt (16 bytes)
```
Additionally, `vault.meta` stores `vault_version` alongside `totp_enabled`.

### Migration Process
On every successful unlock (`cmd_unlock_session`), the backend calls:
1. Read `vault.enc` → parse `EncryptedVault`
2. Check `encrypted.version` against `CURRENT_VAULT_VERSION` (constant = 1)
3. If `version < CURRENT_VAULT_VERSION`: run migration transformations (e.g., clear deprecated fields, re-encrypt with new params)
4. Re-encrypt vault with current `encrypt_vault()` using new format
5. Update `vault.meta` with new `vault_version`
6. If `version >= CURRENT_VAULT_VERSION`: no action needed

### Version Increment Policy
| Change Type | Bump | Example |
|:---|:---|:---|
| Encryption params change (Argon2id m/t/p, key size) | MAJOR | v1.x → v2.0.0 |
| New compatible fields added to VaultConfig | MINOR | v1.0.x |
| Bug fixes, no format change | PATCH | v1.x.x |
| Recovery data format change | MAJOR | v1.x → v2.0.0 |

### Data Survival Across Reinstall
| File | Path | Survives MSI/NSIS Install? |
|:---|:---|:---|
| `vault.enc` | `%APPDATA%\InnologyBD\OmniLock\vault.enc` | ✅ Yes |
| `vault.meta` | `%APPDATA%\InnologyBD\OmniLock\vault.meta` | ✅ Yes |
| `vault.recovery` | `%APPDATA%\InnologyBD\OmniLock\vault.recovery` | ✅ Yes |

MSI/NSIS installers only overwrite files in the installation directory (Program Files). AppData is untouched.

---

## 6. Security Model

### Encryption Layers
1. **Password verification:** Argon2id (m=65536, t=3, p=1, 32-byte output) — stored as hash
2. **Vault encryption key:** Argon2id(password, config password_salt) → 32 bytes → AES-256-GCM key
3. **Recovery data key:** SHA-256(security_answer_lowercased_trimmed) → AES-GCM key
4. **Nonce:** Random 96-bit, per encryption, stored in vault header

### Attack Surface Mitigations
| Attack | Mitigation |
|:---|:---|
| Rename locked binary and re-run | SHA-256 hash verification per locked app |
| Copy encrypted vault to another machine | Vault decryption requires correct password (Argon2id derivation) |
| Modify encrypted vault | AES-GCM authentication tag rejects tampered ciphertext |
| Brute-force password | Argon2id memory-hard KDF (64MB working set, 3 passes) |
| Recovery answer brute-force | Answer stored as SHA-256 hash (constant-time compare) |
| Installer elevation bypass | Installer guard kills known installer processes by name |
| System tool disruption | 6 presets block via registry + process kill |
| Session theft after panic | Panic lock clears vault config from memory immediately |
| 2FA bypass after password reset | Password reset always disables 2FA (user must re-enable) |

---

*End of ARCHITECTURE.md (OmniLock v2.0.0)*
