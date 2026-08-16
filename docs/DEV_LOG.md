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
| File locker (encryption) | `file_locker.rs` | ✅ v0.0.35 rewrite | AES-256-GCM in place; ACL recovery kept |
| Vault storage (add files to vault) | `vault_storage.rs` | ✅ v0.0.36 | OVLF blobs + encrypted OVMF manifest |
| Stale-unlock detection after restart | `lib.rs` + `App.tsx` | ✅ v0.0.36 | `cmd_verify_locked_state` + Re-lock/Keep prompt |
| Legacy service boot sweep | `lib.rs` + `service_client.rs` | ✅ v0.0.36 | Purges persisted ACL `locked_items` at startup |
| Biometric login parity | `lib.rs` | ✅ v0.0.36 | Auto-lock minutes + USB-removal callback on fingerprint login |
| Biometric login (Windows Hello) | `biometric.rs` | ✅ v0.0.36 | `UserConsentVerifier` prompt, proven on HP 850 G5 |
| Drive locker | `drive_locker.rs` | ⚠️ Design concern | NoDrives + drive-root encryption (see AGENTS.md) |
| Biometric (direct WBF) | `biometric.rs` | ❌ dead on owner's HW | v0.0.35 rewrite; driver ignores background captures |
| Panic hotkey | `panic_hotkey.rs` | ✅ Complete | RegisterHotKey Win+Alt+L via FFI |
| Hotkey listener | `panic_hotkey.rs` | ✅ Complete | Windows message loop thread |
| Watchdog | `watchdog.rs` | ✅ Complete | Self-monitoring, no guard binary |
| Password reset | `vault.rs` + `auth.rs` | ✅ Complete | Answer → decrypt → re-encrypt → wipe 2FA |
| Frontend modularization | All `src/components/` | ✅ Done | 18 files, no monolith |
| Tauri bridge cleanup | `tauri-bridge.ts` | ✅ Done | Dead exports removed |
| Fingerprint engine (direct USB) | `omnilock-bio/` | 🚧 In progress | Rust port of python-validity (GPL-3.0); compiles, untested on hardware |
| Inline error feedback | All page components | ✅ Done | No silent catch blocks |
| Controlled Toggle | `Toggle.tsx` + parents | ✅ Done | Parent `on` prop drives state |
| 2FA flow fix | `SecurityPage.tsx` | ✅ Done | Button→QR→verify sequence |

---

## Issue Log

### v0.0.36: Stale-Unlock State Died With the App (temp-unlock didn't survive restart)
**Date:** 2026-08-16
**Symptoms:** Widget temp-unlock left items decrypted but the vault still listed them as locked. If the app exited while the item was open (common), the next login showed a pointless "already unlocked" prompt, and there was no way to re-encrypt or forget the entry.
**Root Cause:** Unlock state was purely in-memory (`WIDGET_TEMP_UNLOCKED`), while the vault config persisted the item as locked. Nothing reconciled the two after a restart.
**Solution:** `cmd_verify_locked_state` checks every `locked_files`/`locked_folders` entry against disk after login (`.omnilock` blob presence via `is_file_locked`/`is_folder_locked`) and returns stale entries. The frontend shows a full-screen prompt — **Re-lock all** (`cmd_relock_entries`: re-encrypts each path, per-item status; failures stay listed) or **Keep unlocked** (`cmd_forget_unlocked_entries`: removes the entries from the vault, saves config + summary). Deliberately no silent auto-re-encrypt: a folder the user is actively using must never be re-encrypted without consent.
**Files Changed:** `src-tauri/src/lib.rs`, `src/App.tsx`, `src/lib/tauri-bridge.ts`
**Prevention:** Any "temporarily lifted" lock must be re-checked against disk at the next login; reconcile vault intent with on-disk reality before trusting either.

---

### v0.0.36: Legacy Service Re-Applied ACL Locks At Boot
**Date:** 2026-08-16
**Symptoms:** After ACL Recovery fixed a file, the next boot re-locked it (owner=SYSTEM). Recovery commands purged the service list one path at a time, but any item the app never touched stayed persistent.
**Root Cause:** The service (`svc.rs`) persists `locked_items` to ProgramData and re-applies `acl::apply_lock` at startup, regardless of what the app does now (encryption locks bypass the service entirely).
**Solution:** One-time boot sweep in the app setup hook: `service_client::get_locked_items()` (new) then `notify_force_remove_locked_item` for each. Runs once per launch; after the first purge the list is empty → no-op.
**Files Changed:** `src-tauri/src/service_client.rs`, `src-tauri/src/lib.rs`
**Prevention:** Legacy persistence must be reconciled at app startup, not only when the user fixes one item at a time.

