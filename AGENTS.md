# OmniLock – AI Agent Instructions

> **START HERE** when joining this project after any break. Read in order.

## 🔴 What This Project Is
OmniLock is a Windows desktop security application (Tauri v2 + React 18/TypeScript + Rust). It locks apps, files, folders, and drives with Argon2id + AES-256-GCM encryption. It has a dark glassmorphism UI with 4 module tabs.

## 📋 Reading Order (follow this when you open the project)
1. **This file** (AGENTS.md) — what, where, how to build, DO NOT rules
2. **CHANGELOG.md** — what version we're at, what changed recently
3. **docs/DEV_LOG.md** — recent issues and their solutions (prevents repeating mistakes)
4. **docs/ARCHITECTURE.md** — system structure and design decisions
5. **docs/INSTRUCTION.md** — how to add new features step-by-step
6. **docs/SECURITY.md** — encryption details and threat model
7. **docs/PRD.md** — product requirements (original spec)
8. **docs/BUILD_SETUP.md** — environment setup (if available)

## Build Commands (exact, from repo root `D:\projects\code\omnilock`)
```powershell
# Check TypeScript only (fast, catches frontend errors)
npx tsc --noEmit

# Rust check only (medium, catches backend errors)
cd src-tauri && cargo check

# Full production build (slow, outputs MSI + NSIS + portable zip)
npx tauri build
```

Build output: `src-tauri/target/release/bundle/` (MSI, NSIS exe, portable zip)

## 🟢 Feature Completeness Matrix
| Feature | Status | Verified? | Notes |
|---|---|---|---|
| Vault encrypt/decrypt | ✅ Real | ✅ Build passes | Uses Argon2id-derived key |
| Vault recover (password reset) | ✅ Real | ⚠️ Not in-app tested | Recovery file logic added, no full E2E test run |
| Vault migration (version bump) | ✅ Real | ⚠️ New code — untested | Runs on unlock, transparent |
| 2FA setup/enable/disable | ✅ Real | ⚠️ QR display works | verify button flow wired |
| Process scan + lock/toggle/remove | ✅ Real | ⚠️ Not in-app tested | IPC wired, UI complete |
| System presets (all 6) | ✅ Real | ⚠️ Registry changes need admin | control_panel & system_restore newly added |
| Installer guard | ✅ Real | ⚠️ Not in-app tested | Polls and kills installer processes |
| File locker (icacls) | ✅ Real | ⚠️ Not in-app tested | ACL deny/grant Everyone |
| Drive locker (DACL + NoDrives) | ✅ Real | ⚠️ Not in-app tested | Combined ACL + registry |
| Panic hotkey (Win+Alt+L) | ✅ Real | ❌ Requires admin + testing | raw FFI with user32.dll |
| Process suspension | ✅ Real | ❌ Requires admin + testing | SuspendThread FFI |
| Watchdog (self-monitoring) | ✅ Real | ⚠️ Cannot easily test | Restarts main if dead |
| Password reset UI flow | ✅ Real | ✅ TypeScript compiles | Login screen has "Forgot password?" |
| Frontend modular structure | ✅ Done | ✅ 18 files, clean | No monolith |
| Inline error/success feedback | ✅ Done | ✅ All pages have it | No silent catch blocks |
| Controlled Toggle component | ✅ Done | ⚠️ Not in-app tested | Driven by parent `on` prop |
| 2FA flow (button→QR→verify) | ✅ Done | ⚠️ Not in-app tested | No bypass of enable dialog |
| Dark glassmorphism UI | ✅ Done | ✅ Visual | oklch tokens, cyan/violet accents |
| Sidebar + TopBar + 4 tabs | ✅ Done | ✅ Visual | w-72 sidebar, status pills |
| Portable EXE performance | ⚠️ Known issue | ❌ Slow due to Windows Defender | Unsigned binary gets scanned; code-signing needed |

## 🔴 Known Issues / Limitations (do not forget these)
1. **Portable exe runs slowly** — Windows Defender scans unsigned binary. Fix: code-sign or add exclusion in Windows Security.
2. **`windows-sys` 0.52 missing some APIs** — RegisterHotKey requires raw `GetProcAddress` FFI fallback.
3. **`tokio` and `zeroize`** declared in Cargo.toml but unused — safe to remove eventually.
4. **No integration tests** — all features are wired but none have been run through end-to-end in the running app.
5. **Panic hotkey + process suspension** need admin to actually function — they are implemented but untested in non-admin context.

