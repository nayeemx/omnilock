# OmniLock - Windows 11 App, Folder & File Locker
## System Architecture & Technical Specification

**Document Version:** 2.2.0
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
               |  │  file_locker.rs   ← AES-256-GCM  │  |
                |  │  drive_locker.rs   ← NoDrives+Enc │  |
                |  │  vault_storage.rs ← Vault Storage │  |
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
- **`VaultConfig.file_encryption_key` (v0.0.35):** 32 random bytes used to encrypt locked files/folders. Generated once at vault setup, independent of the master password, auto-migrated for older vaults on login. Held in `AppState.file_key` + global `ACTIVE_FILE_KEY` while a session is active. Also the key for vault storage blobs/manifest (v0.0.36).
- **Vault storage (v0.0.36):** encrypted blobs in `%APPDATA%\InnologyBD\OmniLock\storage\` + encrypted manifest `vault_files.enc` (see §2.5).

### 2.5 File / Folder Encryption Locking (v0.0.35 — replaced ACL locking)
- **Method:** AES-256-GCM authenticated encryption via the `aes-gcm` crate. No NTFS ACL modification.
- **Lock file:** reads the file, encrypts with a fresh 96-bit nonce, writes `original.omnilock`, deletes the original.
- **Blob layout:** `[4] "OLCK" | [1] version=1 | [12] nonce | [4] path_len (u32 LE) | [N] original path (UTF-8) | [*] ciphertext + 16-byte tag`.
- **Folder lock:** recursive encrypt of every file inside; the folder itself stays browsable (`desktop.ini` and existing `.omnilock` blobs are skipped).
- **Unlock:** recursive decrypt (blob → original path from the embedded header, blob deleted).
- **Key flow:** `cmd_unlock_session` / `cmd_biometric_login` / `cmd_widget_unlock` load `file_encryption_key` into `AppState.file_key` + `ACTIVE_FILE_KEY`. All lock/unlock commands require it.
- **Widget auto-relock:** `cmd_widget_unlock` temporarily lifts the lock without touching the vault; on widget focus-lost the target is re-encrypted via `WIDGET_TEMP_UNLOCKED` + `ACTIVE_FILE_KEY`.
- **`.omnilock` file association:** `shell_context.rs` registers the extension → `OmniLock.exe --open-locked "%1"`; the setup hook stores the path and the unlock widget appears after login.
- **Legacy ACL damage:** `scan_acl_damage` (recursive FindFirstFile walk, owner==SYSTEM detection), `bulk_recover_acl`, `force_unlock`, `safe_recover_acl` are kept for files damaged by the pre-0.0.35 lock. Recovery commands also purge the service's persisted re-lock list.
- **Drive locking (v0.0.35):** `drive_locker.rs` sets the NoDrives registry value only (hide). Whole-drive recursive encryption was removed as a data hazard; `lock_folder` refuses drive roots. True access blocking is an open design question — see AGENTS.md.

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

### 2.5 Vault Storage — Private Encrypted File Storage (v0.0.36)
- **Purpose:** store user files privately inside the vault (backlog user requirement #2) — files are encrypted with the vault's `file_encryption_key` and kept under `%APPDATA%\InnologyBD\OmniLock\storage\`.
- **Blob file:** `<16-random-hex>.vaultfile`, layout `[4] "OVLF" | [1] version=1 | [12] nonce | [4] name_len (u32 LE) | [N] original name (UTF-8) | [8] size (u64 LE) | [*] ciphertext + 16-byte tag`.
- **Manifest:** `vault_files.enc` — itself AES-GCM encrypted (`[4] "OVMF" | [12] nonce | [*] ciphertext JSON`). Plaintext JSON = `[{ "id", "name", "size", "added_at" }]`. Stored file names never leak on disk.
- **Operations:** `store_file` (encrypt → write blob → **delete original only after** the blob is written), `extract_file` (decrypt to user-chosen folder; **refuses to overwrite** existing files; verifies size; removes blob + manifest entry after a successful write), `delete_file` (removes blob + manifest entry).
- **Guards:** reuses `file_locker::do_encrypt`/`do_decrypt`/`check_protected_path` (made `pub`); refuses symlinks and protected paths.
- **Commands:** `cmd_vault_store_file`, `cmd_vault_list_files`, `cmd_vault_extract_file`, `cmd_vault_delete_file` — all require a session + the file key.

### 2.6 Stale-Unlock Reconciliation After Restart (v0.0.36)
- `cmd_verify_locked_state` (post-login): checks every `locked_files`/`locked_folders` entry against disk (`is_file_locked`/`is_folder_locked` — is the `.omnilock` blob still there?). Entries whose blob is gone were temp-unlocked (widget) and never re-locked because the app exited.
- Frontend prompt (App.tsx, full-screen overlay after unlock): **Re-lock all** → `cmd_relock_entries` (re-encrypts each path via `lock_folder`/`lock_file` by `is_dir()`, saves vault + `update_locked_folders` + summary; per-item status `ok`/`already_locked`/error; failures stay listed). **Keep unlocked** → `cmd_forget_unlocked_entries` (removes entries from both `locked_files` and `locked_folders`, `notify_folder_locked`, saves vault + summary). No silent re-encryption.

### 2.7 Legacy Service ACL State Purge (v0.0.36)
- The service (`service/src/bin/svc.rs`) persists `locked_items` to ProgramData and re-applies owner=SYSTEM ACL locks at boot — legacy behavior incompatible with encryption locks.
- On app startup the setup hook calls `service_client::get_locked_items()` and purges each via `notify_force_remove_locked_item`. Runs once per launch; empty afterwards → no-op.

### 2.8 Self-Monitoring Watchdog
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
| `cmd_add_locked_drive` | `drive_letter` | `Result<(), String>` | Lock drive (NoDrives hide only) |
| `cmd_remove_locked_drive` | `drive_letter` | `Result<(), String>` | Unlock drive |
| `cmd_add_locked_file` | `path` | `Result<String, String>` | Lock file (AES-256-GCM encrypt → `.omnilock`) |
| `cmd_remove_locked_file` | `path` | `Result<String, String>` | Unlock file (decrypt in place) |
| `cmd_add_locked_folder` | `path` | `Result<String, String>` | Lock folder (recursive encrypt) |
| `cmd_remove_locked_folder` | `path` | `Result<String, String>` | Unlock folder (recursive decrypt) |
| `cmd_scan_acl_damage` | `path` | `Result<Vec<String>, String>` | Find all items owned by SYSTEM (old-format lock damage) |
| `cmd_bulk_recover_acl` | `paths` | `Vec<(String, String)>` | `force_unlock` each path; purges service re-lock list |
| `cmd_force_unlock` | `path` | `Result<String, String>` | Take ownership + reset DACL (stubborn files) |
| `cmd_rescue_unlock` | `path` | `Result<String, String>` | Rescue mode — needs session + file key |
| `cmd_widget_unlock` | `password` | `Result<(), String>` | Temp-unlock target; auto re-locks on widget close |
| `cmd_authenticate_biometric` | `message` | `Result<bool, String>` | Windows Hello fingerprint verification (`UserConsentVerifier` via PowerShell 5.1) — direct WBF is dead on this hardware (driver ignores background-app captures) |
| `cmd_check_biometric` | None | `BiometricStatus` | `UserConsentVerifier.CheckAvailabilityAsync()` (WinRT) — registry checks unreliable on Win11 (keys absent) |
| `cmd_verify_locked_state` | None | `Vec<UnlockTarget>` | Stale entries: vault says locked, disk has no `.omnilock` blob |
| `cmd_relock_entries` | `paths` | `Vec<(String, String)>` | Re-encrypt stale entries; per-item `ok`/`already_locked`/error |
| `cmd_forget_unlocked_entries` | `paths` | `Result<(), String>` | Remove stale entries from the vault (keep unlocked) |
| `cmd_vault_store_file` | `path` | `Result<VaultFileInfo, String>` | Encrypt file into vault storage, delete original |
| `cmd_vault_list_files` | None | `Vec<VaultFileInfo>` | List stored files (from encrypted manifest) |
| `cmd_vault_extract_file` | `id, dest_dir` | `Result<(), String>` | Decrypt stored file to a folder (no overwrite) |
| `cmd_vault_delete_file` | `id` | `Result<(), String>` | Delete stored blob + manifest entry |
| `cmd_install_context_menu` / `cmd_uninstall_context_menu` | None | `Result<String, String>` | Context menu + `.omnilock` association |
| `cmd_toggle_locked_app` | `name, enabled` | `Result<(), String>` | Toggle app in config |
| `cmd_add_locked_app` | `name, path, sha256` | `Result<(), String>` | Add app to lock list |
| `cmd_remove_locked_app` | `name` | `Result<(), String>` | Remove app from lock list |
| `cmd_generate_totp` | None | `Result<String, String>` | Generate base32 TOTP secret |
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
| `vault_files.enc` + `storage\*.vaultfile` | `%APPDATA%\InnologyBD\OmniLock\` | ✅ Yes (vault storage) |

MSI/NSIS installers only overwrite files in the installation directory (Program Files). AppData is untouched.

---

## 6. Security Model

### Encryption Layers
1. **Password verification:** Argon2id (m=65536, t=3, p=1, 32-byte output) — stored as hash
2. **Vault encryption key:** Argon2id(password, config password_salt) → 32 bytes → AES-256-GCM key
3. **Recovery data key:** SHA-256(security_answer_lowercased_trimmed) → AES-GCM key
4. **Nonce:** Random 96-bit, per encryption, stored in vault header
5. **File/folder lock key (v0.0.35):** `VaultConfig.file_encryption_key` — 32 random bytes, stored inside the encrypted vault, never rotated on password change. Locked files are AES-256-GCM encrypted with a fresh 96-bit nonce per file. If the vault is lost, locked files are unrecoverable (backup the vault or export config).
6. **Vault storage (v0.0.36):** stored files encrypted with the same `file_encryption_key` (fresh nonce per file); the manifest is separately AES-GCM encrypted so names never leak. Same recovery trade-off as locked files.

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

*End of ARCHITECTURE.md (OmniLock v2.2.0)*
