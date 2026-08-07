# OmniLock — Session Handoff

**Show this file to the AI at the start of every new session.**

## Current State

- **Version**: 0.0.35 (version bumped, release published via CI — installer + signature on the GitHub release)
- **Last Updated**: 2026-08-07
- **Git**: working on `main`, v0.0.35 committed + pushed + tagged; CI creates the signed release on tag push.
- **Build status**: ✅ `cargo check` (src-tauri) — 0 errors, 0 warnings. ✅ `npx tsc --noEmit` — clean. ✅ `cargo check` (service) — clean. ✅ Release build + signature produced by CI (`.sig` + installer on the v0.0.35 release). ❌ Runtime E2E testing on a real machine still outstanding.

---

## ⚠️ CRITICAL: How a Future AI Should Start (Reading Order)

1. Read this file **entirely** (especially "Next Session: Continue from Here").
2. `git status` + `git diff --stat` — the 12 modified files ARE the v0.0.35 work. Do NOT revert them.
3. Run `cargo check` (src-tauri) + `npx tsc --noEmit` — both must be clean.
4. Read `CHANGELOG.md` → `docs/DEV_LOG.md` → `docs/ARCHITECTURE.md` → `docs/SECURITY.md` for depth.
5. After ANY work (code, docs, research): **update all `.md` files** (AGENTS.md current state, CHANGELOG.md entry, DEV_LOG.md entry, ARCHITECTURE.md if commands/schema changed). This is a standing rule — always end a session by updating the .md files so the next session can resume instantly.

---

## What Was Done in This Session (2026-08-07) — the v0.0.35 feature set

The user (owner) reported 6 core problems. This session continued the interrupted implementation and **finished + verified** it. Everything below is in the working tree.

### 1. Fingerprint login now uses the Windows Biometric Framework DIRECTLY (no Windows Hello)

User requirement #5: "my computer has fingerprint sensor… I don't want it to work via Windows Hello."

- **Old**: `biometric.rs` called PowerShell → `Windows.Security.Credentials.UI.UserConsentVerifier` = literally the Windows Hello dialog.
- **New** (`src-tauri/src/biometric.rs`, fully rewritten): raw WBF C API via `windows-sys` glob imports (same style as `file_locker.rs`):
  - `check_biometric_available()` → `WinBioEnumBiometricUnits` (counts real sensors; no registry/Hello checks)
  - `authenticate_biometric()` → `WinBioOpenSession(FINGERPRINT, POOL_SYSTEM)` → `WinBioIdentify()` (blocks for finger, 30 s timeout via `tokio::time::timeout`) → verifies returned `WINBIO_IDENTITY.Type == WINBIO_ID_TYPE_SID (3)` → `EqualSid` against current user → `WinBioCloseSession`
  - No PowerShell, no Windows Hello dialog. OmniLock owns the prompt.
- **Cargo.toml**: added `Win32_Devices_BiometricFramework` to **windows-sys** features.
- **Gotcha learned**: windows-sys 0.61 does NOT re-export `WINBIO_TYPE_FINGERPRINT`, `WINBIO_POOL_SYSTEM`, `WINBIO_FLAG_DEFAULT`, `WINBIO_ID_TYPE_SID` as named items — they are defined as raw `u32` constants at the top of `biometric.rs` from SDK header values. Do not delete them.
- Frontend: `SecurityPage.tsx` renamed "Biometric Login (Windows Hello)" → "Biometric Login (Fingerprint)".
- **Server-side enforcement** (from v0.0.34, kept): `cmd_biometric_login` verifies the fingerprint inside the command before loading the DPAPI token.

### 2. AES-256-GCM encryption-based locking REPLACES the old ACL locking (user requirement #6 — "ACL lock was destructive")

