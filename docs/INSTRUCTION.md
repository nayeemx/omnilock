# OmniLock - Windows 11 App, Folder & File Locker
## AI Developer Execution Blueprint & Technical Instruction Guide

**Document Version:** 2.0.0
**Product Name:** OmniLock
**Developer / Publisher:** InnologyBD
**Target Execution Environment:** Rust 1.97+, Tauri v2, Node.js v26+, Windows SDK (Win32 API), MSVC 2022

---

## 1. START HERE — Reading Order for New Sessions
When you open this project after a break, read these in this exact order:

1. **AGENTS.md** (root) — Project identity, build commands, DO NOT rules, feature status matrix
2. **CHANGELOG.md** (root) — What changed in the current version
3. **docs/DEV_LOG.md** (docs/) — Issues encountered and how they were solved, feature tracker
4. **docs/ARCHITECTURE.md** (docs/) — System design, IPC commands, migration policy
5. **docs/INSTRUCTION.md** (this file) — How to add new features
6. **docs/SECURITY.md** (docs/) — Encryption details

---

## 2. Directory Structure (actual as of 2026-07-26)

```
omnilock/
├── AGENTS.md                     # AI agent master instructions (START HERE)
├── CHANGELOG.md                  # Version history
├── docs/
│   ├── ARCHITECTURE.md           # System architecture (v2.0.0)
│   ├── DEV_LOG.md                # Issues + solutions
│   ├── INSTRUCTION.md            # This file
│   ├── SECURITY.md               # Encryption & threat model
│   ├── PRD.md                    # Product requirements
│   └── BUILD_SETUP.md            # Environment setup script
├── src/
│   ├── App.tsx                   # Root routing (~85 lines)
│   ├── main.tsx
│   ├── index.css                 # oklch design tokens, glass utilities
│   ├── index.html
│   ├── components/
│   │   ├── types.ts              # TabId, SetupStep, shared constants
│   │   ├── auth/
│   │   │   ├── LoginScreen.tsx   # Login + password reset/recovery
│   │   │   └── SetupWizard.tsx   # 3-step first-launch wizard
│   │   ├── layout/
│   │   │   ├── Sidebar.tsx
│   │   │   ├── TopBar.tsx
│   │   │   └── Footer.tsx
│   │   ├── pages/
│   │   │   ├── AppLockerPage.tsx
│   │   │   ├── PresetsPage.tsx
│   │   │   ├── VaultPage.tsx
│   │   │   └── SecurityPage.tsx
│   │   └── shared/
│   │       ├── Field.tsx
│   │       ├── Toggle.tsx
│   │       ├── SectionHeader.tsx
│   │       ├── Stat.tsx
│   │       └── StatusPill.tsx
│   └── lib/
│       └── tauri-bridge.ts       # All Tauri IPC wrappers
└── src-tauri/
    ├── Cargo.toml
    ├── tauri.conf.json           # Window 1280×800, CSP allows data: URIs
    ├── build.rs
    ├── app.manifest              # UAC admin manifest
    └── src/
        ├── main.rs               # Tauri runner
        ├── lib.rs                # All IPC handlers + app runner
        ├── vault.rs              # Argon2id+AES-GCM, recovery, migration
        ├── auth.rs               # Setup, unlock, answer verification
        ├── totp.rs               # RFC 6238 TOTP with QR generation
        ├── process_guard.rs      # Process enumeration + real suspension
        ├── system_presets.rs     # All 6 presets + installer guard
        ├── installer_guard.rs    # MSI/setup process killer
        ├── panic_hotkey.rs       # Win+Alt+L hotkey via raw FFI
        ├── file_locker.rs        # icacls DACL locking
        ├── drive_locker.rs       # DACL + NoDrives registry
        ├── watchdog.rs           # Self-monitoring (no guard binary)
        ├── models.rs             # All DTOs (VaultConfig, VaultRecoveryData, etc.)
        └── main.rs               # Backup entry point (see lib.rs)
```

---

## 3. Build & Execution

### TypeScript Check (fast, catches frontend errors)
```powershell
cd D:\projects\code\omnilock
npx tsc --noEmit
```
Must produce zero errors. If there are errors, fix them before proceeding.

### Rust Check (medium, catches backend errors)
```powershell
cd D:\projects\code\omnilock\src-tauri
cargo check
```
Must produce zero errors. If there are errors, read them carefully — Rust gives exact file:line for every issue.

### Full Production Build (slow, outputs all bundles)
```powershell
cd D:\projects\code\omnilock
npx tauri build
```
Output products in `src-tauri/target/release/bundle/`:
- `OmniLock_1.0.0_x64_en-US.msi` — Windows installer
- `OmniLock_1.0.0_x64-setup.exe` — NSIS installer
- `OmniLock_1.0.0_x64_portable.zip` — Portable zip

---

## 4. Adding a New Feature (step-by-step)

### Step 1 — Rust Backend Module
Create `src-tauri/src/<your_module>.rs`. Implement the functionality with proper error handling (`Result<T, String>` everywhere).

