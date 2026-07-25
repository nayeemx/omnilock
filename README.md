# OmniLock

Windows 11 desktop security application by **InnologyBD**. Application, folder, file & drive locker with Argon2id key derivation, AES-256-GCM authenticated encryption, RFC 6238 TOTP 2FA, and a self-monitoring watchdog daemon.

## Quick Start

```powershell
# Check TypeScript
npx tsc --noEmit

# Check Rust
cd src-tauri && cargo check && cd ..

# Full build (outputs MSI, NSIS exe, portable zip)
npx tauri build
```

## Architecture

- **Frontend:** React 18 / TypeScript / Tailwind CSS (dark glassmorphism, oklch tokens)
- **Backend:** Rust (Tauri v2) — 13 modules: auth, vault, totp, process_guard, system_presets, installer_guard, panic_hotkey, file_locker, drive_locker, watchdog, models
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
| File & folder locking via Windows DACL (icacls) | ✅ |
| Drive locking (DACL + NoDrives registry) | ✅ |
| TOTP 2FA (RFC 6238, QR setup) | ✅ |
| Panic hotkey (Win+Alt+L) | ✅ |
| Auto-lock timer | ✅ |
| Self-monitoring watchdog (no external guard binary) | ✅ |
| Vault versioning + transparent migration | ✅ |
| Modular frontend (18 component files) | ✅ |

## Documentation

| File | Purpose |
|:---|:---|
| `AGENTS.md` | AI agent starter — reading order, DO NOT rules, health check |
| `CHANGELOG.md` | Version history |
| `docs/DEV_LOG.md` | Issues encountered and solutions |
| `docs/ARCHITECTURE.md` | System design and technical specification |
| `docs/INSTRUCTION.md` | How to add new features step-by-step |
| `docs/SECURITY.md` | Encryption model and threat analysis |

## Build Output

```
src-tauri/target/release/bundle/
├── msi/OmniLock_1.0.0_x64_en-US.msi
├── nsis/OmniLock_1.0.0_x64-setup.exe
└── (portable zip)
```

## Dev Environment

- Node.js v26+ / npm 11+
- Rust 1.97+ / Cargo 1.97+
- MSVC 2022
- Windows 10/11 x64