- **Old**: `file_locker.rs` `apply_safe_lock` set NTFS owner = SYSTEM + restricted DACL → files/folders became permanently inaccessible (even to the user). This is what broke the user's already-locked files.
- **New** (`src-tauri/src/file_locker.rs`, rewritten, `lock_file/unlock_file/lock_folder/unlock_folder` now take `key_material: &[u8]`):
  - Locking a file **encrypts it in place**: `original.txt` → `original.txt.omnilock`, original deleted.
  - Blob layout: `[4] "OLCK" magic | [1] version=1 | [12] AES-GCM nonce | [4] original path len (u32 LE) | [N] original path (UTF-8) | [*] ciphertext+tag`
  - Folder lock = recursive encrypt of every file inside (folder itself stays browsable). Folder unlock = recursive decrypt.
  - **Key**: 32 random bytes stored in the vault as `VaultConfig.file_encryption_key` (`models.rs`, `auth.rs` generates at setup). It is independent of the master password (survives password changes). Key is loaded into `AppState.file_key` + global `ACTIVE_FILE_KEY` at login.
  - `verify_lock` / `is_file_locked` / `is_folder_locked` detect `.omnilock` blobs.
  - Old ACL-damage recovery functions are **kept**: `safe_recover_acl`, `force_unlock`, `scan_acl_damage`, `bulk_recover_acl`.
- **Vault migration**: `cmd_unlock_session` and `cmd_biometric_login` auto-generate `file_encryption_key` for pre-encryption vaults (len != 32 → generate + re-encrypt). **Old-format locks (owner=SYSTEM) are NOT auto-migrated** — user fixes them via the ACL Recovery scanner below.

### 3. ACL Recovery: recursive scanner + bulk fixer UI (fixes the user's already-broken files)

- `file_locker.rs`: `scan_acl_damage(root) -> Vec<String>` walks with `FindFirstFile/FindNextFile` (works even under restricted ACLs) and collects every item owned by SYSTEM (S-1-5-18) — the old lock's fingerprint. `bulk_recover_acl(paths)` runs `force_unlock` on each.
- `lib.rs`: `cmd_scan_acl_damage`, `cmd_bulk_recover_acl` (both registered). `cmd_force_unlock`/`cmd_recover_acl`/`cmd_bulk_recover_acl` now also call `service_client::notify_force_remove_locked_item` so the service doesn't re-apply the old ACL lock at boot.
- `tauri-bridge.ts`: `scanAclDamage`, `bulkRecoverAcl`.
- `DiagnosticsPage.tsx`: new `AclScannerForm` — path input + Scan → list of damaged items → "Fix All N Items" or per-item Fix → ✅/❌ results.

### 4. Widget popup: temporary unlock + AUTO-RE-LOCK on close (user requirement #4)

- `lib.rs`: new `WIDGET_TEMP_UNLOCKED` static. `cmd_widget_unlock` no longer removes items from the vault config (item stays logically locked); it lifts the lock temporarily and stores the target.
- Widget window `Focused(false)` event → re-applies `lock_file`/`lock_folder`/`lock_drive` using `get_active_file_key()`, then hides. Close the widget → item re-locks automatically.
- Also added `--open-locked <path>` CLI arg + `.omnilock` file association (`shell_context.rs` registers `HKEY_CLASSES_ROOT\.omnilock → OmniLockFile` → `OmniLock.exe --open-locked "%1"`). Double-clicking a `.omnilock` file opens OmniLock and shows the unlock widget after login.

### 5. GitHub Release pipeline (user complaint: "new version not on GitHub with signature")

- `.github/workflows/build.yml`: builds the service + monitor, then:
  - **Tag push (`v*.*.*`)** → `tauri-apps/tauri-action` creates a **signed GitHub Release** (signing key from repo secrets) with the installer + `.sig` attached.
  - Non-tagged push / workflow_dispatch → build only + artifact upload.
- Repo secrets **are configured** on GitHub: `TAURI_SIGNING_PRIVATE_KEY`, `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` (set 2026-07-28).
- **⚠️ CI bugs fixed this session** (every previous run failed before reaching the release):
  1. "Build service daemon" failed: PowerShell `copy` refuses to overwrite the **committed** `src-tauri/resources/omnilock-*.exe` binaries ("item already exists"). Fixed with `shell: bash` + `cp -f`.
  2. tauri-action failed with "Resource not accessible by integration" — the default `GITHUB_TOKEN` is read-only. Fixed with `permissions: contents: write` at the workflow top.
  Never reintroduce either.

### 6. Fixes this session applied on top of the interrupted work (IMPORTANT — the previous session left these broken)

