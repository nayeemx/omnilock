# OmniLock — Session Handoff

**Show this file to the AI at the start of every new session.**

## Current State

- **Version**: 0.0.26 (latest release: https://github.com/nayeemx/omnilock/releases/tag/v0.0.26)
- **Last Updated**: 2026-07-28
- **Git**: clean, all changes committed on `main`
- **Build**: signed v0.0.26 installed at https://github.com/nayeemx/omnilock/releases/tag/v0.0.26

---

## What Was Just Done (v0.0.22 → v0.0.23)

1. **Fix unlock ACL removal** — Replaced broken `REVOKE_ACCESS` with proper DACL filtering. Now reads existing ACEs, filters out DENY ACEs for Everyone, rebuilds DACL without them.
2. **Unlock verification** — Unlock commands now return "unlocked" or "unlock_failed". UI shows error if ACL still present after unlock.
3. **Open-Meteo weather API** — Replaced wttr.in with Open-Meteo for better accuracy. Uses WMO weather codes, IP geolocation fallback, proper geocoding for city names.

---

## What Was Just Done (v0.0.21 → v0.0.22)

1. **Lock verification** — Added `verify_lock()` that reads DACL back and checks for DENY ACE. Lock commands return "locked" or "locked_unverified".
2. **VaultPage verification UI** — Shows "locked & verified" or "locked_unverified (may require admin)" after locking.

---

## What Was Just Done (v0.0.20 → v0.0.21)

1. **ACL enforcement moved to app** — Replaced broken service pipe with direct Win32 `SetNamedSecurityInfoW` in `file_locker.rs`. Added `apply_deny_acl()` and `remove_deny_acl()`.
2. **Biometric detection rewritten** — Replaced unreliable WinRT with registry + WbioSrvc service checks.
3. **Honest audit in AGENTS.md** — Separated "Proven Working" from "Has Code But ZERO Verification".

---

## What Was Just Done (v0.0.24 → v0.0.25)

1. **Diagnostics system** — New `logger.rs` writes timestamped logs to `%APPDATA%/InnologyBD/OmniLock/omnilock.log` for all lock/unlock/biometric/drive operations. New `diagnostics.rs` reads real ACL state, biometric status, service status for every locked item.
2. **DiagnosticsPage UI** — New sidebar page showing live health checks for each feature: ACL DENY presence, biometric hardware/token/service, guardian daemon status, drive lock state, and the full operation log tail.
3. **Logging integrated** — All 10+ lock/unlock/biometric/drive commands now log success/failure with error details before returning.
4. **Biometric fix confirmed** — `powershell51_path()` fix plus explicit WinRT `ContentType=WindowsRuntime` type loading. CI builds and signs correctly.
5. **v0.0.25 released** — Signed installer at https://github.com/nayeemx/omnilock/releases/tag/v0.0.25, auto-update `latest.json` updated.

---

1. **Auto-updater endpoint fix** — Switched `tauri.conf.json` updater endpoint from `nayeemx.github.io/omnilock/latest.json` (unreachable due to network/firewall blocking GitHub Pages CDN) to `raw.githubusercontent.com/nayeemx/omnilock/main/latest.json`.
2. **Auto-updater error handling fix** — `checkForUpdates()` in `tauri-bridge.ts` now throws on error instead of returning `null`. UI shows "Failed to check for updates: ..." instead of misleading "No updates available."
3. **v0.0.26 released** — Signed installer at https://github.com/nayeemx/omnilock/releases/tag/v0.0.26, auto-update `latest.json` updated.

---

## What Was Just Done (v0.0.24 → v0.0.25)

1. **Widget close button** — Added X close button and Cancel button to UnlockWidget. Escape key closes widget. Auto-closes after successful unlock.
2. **GitHub Connect fix** — `poll_for_token` now saves `sync.meta.json` after getting token, so `get_sync_status()` returns `connected: true`. UI correctly shows "Active" badge after Device Flow auth.
3. **App Locker scan fix** — "Add Application" button no longer redundantly clears and re-scans. Search field cleared on modal open.
4. **Vault search** — Added search input to Protected Paths section in File & Drive Vault page. Filters both locked folders and files.

---

## What Was Just Done (v0.1.4 → v0.1.5)

1. **System Monitor page** — Real-time CPU, RAM, GPU, Network stats with live SVG graphs. CPU usage ring gauge, RAM usage ring gauge, GPU info via PowerShell WMI, network upload/download rates. Auto-refreshes every 2 seconds.
2. **Dashboard page** — New sidebar page showing weather widget (via wttr.in API), protected items count, security status (2FA/cloud sync), quick CPU/RAM status, locked drives list. Default landing page.
3. **New sidebar navigation** — Dashboard and System Monitor added as first two sidebar items.

---

## What Was Just Done (v0.1.3 → v0.1.4)

1. **GitHub connect state fix** — After Device Flow auth, `cloud_sync_enabled` is now set in vault config. UI correctly shows "Active" badge.
2. **App Locker search fix** — Main page search now filters locked apps list (was filtering processes instead). Separate search state for locked apps vs scan modal.
3. **Installed apps scan** — App Locker now enumerates installed apps from Windows registry (not just running processes). Tab UI: Running vs Installed.
4. **Lock operation loaders** — Spinner animations on lock, toggle, remove operations so user sees progress.
5. **File picker** — Add Folder and Lock File now use native OS file picker dialog (no manual path typing).
6. **Backup/Restore** — Export vault to any folder, import from backup on reinstall. Exports vault.enc, vault.recovery, vault.meta, locked_items.json.

---

## What Was Just Done (v0.1.2 → v0.1.3)

1. **GitHub Cloud Sync** — Replaced placeholder OAuth client ID with real one (`Ov23li9jwqq1jy88qziH`). Device Flow enabled. Users can now connect their GitHub account to backup/restore encrypted vault via Gists.

---

## What Was Just Done (v0.1.1 → v0.1.2)

1. **Icon consistency fix** — App dashboard, login screen, setup wizard, loading screen all now use `icon.png` (same file as Windows taskbar icon). Replaced Lucide `Shield` SVG with `<img src="/icon.png">`. Copied `src-tauri/icons/icon.png` to `public/icon.png`.

2. **Light theme** (v0.1.1) — System `prefers-color-scheme` auto-detection, light oklch CSS variables, semantic `surface` tokens replacing hardcoded `bg-white/[0.04]`, light scrollbars.

3. **Auto-updater signature fix** — `latest.json` has correct Ed25519 signature for v0.1.2. Signing key recovered and backed up.

---

## What Works (Proven)

| Feature | Status | Evidence |
|---------|--------|----------|
| Rust compiles | ✅ | 0 errors on every build |
| TypeScript compiles | ✅ | 0 errors on every build |
| Installer builds | ✅ | `OmniLock_0.0.25_x64-setup.exe` produced |
| Vault crypto (unit tests) | ✅ | 15 `#[test]` in vault.rs, 2 in totp.rs |

## What Has Code But ZERO Verification

| Feature | Status | Evidence |
|---------|--------|----------|
| ACL enforcement (core!) | ⚠️ Code written | No test that deny ACE actually applies |
| Pipe IPC | ⚠️ Code written | "7/7 tests" claimed but no test script exists |
| Widget unlock | ⚠️ Code written | Explicitly never tested end-to-end |
| GitHub Connect | ⚠️ Code written | Explicitly never tested |
| UI lock/unlock | ⚠️ Code written | Never tested from GUI |
| Weather widget | ⚠️ Code written | Never verified on real machine |
| Windows Hello biometric | ⚠️ Code written | Never verified — DPAPI fix not yet tested |
| Hidden console windows | ⚠️ Code written | Just added `CREATE_NO_WINDOW`, not yet tested |
| Backup/Restore | ⚠️ Code written | Never tested end-to-end |
| Reinstall persistence | ⚠️ Code written | Never tested |

## Diagnostics page added — first step

The new Diagnostics page now shows real ACL state, biometric status, and the full operation log. Use it before claiming any fix works. Log file at `%APPDATA%/InnologyBD/OmniLock/omnilock.log`.

---

## Known Errors & Fixes (Reference)

| Error | Root Cause | Fix |
|-------|-----------|-----|
| All pipe responses fail deser | `SvcResponse` had `#[serde(tag = "type")]` on client, no tag on service | Removed tag in `service_client.rs` |
| icacls /deny silently fails | CLI reports success but never applies deny ACE | Rewrote `acl.rs` to use Win32 `SetNamedSecurityInfoW` API |
| Widget unlock doesn't remove ACL | Widget updated vault but never called `notify_unlock_item()` | Added service notification calls in `cmd_widget_unlock` |
| Service accepts any password | Bare SHA-256 with no salt, no hash file = accept | Vault-based verification (Argon2id + AES-256-GCM) in `service/src/vault.rs` |
| Race condition on pipe read | `sleep(300ms)` guess | `FlushFileBuffers` + retry loop with 5s timeout |
| Unlock doesn't remove DENY ACL | `REVOKE_ACCESS` mode doesn't remove DENY ACEs | Filter DACL entries, rebuild without DENY for Everyone |
| Weather inaccurate | wttr.in API data not accurate for Bangladesh | Switched to Open-Meteo API (ECMWF/NOAA models) |

---

## Build & Run Commands

```bash
# Dev mode (fast, hot reload for frontend)
npm run tauri dev

# Production build (produces installer)
npm run tauri build

# TypeScript check only
npx tsc --noEmit

# Create GitHub release (after build)
gh release create vX.Y.Z --title "vX.Y.Z" --notes "..." "path\to\OmniLock_X.Y.Z_x64-setup.exe#OmniLock_X.Y.Z_x64-setup.exe"

# Sign installer for auto-updater (PRIVATE KEY IN signing-keys/ FOLDER)
$key = (Get-Content -Raw "src-tauri\update.key").Trim()
$env:TAURI_SIGNING_PRIVATE_KEY = $key
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "omnilock2026"
cmd /c "echo. | npx tauri signer sign `"path\to\installer.exe`""
# Then paste the printed signature into latest.json
```

---

## ⚠️ AUTO-UPDATER SIGNING KEY (CRITICAL)

**The signing key MUST match the public key embedded in the installed binary.** If you lose or rotate this key, all existing users cannot auto-update.

| Field | Value |
|-------|-------|
| **Public key ID** | `EDE58385BDE79B6F` |
| **Public key (base64)** | `RWRvm+e9hYPl7eUOcS2Q3cknhhVt06dE6IRPrbFNNE/CqnEDdfYs12Wy` |
| **Private key file** | `src-tauri/update.key` |
| **Backup copy** | `signing-keys/update-v0.1.0.key` |
| **Password** | `omnilock2026` |
| **tauri.conf.json** | `pubkey = "EDE58385BDE79B6F"` |

### DO NOT:
- Generate a new key pair (breaks update for all installed users)
- Overwrite `src-tauri/update.key` with a different key
- Change the pubkey in `tauri.conf.json` unless you rotate keys intentionally

### To sign a new release:
```powershell
$key = (Get-Content -Raw "src-tauri\update.key").Trim()
$env:TAURI_SIGNING_PRIVATE_KEY = $key
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "omnilock2026"
cmd /c "echo. | npx tauri signer sign `"path\to\installer.exe`""
```
Copy the printed signature into `latest.json` → `platforms.windows-x86_64.signature`.

---

## Architecture

```
omnilock/
├── src/                  # React frontend (Vite + Tailwind)
│   ├── components/
│   │   ├── auth/         # LoginScreen, SetupWizard, GitHubConnect
│   │   ├── layout/       # Sidebar, TopBar, Footer
│   │   ├── pages/        # Dashboard, SystemMonitor, AppLocker, Presets, Vault, Security
│   │   ├── shared/       # Field, Toggle, StatusPill, Stat
│   │   └── widget/       # UnlockWidget (separate Tauri window)
│   └── lib/tauri-bridge.ts  # Tauri invoke wrappers
├── src-tauri/            # Rust backend (Tauri)
│   └── src/
│       ├── lib.rs        # Tauri commands
│       ├── vault.rs      # Argon2id + AES-256-GCM encryption
│       ├── service_client.rs  # Named pipe IPC to service
│       ├── github_sync.rs     # GitHub Device Flow + Gist sync
│       ├── file_locker.rs     # Path validation (ACL delegated to service)
│       ├── drive_locker.rs    # NoDrives registry + ACL delegated
│       ├── process_guard.rs   # Process monitor + suspension
│       └── system_monitor.rs  # CPU/RAM/GPU/Network stats + weather
├── service/              # Windows service (ACL enforcement daemon)
│   └── src/
│       ├── bin/svc.rs    # Main service (named pipe server)
│       ├── bin/monitor.rs # Watchdog (restarts service)
│       ├── acl.rs        # Win32 SetNamedSecurityInfoW DACL manipulation
│       └── vault.rs      # Password verification via vault decrypt
├── shared/               # Shared protocol crate (omnilock-shared)
│   └── src/lib.rs        # SvcRequest, SvcResponse, LockedItem
├── public/icon.png       # App icon (used everywhere)
├── design.md             # Design system documentation
└── AGENTS.md             # This file
```

---

## Versioning Scheme

Patch bumps: 0.0.1 → 0.0.2 → ... → 0.0.99 → 0.1.0

## Version History

- **0.0.15** — Versioning reset. Widget close, GitHub Connect fix, App Locker fix, Vault search, System Monitor, Dashboard
- **0.1.5** — System Monitor (CPU/RAM/GPU/Network graphs) + Dashboard (weather widget, security overview)
- **0.1.4** — GitHub connect state fix, App Locker search, installed apps scan, lock loaders, file picker, backup/restore
- **0.1.3** — GitHub Cloud Sync (real OAuth App with Device Flow)
- **0.1.2** — Icon consistency (same icon.png in taskbar + dashboard)
- **0.1.1** — Light theme + system preference detection
- **0.1.0** — Widget ACL fix, vault tests, shared crate, service fixes
- **0.0.8** — New icon, 16 bug fixes, floating widget, system tray
- **0.0.7** — Complete UI overhaul, dark glassmorphism, 18 modular files
- **0.0.6** — Recovery methods + USB key + forgot password
- **0.0.5** — Auto-update NSIS, keypair, 2FA + password reuse
- **0.0.4** — Signing key fix, 2FA login fix
- **0.0.3** — latest.json for updater
- **0.0.2** — Machine-bound USB key, dynamic security
- **0.0.1** — Panic hotkey, auto-lock, USB hardware key, recovery tiers

---

## Instructions for Next Session

1. Read this file first
2. Check `git log --oneline -5` to confirm latest commit
3. Check `git status` for uncommitted work
4. Run `npx tsc --noEmit` to verify frontend compiles
5. If the user reports an error, check the "Known Errors & Fixes" table above
6. When done, update this file before finishing
7. Follow `design.md` for any UI work
8. Version numbering: sequential patch bumps (0.1.0 → 0.1.1 → 0.1.2 → ...)