---

### v0.0.36: `cmd_biometric_login` Parity Gap
**Date:** 2026-08-16
**Symptoms:** Fingerprint login unlocked the vault but never applied auto-lock minutes nor the USB-removal callback — a locked drive removed mid-session would not lock the workstation, and idle auto-lock was inactive until a password login.
**Root Cause:** `cmd_unlock_session` wires these; `cmd_biometric_login` was written before those subsystems existed and was never updated.
**Solution:** `cmd_biometric_login` now also calls `auto_lock::set_auto_lock_minutes(config.auto_lock_minutes)` and `drive_locker::set_usb_removal_callback(...)` (LockWorkStation on locked-drive removal).
**Files Changed:** `src-tauri/src/lib.rs`
**Prevention:** Every session-establishing command must share one wiring routine; grep for `set_session_active`-style calls and check each login path.

---

### v0.0.36: Direct WBF Fingerprint Is Impossible On the Owner's Hardware
**Date:** 2026-08-16
**Symptoms:** "Fingerprint login never works" — the app's `WinBioIdentify` scan never returned: it blocked forever and ignored real finger touches.
**Diagnosis (hardware-proven):** On the HP EliteBook 850 G5 (Synaptics VFS7552, WBF driver `oem27.inf`), the WBF operational log (`Microsoft-Windows-Biometrics/Operational`) records **zero sensor events** during a 90-second `WinBioIdentify` scan with multiple real touches, while the user's own Windows fingerprint sign-ins appear in the same log every time (event 1004 + 1605 + 1702). The driver only serves the Windows Hello pipeline — background apps never get a capture. Root cause of the earlier v0.0.33/34 failures also found: that Hello script referenced the nonexistent WinRT type `UserConsentVerifierResult` (`UserConsentVerificationResult` is the real name) → "Unable to find type" on hardware.
**Solution:** `authenticate_biometric` now calls `Windows.Security.Credentials.UI.UserConsentVerifier.RequestVerificationAsync` through PowerShell 5.1 (`System.WindowsRuntimeSystemExtensions.AsTask`, `MakeGenericMethod` on `UserConsentVerificationResult`). Proven live: the Hello prompt appears, the fingerprint is accepted, `Verified` is returned. `check_biometric_available` is back to fast `sc query WbioSrvc` + `WindowsHello\Enabled=0x1` + Bio Credential Provider registry checks. All WBF code (incl. `Win32_Devices_BiometricFramework` Cargo feature) removed.
**Files Changed:** `src-tauri/src/biometric.rs`, `src-tauri/Cargo.toml`
**Prevention:** On this machine, the ONLY fingerprint path is Windows Hello. Do not reintroduce WinBio; do not use `UserConsentVerifierResult`.

---

### v0.0.36: Biometric "Available" Ignored Enrollments
**Date:** 2026-08-16
**Symptoms:** `check_biometric_available` returned `available: true` whenever a fingerprint sensor existed — even with zero fingerprints enrolled for the current user, so the login screen offered a biometric login that could never succeed.
**Root Cause:** The old check only called `WinBioEnumBiometricUnits` (sensor count) and skipped registry/Hello probes.
**Solution:** Rewrote the check: enumerate units → open a WBF session → for each unit call `WinBioEnumEnrollments` with a `WINBIO_IDENTITY` built from the current user's SID (`GetLengthSid`/`CopySid`); `available: true` only when ≥1 fingerprint is enrolled for that SID. Reason strings ("sensor found, but no fingerprint enrolled for this account…") guide the user. windows-sys 0.61 exports `WinBioEnumEnrollments` but not `WinBioEnumIdentities`, so the SID is built manually.
**Files Changed:** `src-tauri/src/biometric.rs`
**Prevention:** Availability checks must verify the actual credential exists (enrollment), not just the hardware.

---

