# Changelog

All notable changes to OmniLock are documented in this file.

Format: [SemVer](https://semver.org/) — `MAJOR.MINOR.PATCH`

## [1.0.1] - 2026-07-26

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

## [1.0.0] - 2026-07-26

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
| File locker (icacls) | ✅ Real | Not tested |
| Drive locker (DACL + NoDrives) | ✅ Real | Not tested |
| Panic hotkey (Win+Alt+L) | ✅ Real | Needs admin + user32 |
| Process suspension | ✅ Real | Needs admin |
| Watchdog (self-monitoring) | ✅ Real | Cannot easily test |
| Password reset UI flow | ✅ Real | TypeScript compiles |
| Auto-update (GitHub Releases) | ✅ Real | Signed builds |
| Frontend modular structure | ✅ Done | — |
| Inline error/success feedback | ✅ Done | All pages |
| Controlled Toggle component | ✅ Done | Parent-driven |
| 2FA flow (button→QR→verify) | ✅ Done | Sequential |
| Dark glassmorphism UI | ✅ Done | Visual |
| Sidebar + TopBar + 4 tabs | ✅ Done | Visual |

---

*End of CHANGELOG.md*
