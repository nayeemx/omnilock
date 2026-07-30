# OmniLock — Session Handoff

**Show this file to the AI at the start of every new session.**

## Current State

- **Version**: 0.0.32 (unreleased, with file-unlock child-ACL bugfix)
- **Last Updated**: 2026-07-30
- **Git**: working on main, uncommitted fixes (file-unlock child ACL fix + AGENTS.md update)

---

## ROOT CAUSE: Window Not Appearing Since v0.0.25

**Bug**: `logger::init()` in `logger.rs` deadlocks immediately at startup.

**Why**: `init()` locks `LOG_FILE` mutex, then calls `log()` which tries to lock the **same non-reentrant `std::sync::Mutex`** — Rust's `Mutex` is not reentrant, so the thread hangs forever.

**Result**: Process starts (4 threads, ~8-9 MB), but never reaches Tauri's `.run()` — no window, 0-byte log file. Silent hang.

**Fixed in v0.0.29**: `drop(guard)` before `init()` returns, so `log()` can acquire the lock fresh. No `log()` call inside `init()`.

**All versions from v0.0.25 through v0.0.28 are broken** by this deadlock.

---

## What Was Just Done (This Session — File-Unlock Child ACL Fix)

1. **File-unlock bug discovered & fixed**: Unlocking a folder restored the folder's own ACL (user+Admins+SYSTEM) but **child files inside** retained the inherited restricted DACL (Admins+SYSTEM only) from when the folder was locked — they remained inaccessible.
   - **Root cause**: `SetNamedSecurityInfoW` on the folder alone does not retroactively reset inheritance on existing child files that received explicit ACEs during lock propagation.
   - **Fix (app `file_locker.rs`)**: `unlock_folder` now calls `unlock_children_recursive()` — walks all child files, checks if still locked (owner=SYSTEM), calls `remove_safe_lock` on each.
   - **Fix (service `acl.rs`)**: `remove_lock` now calls `remove_children_recursive()` — same recursive reset for child files when path is a directory.

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
| DENY Everyone permanently locks files | Windows ignores owner WRITE_DAC right when DENY Everyone with `GENERIC_ALL` is set | Replaced with owner=SYSTEM + restricted DACL (no DENY). Administrators recoverable via `SeTakeOwnershipPrivilege` |
| **File-unlock: folder accessible but children still locked** | `SetNamedSecurityInfoW` on folder doesn't reset existing inherited ACEs on child files | Recursive `unlock_children_recursive`/`remove_children_recursive` walks children and resets each |
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
│       ├── diagnostics.rs     # Live health checks for ACL/biometric/service
│       └── shell_access.rs    # Native COM Explorer path detection (IShellWindows)
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

## File Icon Overlay: Research & Decision (Preserved)

**Problem**: `IShellIconOverlayIdentifier` COM Shell Extension requires an EV-signed DLL. No free workaround exists for individual file overlays.

**Solution found**: Use the **Windows Cloud Files API** (WinRT `Windows.Storage.Provider`) to add a "Status" column in Explorer. This is the same approach OneDrive, Google Drive, and Nextcloud use. It:
- Shows an **icon + text** per file in the Status column
- Has **no 15-overlay limit** (per-file property, not global slot)
- Requires **no EV cert** (WinRT, not classic COM overlay)
- Works on **Windows 10 1709+**
- Supports **multiple state icons** (locked, shared, etc.)

### Implementation path
1. **Register a sync root** for the vault folder via `StorageProviderSyncRootManager.Register()` or registry
2. **Define custom property definitions** with icon resources (lock.png, unlock.png)
3. **Update per-file state** via `StorageProviderItemProperties.SetAsync()` when lock status changes
4. Explorer auto-adds the Status column with padlock icon

### Requirement
Sync root registration is officially gated behind **MSIX packaging** (Desktop Bridge/Centennial). Tauri currently builds a standard NSIS/installer.
- **Option A**: Wrap the Tauri build in MSIX → full Cloud Files API support (weeks of packaging work)
- **Option B**: Registry-only sync root registration (hacky, no MSIX) → Status column may not appear
- **Option C**: Accept folder-only `desktop.ini` icons + tray notifications for files

### Research references
- Microsoft Cloud Mirror sample: https://github.com/Microsoft/Windows-classic-samples/tree/master/Samples/CloudMirror
- Nextcloud implementation (lock state in Status column): https://github.com/nextcloud/desktop/issues/4854
- Cloud Files API docs: https://learn.microsoft.com/en-us/windows/win32/cfapi/build-a-cloud-file-sync-engine

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

---

## Next Session: Continue from Here

### Where we left off
- **Bug fixed**: File-unlock child ACL inheritance gap — unlocking a folder now recursively resets ACLs on child files (`unlock_children_recursive` in `file_locker.rs`, `remove_children_recursive` in `acl.rs`)
- **Build**: Production build `OmniLock_0.0.32_x64-setup.exe` has been built and signed with the auto-updater key
- **Uncommitted changes**: The binary fix + AGENTS.md update are uncommitted. The new recursive-fix code has been written but **not yet built into a new installer**

### Suggested next steps
1. **Build + sign a new installer** with the child-ACL fix included (`npm run tauri build`, then sign, update `latest.json`)
2. **Test the fix** end-to-end on Windows: lock a folder, verify files inside are inaccessible, unlock the folder, verify files inside are accessible
3. **Consider new features** from the backlog (Cloud Files API Status Column, USB removal lock, context menu, etc.) — see the "Feature ideas" notes preserved above

### Priority issues to investigate
- The file-unlock fix is **not yet built into a signed installer** — current `latest.json` points to the pre-fix build
- Cloud Files API (`StorageProviderItemProperties.SetAsync()`) likely fails for non-MSIX apps; `cloud_status.rs` may need a fallback mechanism

### Feature backlog (from earlier research)

| Feature | Priority | Notes |
|---------|----------|-------|
| Shell context menu (right-click → lock/unlock) | Medium | COM DLL, no EV cert, moderate effort |
| Lock history / audit trail UI | Low | Logger data exists, needs frontend page |
| Bulk import/export lock rules | Low | Useful for power users |
| Auto-lock on workstation idle | Medium | Already have idle detection, just needs folder re-lock trigger |
| Lock on USB removal | Low | Hardware-triggered |
| Drive lock (bitlocker-style) | Low | Skeleton exists in drive_locker.rs |
| File version backup before lock | Medium | Prevent data loss from ACL corruption |
| Cloud Files API Status Column | Medium | WinRT `StorageProviderItemProperties.SetAsync`, needs MSIX