### GitHub Actions Failed On Every Push (copy refuses to overwrite committed binaries)
**Date:** 2026-08-07
**Symptoms:** Every `Build OmniLock` workflow run failed ~1m40s in — but only after the release itself had been created manually, which is why "nothing ever reached GitHub" via the pipeline. Users saw releases without the pipeline working.
**Root Cause:** The "Build service daemon" step ran on the PowerShell default shell: `copy service\target\release\omnilock-svc.exe src-tauri\resources\`. The `src-tauri/resources/omnilock-*.exe` binaries are **committed to git**, so the destination already exists and PowerShell 5.1 `Copy-Item` errors with "An item with the specified name … already exists" instead of overwriting.
**Solution:** `shell: bash` + `cp -f` for the copy commands. Tag pushes now reach `tauri-action` and create the signed release.
**Follow-up:** The first successful build then failed at the release step with "Resource not accessible by integration" — the default `GITHUB_TOKEN` is read-only. Added `permissions: contents: write` to the workflow top so tauri-action can create the release + upload assets.
**Files Changed:** `.github/workflows/build.yml`
**Prevention:** Never use PowerShell `copy`/`Copy-Item` without `-Force` when the destination may exist in the checkout. Any action that creates a GitHub Release needs an explicit `permissions: contents: write` block. Verify CI from the very first commit, not after the fact.

---

### Safety Hardening Pass (code review driven)
**Date:** 2026-08-07
**Symptoms/Findings:** A review of the v0.0.35 encryption rewrite surfaced several data-safety hazards: (1) locking a drive root would recursively encrypt the ENTIRE drive (installed apps, hours of operation); (2) locking the vault directory (`%APPDATA%\InnologyBD`) would encrypt the key that unlocks everything; (3) `unlock_file` would silently overwrite a recreated original file; (4) recursive walks followed junctions/symlinks, potentially encrypting files outside the locked tree; (5) `Key::from_slice` panics on wrong-length keys (command crash); (6) sync scan commands could freeze the UI on large trees; (7) widget unlock of a pre-0.0.35 vault hard-errored instead of migrating the key; (8) the `.omnilock` double-click flow needed a manual context-menu install.
**Solution:** `check_protected_path` guard (drive roots, vault dir, running exe) in `lock_file`/`lock_folder`; drive locking reduced to NoDrives-hide (no whole-drive encryption); no-clobber guard in `unlock_file`; symlink/junction skip in recursive walks; 32-byte key validation in `do_encrypt`/`do_decrypt`; key migration added to `cmd_widget_unlock`; `cmd_scan_acl_damage`/`cmd_bulk_recover_acl` made async via `spawn_blocking`; `.omnilock` association auto-registered on startup (`register_extension_only`); frontend fix so failed ACL repairs stay visible.
**Files Changed:** `src-tauri/src/file_locker.rs`, `src-tauri/src/drive_locker.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/shell_context.rs`, `src/components/pages/DiagnosticsPage.tsx`
**Prevention:** Any lock feature that can be pointed at a user-selected path needs a critical-path guard. Encryption with in-place overwrite needs explicit no-clobber + symlink handling. Always review new encryption code for panic paths (slice casts) and blocking calls.

---

### ACL Locking Permanently Destroyed Access — Replaced With AES-256-GCM Encryption
**Date:** 2026-08-07
**Symptoms:** The old lock (`apply_safe_lock`) set NTFS owner=SYSTEM + restricted DACL (Admins+SYSTEM only). Locked files/folders became permanently inaccessible to the user, and unlocking was unreliable — this was the user's #6 complaint and the reason "nothing works in real life".
**Root Cause:** A user-mode ACL lock that removes the owner from the DACL is destructive and hard to reverse. Windows ignores WRITE_DAC when the owner is SYSTEM and the DACL doesn't include the user.
**Solution:** Rewrote `file_locker.rs` around **AES-256-GCM encryption**: locking encrypts the file in place to `original.omnilock` (magic `OLCK`, nonce, original path, ciphertext+tag) and deletes the original. Unlock decrypts in place. The key is `VaultConfig.file_encryption_key` (32 random bytes, generated at vault setup, independent of the master password, auto-migrated on login).
**Files Changed:** `src-tauri/src/file_locker.rs`, `models.rs`, `auth.rs`, `lib.rs`, `process_guard.rs`, `drive_locker.rs`, `Cargo.toml` (aes-gcm, getrandom already present)
**Prevention:** Never strip the owner's own DACL entry as a lock mechanism. Encryption is reversible with the right key; ACL surgery is not.

---

### windows-sys 0.61 Doesn't Export WINBIO_* Named Constants
**Date:** 2026-08-07
**Symptoms:** `cargo check` errors: `cannot find value WINBIO_TYPE_FINGERPRINT / WINBIO_POOL_SYSTEM / WINBIO_FLAG_DEFAULT / WINBIO_ID_TYPE_SID`; also earlier unresolved-import errors mixing `windows` and `windows-sys` crate types.
**Root Cause:** windows-sys 0.61 does not re-export these WBF constants as named items, and mixing the `windows` crate's `Result`-returning API with `windows-sys` raw HRESULT functions caused type mismatches.
**Solution:** Use **windows-sys exclusively** in `biometric.rs` (glob imports, same style as `file_locker.rs`) and define the missing constants as raw `u32` from SDK header values: `WINBIO_TYPE_FINGERPRINT=0x8`, `WINBIO_POOL_SYSTEM=0x1`, `WINBIO_FLAG_DEFAULT=0x0`, `WINBIO_ID_TYPE_SID=3`. Check HRESULTs as `hr < 0`.
**Files Changed:** `src-tauri/src/biometric.rs`, `src-tauri/Cargo.toml` (feature `Win32_Devices_BiometricFramework` on windows-sys)
**Prevention:** For Win32 APIs in this project use windows-sys; when a constant is missing, define it from the SDK header rather than switching crates.

---

### Encryption Rewrite Left Stale Call Sites (Compile Errors)
**Date:** 2026-08-07
**Symptoms:** After `lock_file/unlock_file/lock_folder/unlock_folder` gained a `key_material` parameter, `drive_locker.rs` (2 sites) and `lib.rs` `cmd_rescue_unlock` still called them with 1 arg → `error[E0061]`; `cmd_rescue_unlock`'s `Ok(())` arm also mismatched the new `Result<String, String>`.
**Root Cause:** The big rewrite never re-ran the compiler before the session hit usage limits.
**Solution:** Thread `key_material` through `lock_drive`/`unlock_drive` (4 call sites in `lib.rs`), give `cmd_rescue_unlock` a `State` + `require_file_key`, and return the decrypted path. Verified every call site via search (18 matches) and re-ran `cargo check` until clean.
**Files Changed:** `src-tauri/src/drive_locker.rs`, `src-tauri/src/lib.rs`
**Prevention:** Always run `cargo check` after a multi-file signature change; search for every call site before assuming the build is green.

---

### File Encryption Key Missing After Fingerprint Login / Widget Unlock
**Date:** 2026-08-07
**Symptoms:** After logging in with the fingerprint or unlocking via the widget, the next lock/unlock would fail with "File encryption key not available. Please log in first."
**Root Cause:** Only `cmd_unlock_session` populated `state.file_key`; `cmd_biometric_login` and `cmd_widget_unlock` set session/config/password but never the file key (nor the global `ACTIVE_FILE_KEY` in the widget path).
**Solution:** Both commands now migrate + set `state.file_key` and call `set_active_file_key`.
**Files Changed:** `src-tauri/src/lib.rs`
**Prevention:** Any code path that establishes a session must also establish the file key; grep for `session_token` writes and check each has a matching `file_key` write.

---

### Service Re-Inflicted Old ACL Damage On Encryption Locks
**Date:** 2026-08-07
**Symptoms:** After the encryption rewrite, the app still called `service_client::notify_lock_file/folder/drive`. The service (`service/src/bin/svc.rs`) responds by applying `acl::apply_lock` (owner=SYSTEM + restricted DACL) and persisting the item for **re-application at every boot** — re-creating the exact destructive behavior the user reported, on top of encryption.
**Root Cause:** The rewrite updated the app's lock mechanics but kept the legacy service notifications.
**Solution:** Removed `notify_lock_file`/`notify_lock_folder`/`notify_lock_drive` from all encryption lock paths (files, folders, drives, apps). Recovery commands (`cmd_force_unlock`, `cmd_recover_acl`, `cmd_bulk_recover_acl`) now call `notify_force_remove_locked_item` so the service stops re-locking fixed paths at boot.
**Files Changed:** `src-tauri/src/lib.rs`
**Prevention:** When the lock mechanism changes, audit every `service_client::notify_*` call — the service's ACL daemon is legacy for lock enforcement.

---

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

## omnilock-bio Session (2026-08-16) — Rust fingerprint engine scaffolded, compiles clean

**Date:** 2026-08-16
**Status:** `cargo check` clean (14 dead-code warnings only — all reserved for enroll/verify milestone). Not yet tested on hardware; sensor still bound to the Synaptics WBF driver (`oem27.inf`).

**Done:** reference read (usb/tls/blobs/blobs_d51/init_flash/flash/sensor/util/hw_tables); provisioning flow mapped; SMBIOS hw_key verified (`HP EliteBook 850 G5\0` + `5CG9233Q98\0` → `485020456c...`); 7 blobs extracted to `resources\*.bin`; `hw_tables.rs` regenerated (417 DeviceInfo + 20 FlashIcInfo rows); `windows\omnilock-sensor.inf` + `scripts\rebind.ps1`/`revert.ps1`; Rust modules `usb.rs` (WinUSB transport), `tls.rs` (full handshake/pairing/flash-block port), `crypto.rs`, `hwkey.rs`, `error.rs`, `hw_tables.rs` (+lookups), `flash.rs`, `sensor.rs`, `init_flash.rs`, `main.rs` (probe/reprovision/reboot/hwkey).

**Errors fixed this session (45 → 0):**
- windows-sys 0.61 gotchas: `CreateFileW`/`FILE_*` live in `Win32::Storage::FileSystem` (not `System::IO`); `GENERIC_READ`/`GENERIC_WRITE` in `Win32::Foundation`; `CreateEventW` requires the `Win32_Security` feature (SECURITY_ATTRIBUTES param); `HDEVINFO` is `isize` (`devs == 0`, not `is_null()`); `WinUsb_QueryPipe` is 4-arg with `WINUSB_PIPE_INFORMATION` (`PipeId`/`MaximumPacketSize` fields OK); `WINUSB_INTERFACE_HANDLE` is `*mut c_void`.
- p256/ecdsa 0.16 gotchas: `as_affine()` (not `to_affine()`); `SigningKey::from_bytes` wants `FieldBytes` (`(&[u8;32]).into()`); `sign_prehash`/`verify_prehash` need `signature::hazmat::{PrehashSigner, PrehashVerifier}` in scope; `Scalar::from_repr` needs `elliptic_curve::PrimeField`; `NonZeroScalar::new(scalar)` (no `TryFrom<Scalar>`); `SharedSecret::raw_secret_bytes()` (no `as_bytes()`); sec1 `Coordinates` variant is `Uncompressed { x, y }` (not `Xy`); `DerSignature` via `sig.to_der().as_bytes()`.
- cbc 0.1: `cbc::cipher::block_padding::NoPadding` (no `cbc::alloc` module).
- Reference-matching fixes: `PartitionInfo::serialize` is `<BBHLL` (access_lvl is u16) + 4 zeros + sha256(12-byte hdr); read/write_flash cmd is `<BBBHLL` (13 bytes, H=0); read_flash size field at rsp[2..6]; `make_cert` DER sig = `msg(0xc0) + <L der> + der + zero-pad to 444`; `encrypt_key` pads with `l` copies of value `l`; `hs_key` prf output byte-reversed then little-endian scalar.
- Generator `gen_hw_tables.py` now emits `"double-quoted"` names and re-emits `dev_info_lookup`/`flash_ic_table_lookup` (they were clobbered by regeneration).

**Next:** user runs `scripts\rebind.ps1` as Administrator (binds sensor to WinUSB, `Service=WinUSB`) → `omnilock-bio probe` (expect PairingFailed → `reprovision`) → enroll/verify milestone (`db.py`/`operation.py`/`registration.py`/`verification.py` reference) → named-pipe daemon → OmniLock integration.

**Files Changed:** `omnilock-bio\` (all), `docs\DEV_LOG.md`

**Prevention:** windows-sys paths change between versions — verify imports against the actual vendored source before writing code. Regenerated files must reproduce lookups/helpers or they silently vanish.

---

Last updated: 2026-08-16