In `src-tauri/src/lib.rs`:
- Add `pub mod <your_module>;` to the module declarations
- Add a `#[tauri::command]` function that calls your module's logic
- Register the command in the `invoke_handler!` macro

If you need a new crate, add it to `src-tauri/Cargo.toml` and run `cargo check` to verify it compiles.

### Step 2 — Tauri Bridge
Add typed async function(s) to `src/lib/tauri-bridge.ts` using `invoke()` from `@tauri-apps/api/core`.

Do NOT export unused functions. Every export must have a caller in the frontend.

### Step 3 — Frontend Component
Create a React component under the appropriate subdirectory in `src/components/`.

- Import the bridge function(s) you need
- Handle every error with inline UI feedback (an error message div, not just `console.error`)
- After successful operations, show a success message
- If the feature modifies vault state, call `refresh()` after success to reload config

### Step 4 — Wire Up in App.tsx
Add the import and render the component in the appropriate tab section of the router in `src/App.tsx`.

### Step 5 — Update Documentation
- Update `docs/ARCHITECTURE.md` IPC command table with any new commands
- Add entry to `CHANGELOG.md`
- Add issue/solution entry to `docs/DEV_LOG.md` if any unexpected issues occurred

### Step 6 — Verify Everything Works
```powershell
npx tsc --noEmit && cd src-tauri && cargo check && cd .. && npx tauri build
```
All three commands must succeed with zero errors. Only then is the feature considered done.

---

## 5. Health Check Procedure
After any session or when things feel wrong, run this:

```powershell
cd D:\projects\code\omnilock
echo "=== TypeScript Check ==="
npx tsc --noEmit
echo "=== Rust Check ==="
cd src-tauri && cargo check
echo "=== Full Build ==="
cd .. && npx tauri build
echo "=== All checks complete ==="
```

If Rust check fails, read the error — it will tell you the exact file and line. If TypeScript fails, the error message is equally precise.

---

## 6. Key Architectural Decisions
1. **Vault location:** `%APPDATA%\InnologyBD\OmniLock\` — survives MSI/NSIS reinstalls. Instal ler only overwrites Program Files.
2. **Encryption key:** Argon2id(password, salt) — per-vault salt in EncryptedVault header. NOT SHA-256(password).
3. **Recovery:** Separate `vault.recovery` file, answer-hash encrypted. NOT in vault.enc.
4. **Versioning:** `EncryptedVault.version: u32` + `vault.meta` tracks vault version. Migration on unlock.
5. **Single process:** No external guard binary. Watchdog thread monitors from within.
6. **All commands return Result<_, String>** — errors always contain human-readable messages.
7. **Error feedback:** Every backend call shows success or error in the UI. No silent catch blocks.
8. **Controlled components:** Toggles and inputs with backend state are driven by props, not local useState.
9. **No tokio:** All async uses `std::thread::spawn`. tokio is in Cargo.toml but unused.
10. **No hardcoded values:** Daemon status, stats, versions come from backend/state.

---

## 7. DO NOT — Rules That Break Things
1. Do not put all code in App.tsx.
2. Do not modify `tauri-bridge.ts` exports without checking `App.tsx` imports.
3. Do not use `SHA-256(password)` for vault encryption — use `vault::hash_password()`.
4. Do not add commands to `lib.rs` without also updating `tauri-bridge.ts` AND `App.tsx`.
5. Do not hardcode daemon status, stats, version strings, or feature flags.
6. Do not use `verifyTotp` or `SessionToken` from `tauri-bridge.ts` — dead exports.
7. Do not touch `EncryptedVault` struct without updating `migrate_vault_if_needed()`.
8. Do not use `tokio` in Rust backend — it's declared but unused.
9. Do not remove `windows-sys` features needed by existing commands.
10. Do not store recovery data inside vault file — it must be in `vault.recovery`.
11. Do not use empty `catch {}` blocks — every call must show feedback to the user.
12. Do not name local state variables the same as imported bridge functions.

---

## 8. Troubleshooting
| Symptom | Likely Cause | Fix |
|:---|:---|:---|
| Rust build fails with missing function | windows-sys version doesn't export the API | Use raw FFI with GetProcAddress from user32.dll |
| Portable exe runs slowly | Windows Defender scanning unsigned binary | Add exclusion or code-sign |
| Vault not found | Vault is in AppData, not Program Files | Check `%APPDATA%\InnologyBD\OmniLock\vault.enc` exists |
| QR code not showing | TOTP secret not generated before QR call | Ensure generateTotpSecret() then generateTotpQr() called in order |
| Toggle not persisting state | Local state shadowing controlled prop | Ensure Toggle receives `on` prop from parent |
| Password reset fails | vault.recovery file missing or answer wrong | Recovery file only created in Setup Wizard |
| Process suspend fails | App not running as Administrator | Requires admin to open process handles for SuspendThread |
| Build takes very long | Normal for first Rust compile subsequent builds are cached | Wait for `Compiling omnilock v1.0.0` progress |

---

*End of INSTRUCTION.md (OmniLock v2.0.0)*
