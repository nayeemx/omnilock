# OmniLock

Windows 11 desktop security application by **InnologyBD**. Application, folder, file & drive locker with Argon2id key derivation, AES-256-GCM authenticated encryption, RFC 6238 TOTP 2FA, and a self-monitoring watchdog daemon.

## Quick Start

```powershell
# Check TypeScript
npx tsc --noEmit

# Check Rust
cd src-tauri && cargo check && cd ..

# Full build (outputs MSI, NSIS exe, portable zip)
# For signed builds (required for auto-updates):
$env:TAURI_SIGNING_PRIVATE_KEY="<key content>"
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD="229689"
npx tauri build
```

## Architecture

- **Frontend:** React 18 / TypeScript / Tailwind CSS (dark glassmorphism, oklch tokens)
- **Backend:** Rust (Tauri v2) — modules: auth, vault, totp, process_guard, system_presets, installer_guard, panic_hotkey, biometric (Windows Hello fingerprint prompt + DPAPI token), file_locker (AES-256-GCM), vault_storage (encrypted file storage), drive_locker, watchdog, models
- **Vault:** `%APPDATA%\InnologyBD\OmniLock\` (survives reinstalls)
- **Target:** Windows 10/11 x64, requires admin (UAC manifest)

## Key Features

| Feature | Status |
|:---|:---|
| Vault encrypt/decrypt (Argon2id + AES-256-GCM) | ✅ |
| Password reset via security question (2FA wiped) | ✅ |
| Application process lock (SHA-256 hash verified) | ✅ |
| System preset lockdown (all 6 presets) | ✅ |
| Installer guard (blocks MSI/setup executables) | ✅ |
| File & folder locking via AES-256-GCM encryption (`.omnilock`) | ✅ v0.0.35 |
| Vault Storage — encrypted private file storage (add/extract/delete) | ✅ v0.0.36 |
| Stale-unlock prompt after restart (re-lock all / keep unlocked) | ✅ v0.0.36 |
| Legacy service ACL state purged at boot | ✅ v0.0.36 |
| Biometric login parity (auto-lock + USB-removal wiring) | ✅ v0.0.36 |
| Biometric availability checks enrolled fingerprints | ✅ v0.0.36 |
| ACL Recovery (repair files damaged by the old DACL lock) | ✅ v0.0.35 |
| Drive locking (NoDrives + encryption; design concern — see AGENTS.md) | ⚠️ |
| Fingerprint login via direct Windows Biometric Framework (no Windows Hello) | ✅ v0.0.35 |
| Widget temp-unlock + auto re-lock on close | ✅ v0.0.35 |
| `.omnilock` double-click → unlock widget | ✅ v0.0.35 |
| Signed GitHub Releases via tag push (CI) | ✅ v0.0.35 |
| TOTP 2FA (RFC 6238, base32, QR setup) | ✅ |
| Panic hotkey (Win+Alt+L) | ✅ |
| Auto-lock timer | ✅ |
| Self-monitoring watchdog (no external guard binary) | ✅ |
| Vault versioning + transparent migration | ✅ |
| Auto-update via GitHub Releases (signed) | ✅ |
| Modular frontend (18 component files) | ✅ |

## Auto-Update

OmniLock supports automatic updates via GitHub Releases:

1. Open OmniLock → Go to Security page
2. Click "Check for Updates"
3. If new version available → click "Install Update & Restart"
4. App downloads, installs, and restarts

For developers: updates must be signed with the Tauri signing key (`src-tauri/update.key`).

## Documentation

| File | Purpose |
|:---|:---|
| `AGENTS.md` | AI agent starter — reading order, DO NOT rules, health check |
| `CHANGELOG.md` | Version history |
| `docs/DEV_LOG.md` | Issues encountered and solutions |
| `docs/ARCHITECTURE.md` | System design and technical specification |
| `docs/INSTRUCTION.md` | How to add new features step-by-step |
| `docs/SECURITY.md` | Encryption model and threat analysis |
| `docs/BUILD_SETUP.md` | Environment setup |

## Build Output

```
src-tauri/target/release/
├── omniLock.exe                          # Portable
├── bundle/
│   ├── msi/OmniLock_1.0.0_x64_en-US.msi
│   ├── msi/OmniLock_1.0.0_x64_en-US.msi.sig
│   ├── nsis/OmniLock_1.0.0_x64-setup.exe
│   └── nsis/OmniLock_1.0.0_x64-setup.exe.sig
└── dist/OmniLock_1.0.0_x64.zip          # All-in-one package
```

## Dev Environment

- Node.js v26+ / npm 11+
- Rust 1.97+ / Cargo 1.97+
- MSVC 2022
- Windows 10/11 x64
