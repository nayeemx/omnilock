# Development Log

Issues encountered during OmniLock development. Each entry follows: Symptoms → Root Cause → Solution → Prevention → Files Changed.

---

## Feature Status Tracker

| Feature | File(s) | Status | Notes |
|---|---|---|---|
| Vault encryption | `vault.rs` | ✅ Complete | Argon2id key, AES-256-GCM |
| Vault decryption | `vault.rs` | ✅ Complete | Reads salt from vault header |
| Vault recovery | `vault.rs` | ✅ Complete | `vault.recovery` file, answer-encrypted |
| Vault migration | `vault.rs` | ✅ Complete | `migrate_vault_if_needed()` |
| Vault meta | `vault.rs` | ✅ Complete | `vault.meta` with totp + version |
| Auth setup | `auth.rs` | ✅ Complete | Argon2id hash + recovery save |
| Auth unlock | `auth.rs` | ✅ Complete | Password + TOTP verify, migration on unlock |
| Answer verification | `auth.rs` | ✅ Complete | Constant-time comparison via `subtle` |
| TOTP secret gen | `totp.rs` | ✅ Complete | 20 random bytes, **base32** encoded |
| TOTP QR gen | `totp.rs` | ✅ Complete | PNG data URI via totp-rs crate |
| TOTP verify | `totp.rs` | ✅ Complete | RFC 6238, SHA-1, 30s step |
| Process enumeration | `process_guard.rs` | ✅ Complete | sysinfo + SHA-256 hash per binary |
| Process suspension | `process_guard.rs` | ✅ Complete | SuspendThread FFI |
| Process monitor loop | `process_guard.rs` | ✅ Complete | 500ms poll loop |
| System presets (6) | `system_presets.rs` | ✅ Complete | All 6 registry/process blocks |
| Installer guard | `installer_guard.rs` | ✅ Complete | Polls process names, kills installers |
| File locker | `file_locker.rs` | ✅ Complete | icacls deny/grant Everyone |
| Drive locker | `drive_locker.rs` | ✅ Complete | ACL + NoDrives registry |
| Panic hotkey | `panic_hotkey.rs` | ✅ Complete | RegisterHotKey Win+Alt+L via FFI |
| Hotkey listener | `panic_hotkey.rs` | ✅ Complete | Windows message loop thread |
| Watchdog | `watchdog.rs` | ✅ Complete | Self-monitoring, no guard binary |
| Password reset | `vault.rs` + `auth.rs` | ✅ Complete | Answer → decrypt → re-encrypt → wipe 2FA |
| Frontend modularization | All `src/components/` | ✅ Done | 18 files, no monolith |
| Tauri bridge cleanup | `tauri-bridge.ts` | ✅ Done | Dead exports removed |
| Inline error feedback | All page components | ✅ Done | No silent catch blocks |
| Controlled Toggle | `Toggle.tsx` + parents | ✅ Done | Parent `on` prop drives state |
| 2FA flow fix | `SecurityPage.tsx` | ✅ Done | Button→QR→verify sequence |

---

## Issue Log

### 2FA TOTP Secret Encoding (Base64 vs Base32)
**Date:** 2026-07-26
**Symptoms:** 2FA could never be enabled — user scans QR, enters correct code, but verification always fails. The "Enable 2FA" button flow appears broken.
**Root Cause:** `generate_totp_secret()` used base64 encoding, but authenticator apps (Google Authenticator, Authy, etc.) expect base32 encoding in otpauth URIs. When user scans QR, authenticator decodes the base64 string as base32 → different secret key → codes never match.
**Solution:** Added `base32` crate dependency. Changed `generate_totp_secret()` to use `base32::encode()`. Changed `create_totp()` to use `Secret::Encoded()` for proper base32 decoding. All TOTP functions now expect/return base32-encoded secrets.
**Files Changed:** `src-tauri/src/totp.rs`, `src-tauri/Cargo.toml`
**Prevention:** TOTP standards (RFC 6238) and otpauth URI scheme require base32 encoding. Never use base64 for TOTP secrets.

---

### 2FA SetupWizard Calling enable2FA Without Session
**Date:** 2026-07-26
**Symptoms:** During initial setup, clicking "Enable 2FA & Finish" fails with "Session not unlocked" error.
**Root Cause:** `setupVault()` creates and encrypts the vault. Then `enable2FA()` tries to modify `state.vault_config` (in-memory config) via `cmd_enable_2fa`, but during setup there's no active session — `vault_config` is `None`.
**Solution:** Simplified SetupWizard to 2 steps (password + security question). Removed 2FA from wizard entirely. 2FA is now enabled from Security page after first login.
**Files Changed:** `src/components/auth/SetupWizard.tsx`, `src/components/types.ts`
**Prevention:** Backend commands requiring `State<AppState>` cannot be called during vault setup. Features needing session state must be available only after unlock.