## 🔧 Health Check (how to verify nothing is broken after changes)
```powershell
cd D:\projects\code\omnilock
npx tsc --noEmit        # TypeScript check — must have 0 errors
cd src-tauri
cargo check             # Rust check — must have 0 errors
cd ..
npx tauri build         # Full build — must produce all 3 bundle outputs
```
If any step fails, STOP and fix before proceeding. Do not continue building on errors.

## 🔧 What Was Last Built (session marker)
- **Build version:** 1.0.0 (as of build date in CHANGELOG.md)
- **Last full build:** `npx tauri build` succeeded, 0 errors, 0 warnings
- **Last change:** All 5 phases of documentation + backend + frontend work completed
- **Key files modified this session:** All docs files created/updated, backend modules fixed, frontend split

## 📐 DO NOT — Rules That Break Things
1. **DO NOT** put all code in App.tsx — it was split into 18 files for maintainability
2. **DO NOT** modify `src/lib/tauri-bridge.ts` exports without checking `App.tsx` imports — unused exports waste bundle space
3. **DO NOT** use `SHA-256(password)` for vault encryption — always use `vault::hash_password()` for key derivation
4. **DO NOT** add a command to `lib.rs` without also adding it to `tauri-bridge.ts` AND wiring it in `App.tsx` AND showing error/success to the user
5. **DO NOT** hardcode daemon status, stats, version strings — get from backend/state/config
6. **DO NOT** use `verifyTotp` or `SessionToken` from `tauri-bridge.ts` — they were removed (dead exports)
7. **DO NOT** touch `EncryptedVault` struct without updating `migrate_vault_if_needed()` in vault.rs
8. **DO NOT** use `tokio` in Rust backend — all threading uses `std::thread::spawn` (tokio declared but unused)
9. **DO NOT** remove `windows-sys` features needed by existing commands — if you need a new Win32 API, check if its feature flag exists
10. **DO NOT** store recovery data inside the vault file — it must be in a separate `vault.recovery` file
11. **DO NOT** use empty `catch {}` blocks — every backend call must surface success or error to the user
12. **DO NOT** name local React state the same as imported bridge functions (e.g., don't call local state `setAutoLock` when `setAutoLock()` is also the IPC function)

## 📐 Adding a New Feature
1. **Rust backend:** Create `src-tauri/src/<module>.rs`, implement logic, add `mod <module>` to `lib.rs`, add `#[tauri::command]` handler, register in `invoke_handler!`, add new crates to `Cargo.toml` if needed
2. **Tauri bridge:** Add typed `async function` to `src/lib/tauri-bridge.ts` using `invoke()`
3. **Frontend component:** Create React component under `src/components/<category>/`, import bridge function, handle every error with inline UI feedback (never silent catch)
4. **Wire up:** Add component import to `src/App.tsx`, add to router render section
5. **Docs:** Update `docs/ARCHITECTURE.md` IPC table, add entry to `CHANGELOG.md`, add issue/solution to `docs/DEV_LOG.md`
6. **Verify:** `npx tsc --noEmit && cd src-tauri && cargo check && cd .. && npx tauri build`

## 📂 Vault & Data Safety
- **Vault location:** `%APPDATA%\InnologyBD\OmniLock\`
- **Files in vault directory:** `vault.enc` (encrypted config), `vault.recovery` (encrypted recovery data), `vault.meta` (totp_version config)
- **Installers do NOT touch AppData** — MSI/NSIS only overwrite Program Files. Vault is always safe across reinstalls.
- **Version upgrades** happen transparently on first unlock via `migrate_vault_if_needed()`
- **Never change** encryption params (Argon2id cost, salt size, nonce size) without bumping vault version number

## 🚨 If Things Break — Recovery Guide
1. **Build fails with Rust errors:** Run `cd src-tauri && cargo check` to see exact errors. Read the error message — Rust tells you exactly what's wrong (missing import, type mismatch, lifetime error).
2. **Build fails with TypeScript errors:** Run `npx tsc --noEmit` — errors line numbers are exact. Fix type errors first.
3. **App won't start / crashes:** Check `vault.enc` exists. If missing, run Setup Wizard. If corrupt, password reset is needed.
4. **Features don't work in UI:** The UI is fully wired — if something renders but does nothing, it's either (a) missing bridge function in tauri-bridge.ts, (b) missing command in lib.rs, (c) the backend function returned an error that's not shown on screen.
5. **Password reset fails:** Check that `vault.recovery` exists. It's only created during first setup. If missing, no password reset is possible (setup new vault).
6. **Portable exe slow:** Windows Defender scanning unsigned binary. Not a code issue.

Last updated: 2026-07-26