| # | Issue found | Fix |
|---|---|---|
| a | **Compile errors**: `drive_locker.rs` called `lock_folder/unlock_folder` without the new key param; `lib.rs` `cmd_rescue_unlock` had a stale `unlock_file(&path)` + wrong match arm | `lock_drive`/`unlock_drive` now take `key_material`; all 4 call sites pass the key (`cmd_add_locked_drive`, `cmd_remove_locked_drive`, `cmd_widget_unlock` drive branch, widget auto-relock drive branch). `cmd_rescue_unlock` now takes `State`, calls `require_file_key`, returns the decrypted path |
| b | **File key never set on biometric login**: `cmd_biometric_login` set session/password/config but NOT `state.file_key`/`ACTIVE_FILE_KEY` → every lock/unlock after a fingerprint login would fail with "File encryption key not available" | Added key migration + `state.file_key` set + `set_active_file_key` in `cmd_biometric_login` |
| c | **File key never set on widget unlock**: same bug in `cmd_widget_unlock` (only set the global, not `state.file_key`) | Now sets both |
| d | **Old service would re-inflict ACL damage**: app still called `service_client::notify_lock_file/folder/drive`, which makes the service apply owner=SYSTEM + restricted DACL and persist it for re-application at every boot — exactly the destructive behavior the user reported | Removed those notifications from all encryption lock paths (`cmd_add_locked_file/folder/drive`, `cmd_toggle_locked_app`, `cmd_add_locked_app`). ACL recovery commands now purge the service's persisted list |
| e | Unused imports + dead code warnings | Removed unused `Security::Authorization` (biometric.rs) + `Storage::FileSystem` (file_locker.rs) glob imports; `#[allow(dead_code)]` on reserved `backup_path_for` |
| f | **Drive lock would encrypt the WHOLE drive** (data hazard: installed apps, hours of operation) | `drive_locker` no longer calls recursive `lock_folder` on the drive root — drive lock = NoDrives hide only (safe). `lock_folder` now **refuses drive roots** so the folder UI can't encrypt `D:\` by accident |
| g | **No critical-path guard** (locking the vault dir would encrypt the key itself; locking the exe bricks the app) | `check_protected_path` in `file_locker.rs` refuses: drive roots, `%APPDATA%\InnologyBD` (vault), and the running exe — enforced in `lock_file` + `lock_folder` |
| h | **`unlock_file` would silently overwrite a (re)created original file** (data loss) | Refuses to write when the original already exists |
| i | **Recursive encrypt/decrypt follows junctions/symlinks** — could encrypt files OUTSIDE the locked tree (OneDrive redirects etc.) | Skip symlinks/reparse points in `encrypt_dir_recursive` / `decrypt_dir_recursive` |
| j | **`Key::from_slice` panics on wrong-length key** — a panic in a command kills the app | `do_encrypt`/`do_decrypt` validate 32-byte key and return an error instead |
| k | **`cmd_scan_acl_damage` / `cmd_bulk_recover_acl` run synchronously** — deep walks can freeze the UI | Made `async` + `tokio::task::spawn_blocking` |
| l | **Pre-0.0.35 vault + widget unlock before main-app login** failed (widget only errored on key length instead of migrating) | `cmd_widget_unlock` now runs the same key migration as the other login paths |
| m | `.omnilock` double-click flow required manual context-menu install | New `shell_context::register_extension_only()` called in the setup hook on every launch (idempotent) |
| n | Frontend: failed per-item ACL fix vanished from the damaged list | `AclScannerForm.handleFixOne` keeps items whose fix returned an error |

---

## Files Modified in This Session (git status — DO NOT REVERT)

```
 M .github/workflows/build.yml
 M src-tauri/Cargo.toml
 M src-tauri/src/auth.rs
 M src-tauri/src/biometric.rs
 M src-tauri/src/drive_locker.rs
 M src-tauri/src/file_locker.rs
 M src-tauri/src/lib.rs
 M src-tauri/src/models.rs
 M src-tauri/src/process_guard.rs
 M src-tauri/src/shell_context.rs
 M src/components/pages/DiagnosticsPage.tsx
 M src/components/pages/SecurityPage.tsx
 M src/lib/tauri-bridge.ts
```