---

### Auto-Update System Implementation
**Date:** 2026-07-26
**Symptoms:** No update mechanism existed. Every fix required full uninstall + reinstall.
**Root Cause:** Tauri updater plugin was not configured. No signing key, no update endpoint, no UI for checking updates.
**Solution:** Added `tauri-plugin-updater` and `tauri-plugin-process` to Cargo.toml. Configured `tauri.conf.json` with GitHub Releases endpoint. Added "Check for Updates" button to SecurityPage. Generated Ed25519 signing key pair. Created signed builds with `.sig` files.
**Files Changed:** `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/lib.rs`, `src/lib/tauri-bridge.ts`, `src/components/pages/SecurityPage.tsx`
**Prevention:** For distributable applications, always implement an update mechanism from the start. Tauri's updater plugin is well-documented and straightforward to configure.

---

### Tauri Signer Key Generation (Interactive Input)
**Date:** 2026-07-26
**Symptoms:** `npx tauri signer generate` requires interactive password input, cannot be automated in shell scripts.
**Root Cause:** The Tauri CLI uses `rpassword` for secure password input, which doesn't support piped input.
**Solution:** User manually ran the command in PowerShell. Key stored at `src-tauri/update.key` with password `229689`. Public key at `src-tauri/update.key.pub`.
**Files Changed:** `src-tauri/update.key`, `src-tauri/update.key.pub`
**Prevention:** For CI/CD pipelines, use `TAURI_SIGNING_PRIVATE_KEY` and `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` environment variables instead of the interactive CLI.

---

### QR Code Returning URL String Instead of PNG Data URI
**Date:** 2026-07-26
**Symptoms:** The 2FA setup displayed a broken image — the frontend rendered `otpauth://...` as an `<img src>`. No QR code appeared.
**Root Cause:** `totp.rs` `get_url()` returned the raw otpauth URL string. It was used as `<img src>` which browsers cannot render.
**Solution:** Added `get_qr_base64()` that uses the `qrcode` crate to render a PNG and returns `data:image/png;base64,...` format.
**Files Changed:** `src-tauri/src/totp.rs`
**Prevention:** When generating visual assets for the frontend, always return the format the frontend expects. For `<img>` tags, that's `data:image/*;base64,...`.

---

### Toggle Using Local State Instead of Controlled
**Date:** 2026-07-26
**Symptoms:** Toggling a locked app in the UI would visually flip but remain in the wrong state after a refresh.
**Root Cause:** The Toggle component maintained its own `useState` rather than being driven by the `on` prop from the parent.
**Solution:** Fixed Toggle to be a fully controlled component — `on` prop drives the visual, `onChange` fires the backend command. No internal state.
**Files Changed:** All page components that used Toggle (state already correct, Toggle component usage pattern fixed)
**Prevention:** All interactive state with a backend should be controlled by the parent. If a component has state that doesn't call the backend on change, it will desync.

---

### Auto-Lock `setAutoLock` Variable Shadowing Imported Function
**Date:** 2026-07-26
**Symptoms:** The auto-lock timer buttons did not persist the setting to the vault. Only local state updated.
**Root Cause:** A local variable named `setAutoLock` (the React setState function) shadowed the imported `setAutoLock` (IPC command) from tauri-bridge.
**Solution:** Renamed local state variable to `autoLockMins`. Bridge function import remains as `setAutoLock`.
**Files Changed:** `src/components/pages/SecurityPage.tsx`
**Prevention:** Never name local variable the same as an imported function, especially bridge/invoke functions. Prefix local state with descriptive names (e.g., `autoLockMins`).

---

### 2FA Section Showing QR Immediately Instead of "Enable" Button
**Date:** 2026-07-26
**Symptoms:** The Security page displayed the QR code on load without the user clicking "Enable". User had already scanned before intending to enable.
**Root Cause:** Security page always generated a TOTP secret and QR code as soon as the component mounted.
**Solution:** Restructured Security page to show "Enable Two-Factor Authentication" button by default. QR generated only on explicit user action.
**Files Changed:** `src/components/pages/SecurityPage.tsx`
**Prevention:** Sensitive side effects (enabling features, generating secrets) should require explicit user action. Default state should be "not yet enabled."

