# OmniLock — Session Handoff

**Show this file to the AI at the start of every new session.**

## Current State

- **Version**: 0.1.4 (latest release: https://github.com/nayeemx/omnilock/releases/tag/v0.1.4)
- **Last Updated**: 2026-07-27
- **Git**: clean, all changes committed on `main`
- **Build**: compiles clean (0 Rust errors, 0 TS errors)

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

## What Works (Verified)

| Feature | Status | Evidence |
|---------|--------|----------|
| Tauri app builds | ✅ | `OmniLock_0.1.2_x64-setup.exe` produced |
| Rust compiles | ✅ | 0 errors |
| TypeScript compiles | ✅ | 0 errors |
| Pipe IPC (Lock/Unlock/Status) | ✅ | 7/7 pipe tests passed (session 2026-07-27) |
| Tauri app launches | ✅ | Correct title, communicates with service |
| Vault persistence | ✅ | `vault.enc`, `vault.meta`, `vault.recovery` persist |
| Light/dark theme | ✅ | System preference auto-detection |
| Icon consistency | ✅ | Same `icon.png` everywhere |

## What Does NOT Work

| Issue | Blocker |
|-------|---------|
| GitHub OAuth | Client ID `Ov23li9jwqq1jy88qziH` registered, Device Flow enabled |
| Unlock widget popup | Never tested end-to-end |
| UI-level lock/unlock | Never tested from app GUI (only via pipe) |
| Reinstall persistence | Never tested |

---

## Known Errors & Fixes (Reference)

| Error | Root Cause | Fix |
|-------|-----------|-----|
| All pipe responses fail deser | `SvcResponse` had `#[serde(tag = "type")]` on client, no tag on service | Removed tag in `service_client.rs` |
| icacls /deny silently fails | CLI reports success but never applies deny ACE | Rewrote `acl.rs` to use Win32 `SetNamedSecurityInfoW` API |
| Widget unlock doesn't remove ACL | Widget updated vault but never called `notify_unlock_item()` | Added service notification calls in `cmd_widget_unlock` |
| Service accepts any password | Bare SHA-256 with no salt, no hash file = accept | Vault-based verification (Argon2id + AES-256-GCM) in `service/src/vault.rs` |
| Race condition on pipe read | `sleep(300ms)` guess | `FlushFileBuffers` + retry loop with 5s timeout |

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
$key = Get-Content -Raw "src-tauri\update.key"
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
$key = Get-Content -Raw "src-tauri\update.key"
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
│   │   ├── pages/        # AppLocker, Presets, Vault, Security
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
│       └── process_guard.rs   # Process monitor + suspension
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

## Version History

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