Plus the documentation updated in this session: `AGENTS.md`, `CHANGELOG.md`, `README.md`, `docs/ARCHITECTURE.md`, `docs/BUILD_SETUP.md`, `docs/DEV_LOG.md`, `docs/INSTRUCTION.md`, `docs/PRD.md`, `docs/SECURITY.md`.

---

## Known Errors & Fixes (Reference)

| Error | Root Cause | Fix |
|-------|-----------|-----|
| App window not appearing (v0.0.25+) | `logger::init()` deadlocks on non-reentrant `Mutex` | `drop(guard)` before `log()` call in `init()` |
| All pipe responses fail deser | `SvcResponse` had `#[serde(tag = "type")]` on client, no tag on service | Removed tag in `service_client.rs` |
| icacls /deny silently fails | CLI reports success but never applies deny ACE | Rewrote `acl.rs` to use Win32 `SetNamedSecurityInfoW` |
| **ACL lock permanently destroys access (user #6)** | `apply_safe_lock` set owner=SYSTEM + restricted DACL; Windows ignores WRITE_DAC under DENY-everyone style locks | **Replaced with AES-256-GCM encryption** (`file_locker.rs`) + ACL Recovery scanner to fix old damage |
| **`WINBIO_*` constants not found (cargo E0432)** | windows-sys 0.61 doesn't re-export `WINBIO_TYPE_FINGERPRINT`, `WINBIO_POOL_SYSTEM`, `WINBIO_FLAG_DEFAULT`, `WINBIO_ID_TYPE_SID` | Define them as raw `u32` constants in `biometric.rs` (SDK header values: 0x8, 0x1, 0x0, 3) |
| **`unlock_file/lock_folder` missing key arg (cargo E0061)** | encryption rewrite changed signatures; `drive_locker.rs` + `cmd_rescue_unlock` were never updated | Pass `key_material` at every call site (verified by search: 18 call sites) |
| **"File encryption key not available" after biometric/widget login** | `cmd_biometric_login` and `cmd_widget_unlock` never set `state.file_key` (only `cmd_unlock_session` did) | Set `state.file_key` + `set_active_file_key` in both commands |
| **Service re-locks fixed files at boot** | Service persists `locked_items` and re-applies `acl::apply_lock` at startup; ACL recovery fixed the ACL but never purged the list | Recovery commands (`cmd_force_unlock`, `cmd_recover_acl`, `cmd_bulk_recover_acl`) now call `notify_force_remove_locked_item` |
| **CI fails every push at ~1m40s** | "Build service daemon" step: PowerShell `copy` refuses to overwrite the committed `src-tauri/resources/omnilock-*.exe` → "An item … already exists" | `shell: bash` + `cp -f` in `build.yml` |
| **CI release step: "Resource not accessible by integration"** | Default `GITHUB_TOKEN` is read-only; creating a release needs `contents: write` | `permissions: contents: write` at the top of `build.yml` |
| Widget steals focus while user works | Monitor calls `widget.show()+set_focus()` every 2 s poll | `PROMPTED_FOLDERS` static — prompt once per folder (v0.0.34) |
| Biometric token loadable without fingerprint | Token was DPAPI-only; Hello prompt was client-side | `authenticate_biometric` runs inside `cmd_biometric_login` before loading token (v0.0.34) |
| Weather inaccurate | wttr.in bad for Bangladesh | Open-Meteo API |

---

## Build & Run Commands

```bash
# Dev mode (hot reload)
npm run tauri dev

# Production build (produces installer)
npm run tauri build

# TypeScript check only
npx tsc --noEmit

# Rust checks
cd src-tauri && cargo check && cd ..
cd service && cargo check && cd ..
```

---

## ⚠️ AUTO-UPDATER SIGNING KEY (CRITICAL — UNCHANGED)

**The signing key MUST match the public key embedded in the installed binary.** Never regenerate or overwrite it.

| Field | Value |
|-------|-------|
| **Public key ID** | `EDE58385BDE79B6F` |
| **Public key (base64)** | `RWRvm+e9hYPl7eUOcS2Q3cknhhVt06dE6IRPrbFNNE/CqnEDdfYs12Wy` |
| **Private key file** | `src-tauri/update.key` |
| **Backup copy** | `signing-keys/update-v0.1.0.key` |
| **Password** | `omnilock2026` |
| **tauri.conf.json** | `pubkey = "EDE58385BDE79B6F"` |

To sign a release manually (or in CI via secrets `TAURI_SIGNING_PRIVATE_KEY` / `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`):
```powershell
$key = (Get-Content -Raw "src-tauri\update.key").Trim()
$env:TAURI_SIGNING_PRIVATE_KEY = $key
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "omnilock2026"
cmd /c "echo. | npx tauri signer sign `"path\to\installer.exe`""
```

---

## Architecture (updated for v0.0.35)

```
omnilock/
├── src/                      # React frontend (Vite + Tailwind)
│   ├── components/
│   │   ├── auth/             # LoginScreen, SetupWizard, GitHubConnect
│   │   ├── layout/           # Sidebar, TopBar, Footer
│   │   ├── pages/            # Dashboard, SystemMonitor, AppLocker, Presets, Vault, Security, Diagnostics, History
│   │   ├── shared/           # Field, Toggle, StatusPill, Stat, SectionHeader
│   │   └── widget/           # UnlockWidget (separate Tauri window)
│   └── lib/tauri-bridge.ts   # Tauri invoke wrappers
├── src-tauri/                # Rust backend (Tauri)
│   └── src/
│       ├── lib.rs            # Tauri commands, AppState (file_key), UNLOCK_TARGET/WIDGET_TEMP_UNLOCKED/ACTIVE_FILE_KEY
│       ├── vault.rs          # Argon2id + AES-256-GCM vault
│       ├── auth.rs           # setup/unlock + file_encryption_key generation
│       ├── biometric.rs      # DIRECT WBF fingerprint (WinBio*), DPAPI token
│       ├── file_locker.rs    # AES-256-GCM encryption locking + ACL damage scanner/recovery
│       ├── drive_locker.rs   # NoDrives registry + drive-root encryption (design concern — see below)
│       ├── process_guard.rs  # app kill monitor + Explorer folder monitor (widget prompt, auto-relock)
│       ├── shell_context.rs  # context menu + .omnilock file association
│       └── ...               # totp, system_presets, installer_guard, watchdog, system_monitor, etc.
├── service/                  # Windows service (named-pipe ACL daemon — LEGACY, now only for password
│   │                         #   verification + legacy ACL items; encryption locks bypass it)
├── shared/                   # omnilock-shared protocol crate (SvcRequest/SvcResponse/LockedItem)
├── AGENTS.md                 # THIS FILE — session handoff
└── docs/                     # ARCHITECTURE, BUILD_SETUP, DEV_LOG, INSTRUCTION, PRD, SECURITY
```

---

## Versioning Scheme

Patch bumps: 0.0.35 → 0.0.36 → … → 0.1.0

## Version History (most recent first)

- **0.0.35 (UNRELEASED — working tree)** — AES-256-GCM encryption-based file/folder locking replaces destructive ACL locking; direct WBF fingerprint (no Windows Hello); ACL damage scanner + bulk recovery UI; widget temp-unlock + auto-relock; `.omnilock` file association + `--open-locked`; GitHub Release pipeline on tag push. Build-verified, NOT runtime-tested, NOT released.
- **0.0.34** — Widget focus-steal fix (prompt once per folder). Server-side Windows Hello enforcement in `cmd_biometric_login`. Signed installer + updated `latest.json`.
- **0.0.33** — File-unlock child ACL fix v3, `force_unlock`, widget capability fix, `sync_vault_to_service` wiring.
- **0.0.29** — Deadlock fix in `logger::init()`. Fallback updater endpoints.
- **0.0.25** — Diagnostics system + logger.rs (INTRODUCED the deadlock that hid the window).
- … earlier history in CHANGELOG.md

---

## Next Session: Continue from Here

### Where we left off
All v0.0.35 code changes are in the working tree and **compile clean**. The previous session died mid-way with usage limits; this session fixed the remaining compile errors, the two file-key session bugs, and the destructive service-notification problem, then verified builds. **Nothing is committed, no release exists.**

### Suggested next steps (in order)
1. **Run the app end-to-end on Windows** (you must do this on a real machine — I cannot):
   - `npm run tauri dev`
   - Setup wizard → create vault → verify `file_encryption_key` in vault
   - Lock a **test** file/folder (start small — see the drive warning below!) → confirm `.omnilock` blob created, original gone
   - Open the locked folder in Explorer → widget pops up → unlock with password → use files → close widget → verify auto re-lock
   - Fingerprint: enroll a fingerprint in Windows Settings → Accounts → Sign-in options (must be enrolled for the user account), then test biometric login — expect the WBF scan prompt, NOT a Windows Hello dialog
   - Diagnostics → ACL Recovery: scan a previously-broken path, Fix All, verify the service no longer re-locks at boot
   - Double-click a `.omnilock` file → OmniLock opens with unlock widget (tests file association + `--open-locked`)
2. **Fix anything the test surfaces**, then commit the v0.0.35 work (`git add` the 12 files).
3. **Release v0.0.35**: bump `version` in `src-tauri/Cargo.toml` + `package.json` + `tauri.conf.json` to `0.0.35`, commit, `git tag v0.0.35 && git push origin main --tags`. The CI workflow builds and creates the signed GitHub Release automatically (requires the 2 signing secrets in repo settings). After release, verify `latest.json`-style auto-update endpoint (check `tauri.conf.json` updater config) and download URL are live.
4. Update this file + CHANGELOG + DEV_LOG as you go (standing rule).

### Priority issues to investigate (known design concerns)
1. **DRIVE LOCK (resolved for now, more work optional)**: v0.0.35 drive lock = NoDrives hide only (whole-drive recursive encryption was removed as a data hazard). Direct path access (e.g. `D:\` in the address bar) still works — true blocking needs a kernel driver or per-user-selected subpath encryption. Decide whether that matters and implement if so.
2. **Temp-unlock does not survive an app restart**: if the app exits while a widget-unlocked item is open, files stay decrypted but the vault still lists them as locked → next login shows a pointless prompt and the files were unprotected in between. Recommended: on login, verify each locked_files/locked_folders entry and re-encrypt if its blobs are missing (or mark it unlocked). Do NOT auto-re-encrypt folders the user may be actively using — prompt first.
3. **Legacy service state**: old locked items may still be in the service's persisted `locked_items` (ProgramData) and get ACL-re-locked at boot. The ACL Recovery scanner purges them one by one; consider a one-time boot sweep that force-removes legacy items.
4. **`cmd_biometric_login`** does not set the USB-removal callback / drive monitor wiring that `cmd_unlock_session` does (minor parity gap).
5. **App locking = encrypting the .exe** (`cmd_add_locked_app`/`cmd_toggle_locked_app` call `lock_file` on the exe path): works only if the app isn't running, and makes the installed program data unreadable until unlocked. Verify this matches intent (process-kill + exe encryption is a heavy "app lock").
6. **`.omnilock` blobs have no recovery if the vault is lost** — the key is inside the encrypted vault. Document this trade-off to the user (recovery = vault backup + recovery key flow).
7. **`check_biometric_available` counts sensors, not enrollments** — fingerprint login may be offered before any fingerprint is enrolled for the user. Optionally check `WinBioEnumEnrollments` for the current SID.
8. **`ACTIVE_FILE_KEY` intentionally stays in memory while logged out** — the widget temp-unlock/auto-relock model needs it (the widget works from the login screen). If stronger hygiene is wanted, clear it when no widget target is pending and no session exists (design trade-off).

### Feature backlog
| Feature | Priority | Notes |
|---------|----------|-------|
| Vault "add files to vault" storage (user #2) | High | Vault currently only stores lock lists, not arbitrary files |
| Hide files/folders/drives (user #3) | Medium | Partial via NoDrives; folder hiding not implemented |
| Auto-relock after idle | Medium | Idle detection exists; re-lock trigger wired via `auto_lock` |
| Shell context menu (right-click lock/unlock) | Done | `shell_context.rs` + `.omnilock` association |
| Cloud Files API Status column | Low | WinRT `StorageProviderItemProperties.SetAsync`, needs MSIX |
| File version backup before lock | Low | `backup_path_for` helper retained, unused |
| Lock history / audit UI | Low | Logger data exists, History page exists |
