# Changelog

All notable changes to OmniLock are documented in this file.

Format: [SemVer](https://semver.org/) — `MAJOR.MINOR.PATCH`

## [0.0.35] - 2026-08-07 (released — installer + signature on the GitHub release; runtime E2E still outstanding)

### Added
- **AES-256-GCM file/folder encryption locking** replaces the destructive ACL locking (`file_locker.rs` rewritten). Locking encrypts a file in place → `original.omnilock` (magic `OLCK`, nonce, original path, ciphertext+tag). Folder lock = recursive encrypt; folder stays browsable. Unlock = recursive decrypt.
- **`VaultConfig.file_encryption_key`** — 32 random bytes generated at vault setup (`auth.rs`), independent of the master password, auto-migrated for pre-encryption vaults on login/biometric-login.
- **Direct WBF fingerprint** (`biometric.rs` rewritten): `WinBioEnumBiometricUnits` / `WinBioOpenSession` / `WinBioIdentify` + SID compare — no PowerShell, no Windows Hello dialog. `Win32_Devices_BiometricFramework` feature added to windows-sys.
- **ACL Recovery**: `scan_acl_damage` (recursive SYSTEM-owned-item finder via FindFirstFile), `bulk_recover_acl` (force_unlock batch), new `cmd_scan_acl_damage`/`cmd_bulk_recover_acl` commands + `AclScannerForm` UI on the Diagnostics page (Scan → Fix All / per-item).
- **Widget temporary unlock + auto-re-lock**: `cmd_widget_unlock` no longer removes items from the vault; the widget re-locks the item on close (focus-lost) via `WIDGET_TEMP_UNLOCKED` + `ACTIVE_FILE_KEY`.
- **`.omnilock` file association** (`shell_context.rs`): double-clicking an encrypted file opens OmniLock with `--open-locked <path>` → unlock widget after login.
- **GitHub Release pipeline** (`build.yml`): tag pushes (`v*.*.*`) build + create a signed GitHub Release with installer + `.sig`; other pushes build + upload artifact.

### Fixed
- `drive_locker.rs` `lock_drive`/`unlock_drive` now take the encryption key (previously stale 1-arg calls to `lock_folder`/`unlock_folder` — compile errors).
- `cmd_rescue_unlock` now requires a session + file key and returns the decrypted path (was a stale keyless call with wrong return type).
- **File key missing after fingerprint login / widget unlock**: `cmd_biometric_login` and `cmd_widget_unlock` now set `state.file_key` + `ACTIVE_FILE_KEY` — lock/unlock would otherwise fail with "File encryption key not available".
- **Old service no longer re-inflicts ACL damage**: removed `notify_lock_file`/`notify_lock_folder`/`notify_lock_drive` from all encryption lock paths (the service applied owner=SYSTEM + restricted DACL and re-applied it at every boot).
- ACL recovery commands (`cmd_force_unlock`, `cmd_recover_acl`, `cmd_bulk_recover_acl`) now purge the service's persisted re-lock list via `notify_force_remove_locked_item`, so fixed files stay fixed across reboots.
- **Drive lock no longer encrypts the whole drive** — removed the dangerous recursive drive-root encryption; drives are locked with NoDrives (hide) only. `lock_folder` refuses drive roots so the folder UI can't encrypt `D:\` by accident.
- **Critical-path guard** (`check_protected_path`): refuses to lock drive roots, the vault directory (`%APPDATA%\InnologyBD` — would encrypt the key itself), or the running exe.
- **`unlock_file` refuses to overwrite a (re)created original file** (prevents silent data loss); recursive encrypt/decrypt skip symlinks/junctions so nothing outside the locked tree is touched.
- **Crash-proof key handling**: `do_encrypt`/`do_decrypt` validate the 32-byte key instead of panicking; `cmd_widget_unlock` migrates pre-0.0.35 vaults (was a hard error).
- `cmd_scan_acl_damage` / `cmd_bulk_recover_acl` are now async (`spawn_blocking`) so deep scans don't freeze the UI.
- `.omnilock` file association is registered automatically on app startup (`shell_context::register_extension_only`) — no manual context-menu install needed for double-click unlock.
- Removed unused imports (`Security::Authorization`, `Storage::FileSystem`); build is now warning-free.

### Infra
- **CI pipeline fixed**: every prior GitHub Actions run failed at "Build service daemon" because PowerShell `copy` refused to overwrite the committed `src-tauri/resources/*.exe`. Now `shell: bash` + `cp -f`. Tag pushes now build + create the signed release automatically.

### Security
- File/folder contents are now unreadable without the vault key (AES-256-GCM), instead of a system-level ACL that permanently removed the owner's access.
- Fingerprint identity is verified server-side (SID compare) before the DPAPI token is decrypted.

### Drive locking (v0.0.35)
Drive lock = **NoDrives hide only** (whole-drive recursive encryption was removed as a data hazard — it would encrypt installed apps). Direct path access (`D:\` in the address bar) still works; true blocking needs a kernel driver or per-subpath encryption — see `AGENTS.md` → "Priority issues to investigate".

## [0.0.34] - 2026-08-03

### Fixed
- **Widget steals focus every 2s while working**: the file-access monitor prompted the always-on-top unlock widget on every 2s poll for each locked folder open in Explorer. Now prompts once per folder (`PROMPTED_FOLDERS` static in `process_guard.rs`), cleared on relock or when the folder closes in Explorer.
- **Windows Hello bypassable**: `cmd_biometric_login` loaded the DPAPI-protected master password with no server-side verification; the Hello prompt was client-only. Verification (`authenticate_biometric`) now runs inside the command before the token is loaded, and the redundant client-side prompt was removed from `LoginScreen.tsx`.

## [0.0.33] - 2026-08-03

### Fixed
- **File-unlock child ACL fix v3**: recursive unlock now unconditionally resets every child file/dir ACL (v2 gated children on `verify_lock`, but children inherit the restricted DACL while keeping their original owner, so all were skipped and files inside locked folders stayed inaccessible)
- **v2 never compiled**: `unlock_files_recursive`/`remove_files_recursive` are `unsafe fn` but were called without an `unsafe` block
- Widget window missing in Tauri capability (fixed via `capabilities/default.json`)
- Removed dead bridge exports (`widgetListLocked`, `githubConnectToken`, `listBackups`, `restoreBackup`)
- `sync_vault_to_service` now called after every vault write

### Added
- `force_unlock` command (`SeTakeOwnershipPrivilege` + `SeRestorePrivilege`) for stubborn files
- `--info` CSS variable in both themes

## [0.0.32] - 2026-07-30

### Fixed
- **File-unlock child ACL inheritance gap**: Unlocking a folder now recursively resets ACLs on all child files that inherited the restricted DACL during the lock. Previously, child files remained inaccessible (Admins+SYSTEM only) after folder unlock.
  - App (`file_locker.rs`): `unlock_children_recursive` walks children, calls `remove_safe_lock` on each locked file
  - Service (`acl.rs`): `remove_children_recursive` same recursive reset for child files

---

## [0.1.3] - 2026-07-27

### Added
- GitHub Cloud Sync with real OAuth App (Device Flow enabled)

## [0.1.2] - 2026-07-27

### Fixed
- App logo now uses `icon.png` everywhere (dashboard, login, setup, widget) — taskbar and UI icon are now consistent

---

## [0.1.1] - 2026-07-27

### Added
- Light theme support with system preference auto-detection (`prefers-color-scheme`)
- Dark mode via Tailwind `class` strategy (was hardcoded dark-only)
- Semantic `surface` color tokens across all components (replaces hardcoded `bg-white/[0.04]` etc.)
- Light scrollbar styles
- `glass-subtle` and widget adapt to active theme

### Fixed
- GitHub OAuth device flow: better error message when client ID is invalid or not registered

---

## [0.1.0] - 2026-07-27

### Added
- GitHub cloud sync via Gist (Device Flow OAuth — placeholder client ID)
- Service vault-based password verification (Argon2id + AES-256-GCM)
- GitHub token encrypted at rest using Windows DPAPI
- Shared protocol crate (`omnilock-shared`) for pipe types
- 15 vault encryption unit tests

### Fixed
- **CRITICAL**: Widget unlock now properly notifies service to remove ACL deny (was silently failing)
- `save_locked_items_summary` now called in all lock/unlock paths (was missing in 7 commands)
- Pipe IPC: removed `#[serde(tag = "type")]` deserialization mismatch
- Pipe IPC: replaced `sleep(300ms)` with `FlushFileBuffers` + retry loop (race condition fix)
- File/folder locking: removed broken `icacls` calls; ACL enforcement delegated to Windows service
- Service password verification: no longer accepts any password when hash file missing
- Service `SvcRequest`/`SvcResponse` enums synced between Tauri app and service
- windows-sys 0.61 migration (HANDLE types, imports, pointer params)
- Widget no longer hides on unfocus when there's a pending unlock target

### Changed
- ACL enforcement: `icacls /deny` CLI → Win32 `SetNamedSecurityInfoW` API (in service)
- Service state saved before ACL operation (prevents data loss on ACL failure)
- Process monitor sleeps 5s when no apps locked (was 1s always polling)
- Panic hotkey uses windows-sys imports (was raw FFI for standard APIs)
- All crates version bumped to 0.1.0

### Security
- GitHub OAuth token encrypted with DPAPI (was plaintext on disk)
- Service verifies password by decrypting vault (was bare SHA-256)
- Widget unlock now properly removes ACL deny (was leaving protection in place)

---

## [0.0.8] - 2026-07-26

### Added
- Auto-update system via GitHub Releases (tauri-plugin-updater)
- "Check for Updates" button in Security page
- Signed update artifacts (.sig files) for secure updates
- `tauri-plugin-updater` and `tauri-plugin-process` dependencies

### Fixed
- 2FA TOTP secret encoding: changed from base64 to base32 (RFC 4648) — authenticator apps now work correctly
- 2FA setup in SetupWizard: removed broken `enable2FA` call after `setupVault` (no session state during setup)
- 2FA flow is now sequential: Enable button → QR → scan → code → verify → enabled → disable option
- TOTP code input now enforces 6-digit numeric only

### Changed
- SetupWizard simplified to 2 steps (password + security question) — 2FA enabled after first login from Security page
- SecurityPage uses clean state machine: `idle` → `setup` → `enabled`

### Security
- TOTP secrets now use base32 encoding for authenticator app compatibility
- Update artifacts signed with Ed25519 key pair

---

## [0.0.7] - 2026-07-26

### Added
- Complete UI overhaul: dark obsidian glassmorphism, oklch design tokens, cyan/violet neon accents
- Sidebar + topbar + 4 module tabs (Application Locker, System Presets, File & Drive Vault, Security & 2FA)
- Modular frontend: split monolithic App.tsx into 18 focused component files
- Password reset flow with security question (2FA wiped on reset)
- Vault recovery data stored separately in `vault.recovery` (AES-GCM encrypted with answer hash key)
- Vault versioning: `EncryptedVault.version` field + `vault.meta` stores `vault_version`
- Migration framework: `migrate_vault_if_needed()` runs on every unlock, transparent upgrades
- Real process suspension via `SuspendThread` FFI (was stub)
- Real panic hotkey via `RegisterHotKey(Win+Alt+L)` with raw user32.dll FFI
- All 6 system presets implemented (added Control Panel + System Restore — were no-ops)
- Watchdog self-monitoring (no external guard binary dependency)
- Argon2id-derived encryption key for vault (was `SHA-256(password)` — security fix)
- Inline error/success feedback on all pages (AppLocker, Presets, Vault, Security)
- Controlled Toggle component (was using local state, now driven by parent)
- 2FA flow: button→QR→verify (was showing QR immediately)
- QR code returns PNG data URI (was returning raw otpauth URL string)
- Auto-lock renamed from `setAutoLock` shadow to `autoLockMins` (fixed desync)

### Fixed
- QR code `get_qr_base64()` returning data:image/png;base64 URI (was `get_url()` returning raw otpauth string)
- Toggle component using local state instead of controlled prop
- Auto-lock local state shadowing imported `setAutoLock` function
- 2FA section showing QR immediately instead of "Enable 2FA" button first
- Silent `console.error` in catch blocks → now shows inline error/success messages
- Dead exports (`verifyTotp`, `SessionToken`) removed from tauri-bridge.ts
- Hardcoded Installer Guard "Active" badge → now conditional on `installer_guard_enabled`
- `SessionToken` return value from `unlockSession` properly handled

### Changed
- Window size: 1000×680 → 1280×800 (sidebar layout needed more space)
- Vault encryption: SHA-256(password) → Argon2id(password, salt)
- Watchdog: external `omnilock-guard.exe` dependency → self-monitoring
- File structure: 1 monolithic file → 18 modular files

### Security
- Vault encryption key now uses Argon2id output (32 bytes), not fast SHA-256
- Recovery data encrypted with answer hash as key (AES-GCM)
- Password reset wipes 2FA to prevent lockout after credential recovery

### Migration Notes
- **vault.enc format unchanged** — migration is transparent on first unlock
- **vault.meta** now includes `vault_version` field alongside `totp_enabled`
- **vault.recovery** is a new file created during setup and password reset
- Old vaults without `vault.recovery` will not have password reset capability (create one via Setup Wizard)

---

## Feature Status Reference

| Feature | Status | Tested in App? |
|---|:---:|:---:|
| Vault encrypt/decrypt | ✅ Real | Build only |
| Vault recover (password reset) | ✅ Real | UI wired, no E2E |
| Vault migration framework | ✅ Real | New — untested |
| 2FA setup/enable/disable | ✅ Real | Base32 encoding verified |
| Process scan + lock/toggle/remove | ✅ Real | IPC wired |
| System presets (all 6) | ✅ Real | Registry needs admin |
| Installer guard | ✅ Real | Not tested |
| File locker (Win32 API via service) | ✅ Real | Pipe tested |
| Drive locker (DACL + NoDrives) | ✅ Real | Pipe tested |
| Panic hotkey (Win+Alt+L) | ✅ Real | Needs admin + user32 |
| Process suspension | ✅ Real | Needs admin |
| Watchdog (self-monitoring) | ✅ Real | Cannot easily test |
| Password reset UI flow | ✅ Real | TypeScript compiles |
| Auto-update (GitHub Releases) | ✅ Real | Signed builds |
| GitHub cloud sync | ⚠️ Placeholder | OAuth client ID needed |
| Frontend modular structure | ✅ Done | — |
| Inline error/success feedback | ✅ Done | All pages |
| Controlled Toggle component | ✅ Done | Parent-driven |
| 2FA flow (button→QR→verify) | ✅ Done | Sequential |
| Dark glassmorphism UI | ✅ Done | Visual |
| Sidebar + TopBar + 4 tabs | ✅ Done | Visual |

---

*End of CHANGELOG.md*