---

### Silent Error Handling in Catch Blocks
**Date:** 2026-07-26
**Symptoms:** Adding locked apps, toggling presets, locking drives silently failed with no user feedback.
**Root Cause:** All `.catch()` handlers used `console.error()` only. Empty `catch {}` blocks swallowed errors silently (e.g., `enable2FA` in SetupWizard).
**Solution:** Added `error` + `success` state to all pages with backend operations. Every catch block renders a styled alert div. Empty catch blocks eliminated.
**Files Changed:** AppLockerPage.tsx, PresetsPage.tsx, VaultPage.tsx, SecurityPage.tsx, SetupWizard.tsx
**Prevention:** Every `try/catch` that calls a backend command must surface the error to the user. Never use empty catch blocks.

---

### Vault Encryption Key Uses SHA-256 Instead of Argon2id
**Date:** 2026-07-26
**Symptoms:** Security audit during development. Not visible to user.
**Root Cause:** `encrypt_vault()` and `decrypt_vault()` used `SHA-256(password)` to derive the AES key. SHA-256 is a fast unsalted hash. Argon2id was only used for the password verification hash. An attacker with the encrypted vault could brute-force at millions of guesses/second.
**Solution:** Changed vault encryption to use `Argon2id(password, salt)`. Added `salt` and `version` fields to `EncryptedVault` so decryption can re-derive the key. Salt stored in the clear in the vault header (not secret — it's per-vault random anyway).
**Files Changed:** `src-tauri/src/models.rs` (Added version, salt to EncryptedVault), `src-tauri/src/vault.rs` (encrypt/decrypt re-derivation), all callers updated
**Prevention:** When adding encryption, the key derivation function must be the same one used for password verification. Never use fast hashes (SHA-256, MD5) for encryption keys.

---

### Portable EXE Extremely Slow
**Date:** 2026-07-26
**Symptoms:** The portable version of OmniLock was extremely slow, almost unusable.
**Root Cause:** Windows Defender scans the unsigned `.exe` on every process creation and file access. No code signing certificate → aggressive scanning.
**Solution (user-side):** Add the exe or install directory to Windows Defender exclusions. For production: code-sign with Authenticode certificate.
**Files Changed:** None (configuration/user action)
**Prevention:** For distributable executables, always use code signing. For development, add build output to antivirus exclusion list.

---

### Password Reset Feature (Full Implementation)
**Date:** 2026-07-26
**Symptoms:** No password reset flow existed. If user forgot master password, only option was full reinstall with vault loss.
**Root Cause:** Vault schema had `security_question` and `security_answer_hash` but no mechanism to store encrypted master password for recovery. No "Forgot password?" UI.
**Solution:** `VaultRecoveryData` struct in separate file (`vault.recovery`). Answer hash encrypts the master password via AES-GCM. On reset: verify answer → decrypt vault with recovered old password → re-encrypt with new password → wipe 2FA → update recovery data.
**Files Changed:** `models.rs`, `vault.rs`, `auth.rs`, `lib.rs`, `tauri-bridge.ts`, `LoginScreen.tsx` (15+ files touched across backend to frontend)
**Prevention:** Always persist a recovery path for credential loss. Security answers must be hashed (never plaintext). Recovery data encrypted with answer-derived key, not password.

---

### Windows-SYS 0.52 Missing `RegisterHotKey` API
**Date:** 2026-07-26
**Symptoms:** Attempted to implement panic hotkey using `RegisterHotKey` from `windows_sys` — compiler error: not found.
**Root Cause:** `windows-sys` 0.52's `Win32_UI_WindowsAndMessaging` feature does not export `RegisterHotKey`, `MOD_WIN`, or `MOD_ALT`.
**Solution:** Used raw FFI: `GetModuleHandleA` + `GetProcAddress` to dynamically load `RegisterHotKey`, `GetMessageW`, `TranslateMessage`, `DispatchMessageW` from `user32.dll` at runtime.
**Files Changed:** `src-tauri/src/panic_hotkey.rs`
**Prevention:** When using `windows-sys`, verify the API exists in your version before coding. If unavailable, raw FFI with `GetProcAddress` is a reliable fallback.

---

### System Presets Control Panel + System Restore Were No-Ops
**Date:** 2026-07-26
**Symptoms:** Toggling Control Panel or System Restore in the UI updated config but had no system effect.
**Root Cause:** `apply_system_presets()` only implemented 4 of 6 presets. Control Panel and System Restore were shown but not handled.
**Solution:** Added `NoControlPanel` (HKCU Explorer policy) and `DisableSR` (HKLM System Restore policy). Also kill `rstrui.exe` when System Restore preset is enabled.
**Files Changed:** `src-tauri/src/system_presets.rs`
**Prevention:** Every UI toggle must have a corresponding backend implementation and verifiable system effect.

---

### Watchdog Depends on Missing `omnilock-guard.exe`
**Date:** 2026-07-26
**Symptoms:** Sidebar showed fictitious daemon status (PID 4812, 14d uptime). Watchdog thread checked for a binary that doesn't exist.
**Root Cause:** Original design assumed a separate guard binary that was never implemented. All watchdog logic was dead code against a phantom process.
**Solution:** Rewrote watchdog as self-monitoring: checks if `omnilock.exe` is still alive, restarts it if dead. Removed dependency on external guard binary.
**Files Changed:** `src-tauri/src/watchdog.rs`, `src/components/layout/Sidebar.tsx` (daemon status now static)
**Prevention:** Do not depend on external binaries that are not built by the project. Self-contained always beats external.

---

### Duplicate `installer_guard.rs` in Directory Tree
**Date:** 2026-07-26
**Symptoms:** AGENTS.md directory tree listed `installer_guard.rs` twice and omitted `models.rs` from that listing.
**Root Cause:** Copy-paste error when generating directory tree from memory.
**Solution:** Fixed tree to correctly list each file exactly once.
**Files Changed:** `AGENTS.md`
**Prevention:** When copy-pasting directory trees, verify each file appears exactly once.

---

### `DecryptBytes` Not Public (Cargo Error)
**Date:** 2026-07-26
**Symptoms:** `auth.rs` could not call `vault::encrypt_bytes()` — function was private (`fn` not `pub fn`).
**Root Cause:** Added `encrypt_bytes` as private function during password reset implementation. `auth.rs` is in a different module and can't access private items.
**Solution:** Changed `fn encrypt_bytes` to `pub fn encrypt_bytes`.
**Files Changed:** `src-tauri/src/vault.rs`
**Prevention:** Functions needed across module boundaries must be `pub`. When writing a helper for another module, make it `pub` from the start.

---

### `payload.security_question` Moved Before Reuse (Cargo Error)
**Date:** 2026-07-26
**Symptoms:** `auth.rs` failed to compile — `payload.security_question` moved into `config.security_question` on line 35, then was borrowed again on line 48.
**Root Cause:** String fields in Rust are moved (not copied) on assignment. Using the same field twice in the same function requires cloning or reordering.
**Solution:** Cloned `payload.security_question` into a local `question` variable before moving it into `config.security_question`. Used `question` clone for both config assignment and recovery save. Also removed redundant recomputation of `answer_hash`.
**Files Changed:** `src-tauri/src/auth.rs`
**Prevention:** In Rust, check if a string value will be used more than once before assigning it. Pre-clone values that are needed later, or restructure to avoid double usage.

---

---

### File-Unlock Child ACL Inheritance Gap

**Date:** 2026-07-30
**Symptoms:** Unlocking a folder restores the folder's own ACL (user+Admins+SYSTEM) but files inside remain inaccessible — they still have the restricted DACL (Admins+SYSTEM only) inherited from when the folder was locked.
**Root Cause:** `remove_safe_lock` calls `SetNamedSecurityInfoW` on the folder alone, which does NOT retroactively reset inheritance on existing child files that received inherited ACEs during lock propagation. The `SUB_CONTAINERS_AND_OBJECTS_INHERIT` flag propagates the restricted ACEs on lock, but restoring the parent's DACL on unlock doesn't undo ACEs already stored on children.

**Fix (app `file_locker.rs`):** `unlock_folder` now calls `unlock_children_recursive()` — walks all child files, checks if owner=SYSTEM (`verify_lock`), and calls `remove_safe_lock` on each.

**Fix (service `acl.rs`):** `remove_lock` now calls `remove_children_recursive()` for directory paths — same walk + reset for child files.

**Files Changed:** `src-tauri/src/file_locker.rs`, `service/src/acl.rs`

**Prevention:** When modifying ACLs on a folder, always consider existing child objects that may have inherited ACEs. A parent-only `SetNamedSecurityInfoW` does not propagate the change to children that already have the inherited ACE stored locally. Explicit per-child cleanup is required.

---

Last updated: 2026-07-30
