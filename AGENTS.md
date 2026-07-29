# OmniLock — Session Handoff

**Show this file to the AI at the start of every new session.**

## Current State

- **Version**: 0.0.29 (unreleased, fix for window-not-appearing root cause found)
- **Last Updated**: 2026-07-29
- **Git**: working on main, committing deadlock fix

---

## ROOT CAUSE: Window Not Appearing Since v0.0.25

**Bug**: `logger::init()` in `logger.rs` deadlocks immediately at startup.

**Why**: `init()` locks `LOG_FILE` mutex, then calls `log()` which tries to lock the **same non-reentrant `std::sync::Mutex`** — Rust's `Mutex` is not reentrant, so the thread hangs forever.

**Result**: Process starts (4 threads, ~8-9 MB), but never reaches Tauri's `.run()` — no window, 0-byte log file. Silent hang.

**Fixed in v0.0.29**: `drop(guard)` before `init()` returns, so `log()` can acquire the lock fresh. No `log()` call inside `init()`.

**All versions from v0.0.25 through v0.0.28 are broken** by this deadlock.

---

## What Was Just Done (v0.0.28 → v0.0.29)

1. **Deadlock fix in `logger.rs`** — `logger::init()` no longer calls `log()` while holding the mutex. This was causing v0.0.25+ to silently hang at startup with no window and a 0-byte log file.
2. **Removed diagnostic MessageBoxW** — The 3 debugging dialogs added in v0.0.28 are no longer needed since root cause is confirmed.
3. **Fallback updater endpoints** — Added `nayeemx.github.io/omnilock/latest.json` as a secondary endpoint after the primary `raw.githubusercontent.com` endpoint. If one is blocked by firewall, the other still works.
4. **Version bumped** to v0.0.29.

---

## Known Errors & Fixes (Reference)

| Error | Root Cause | Fix |
|-------|-----------|-----|
| App window not appearing (v0.0.25+) | `logger::init()` deadlocks on non-reentrant `Mutex` | `drop(guard)` before `log()` call in `init()`, or write directly |
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
│       ├── system_monitor.rs  # CPU/RAM/GPU/Network stats + weather
│       ├── logger.rs          # Timestamped operation logger
│       └── diagnostics.rs     # Live health checks for ACL/biometric/service
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

- **0.0.29** — Deadlock fix: `logger::init()` no longer deadlocks on non-reentrant `Mutex`. Fallback updater endpoints. Removed debug MessageBoxW.
- **0.0.28** — Added startup MessageBoxW for diagnostics. (Same deadlock bug, never produced working window.)
- **0.0.27** — Setup hook always returns `Ok(())`, no unwrap/panic, step logging. (Same deadlock bug.)
- **0.0.26** — Switched updater endpoint to `raw.githubusercontent.com`. Fixed `checkForUpdates()` error handling.
- **0.0.25** — Added diagnostics system + `logger.rs`. (INTRODUCED THE DEADLOCK — window stopped appearing.)
- **0.0.24** — Rescue mode, biometric fix, weather switch.
- **0.0.23** — Fix unlock ACL removal, Open-Meteo weather API.
- **0.0.22** — Lock verification.
- **0.0.21** — ACL enforcement moved to app, biometric rewrite.
- **0.0.20** — Hidden console windows, vault unlock fix, logout button.
- **0.0.19** — Improved biometric detection.
- **0.0.18** — Windows Hello fingerprint login + weather city search.
- **0.0.17** — Weather search by city name.
- **0.0.16** — Fix system monitor UI freeze.
- **0.0.15** — Versioning reset.

---

## Instructions for Next Session

1. Read this file first
2. Check `git log --oneline -5` to confirm latest commit
3. Check `git status` for uncommitted work
4. Run `npx tsc --noEmit` to verify frontend compiles
5. If the user reports an error, check the "Known Errors & Fixes" table above
6. When done, update this file before finishing
7. Follow `design.md` for any UI work
8. Version numbering: sequential patch bumps (0.0.29 → 0.0.30 → ...)
