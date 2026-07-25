# OmniLock - Windows 11 App, Folder & File Locker
## AI Developer Execution Blueprint & Technical Instruction Guide

**Document Version:** 1.2.0  
**Product Name:** OmniLock  
**Developer / Publisher:** InnologyBD  
**Target Execution Environment:** Rust 1.75+, Tauri v2, Node.js v20+/v25+, Windows SDK (Win32 API)  

---

## 1. Directory Structure Specification

```
omnilock/
├── docs/
│   ├── PRD.md
│   ├── ARCHITECTURE.md
│   ├── SECURITY.md
│   ├── INSTRUCTION.md
│   └── BUILD_SETUP.md
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── app.manifest
│   └── src/
│       ├── main.rs                 # Entry point & Tauri runner initialization
│       ├── lib.rs                  # Module registry & IPC command handlers
│       ├── vault.rs                # Argon2id + AES-256-GCM vault engine
│       ├── auth.rs                 # Master Password, Q&A, and Recovery Key
│       ├── totp.rs                 # RFC 6238 TOTP engine
│       ├── process_guard.rs        # Win32 Process Monitor & NtSuspendProcess hook
│       ├── system_presets.rs       # Task Manager, Control Panel, CMD/PowerShell lock presets
│       ├── installer_guard.rs      # MSI & Setup installer execution blocker
│       ├── panic_hotkey.rs         # Win+Alt+L global hotkey & audio mute listener
│       ├── file_locker.rs          # Windows NT DACL ACL modifier
│       ├── drive_locker.rs         # Drive volume protection & NoDrives policy
│       ├── watchdog.rs             # Dual-process health check daemon
│       └── models.rs               # Rust DTO structs
├── src/
│   ├── index.html
│   ├── main.tsx
│   ├── App.tsx                     # Main Router & Vault State Manager
│   ├── index.css                   # Glassmorphism theme
│   ├── components/
│   │   ├── SetupWizard.tsx         # First-launch setup
│   │   ├── LoginModal.tsx          # Password + 2FA OTP Prompt
│   │   ├── PasswordResetModal.tsx  # Q&A + Recovery Key password reset view
│   │   ├── AppLockerView.tsx       # Application Protection management dashboard
│   │   ├── PresetsView.tsx         # System Lockdown & Installer Guard controls
│   │   ├── FileLockerView.tsx      # Folder, File & Drive Vault manager
│   │   ├── SettingsView.tsx        # 2FA configuration, Auto-lock & About InnologyBD
│   │   ├── LockOverlay.tsx         # Intercepted app lock overlay
│   │   └── Footer.tsx              # "OmniLock Security • Powered by InnologyBD"
│   └── lib/
│       └── tauri-bridge.ts         # Strongly-typed IPC wrapper functions
└── package.json
```

---

## 2. Cargo Dependencies (`src-tauri/Cargo.toml`)

```toml
[package]
name = "omnilock"
version = "1.0.0"
authors = ["InnologyBD <support@innologybd.com>"]
edition = "2021"
description = "OmniLock - Windows 11 App, Folder & File Locker by InnologyBD"

[build-dependencies]
tauri-build = { version = "2.0.0", features = [] }

[dependencies]
tauri = { version = "2.0.0", features = ["tray-icon", "image-png"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.35", features = ["full"] }
argon2 = { version = "0.5", features = ["std", "zeroize"] }
aes-gcm = "0.10"
totp-rs = { version = "5.5", features = ["qr"] }
sha2 = "0.10"
zeroize = { version = "1.7", features = ["zeroize_derive"] }
getrandom = "0.2"
base64 = "0.21"
subtle = "2.5"
sysinfo = "0.30"
windows-sys = { version = "0.52", features = [
    "Win32_Foundation",
    "Win32_System_Threading",
    "Win32_System_ProcessStatus",
    "Win32_Security",
    "Win32_Security_Authorization",
    "Win32_Storage_FileSystem",
    "Win32_UI_WindowsAndMessaging"
] }
```

---

## 3. Build & Execution Checklist

### Step 0 — Automated Dependency Setup (Run First on Any New PC)
Before doing anything else, run the bundled bootstrapper script to auto-install all missing tools (Rust, Node.js, MSVC C++ Build Tools, WiX):

```powershell
# Open PowerShell as Administrator, navigate to docs/, then run:
Set-ExecutionPolicy Unrestricted -Scope Process -Force
.\setup_environment.ps1
```

The script will check and install each tool automatically. Once it reports **"Environment Setup Complete!"**, proceed to the steps below.

---

### Step 1 — Initialize Tauri Project
```bash
npm create tauri-app@latest omnilock -- --template react-ts
cd omnilock
```

### Step 2 — Install Frontend Dependencies
```bash
npm install lucide-react clsx
npm install -D tailwindcss postcss autoprefixer
npx tailwindcss init -p
```

### Step 3 — Replace Cargo.toml with OmniLock dependencies
Copy the `[dependencies]` block from Section 2 above into `src-tauri/Cargo.toml`.

### Step 4 — Generate All Rust Backend Modules
Implement each module under `src-tauri/src/` per the blueprints in this document and `ARCHITECTURE.md`:
- `vault.rs`, `auth.rs`, `totp.rs`
- `process_guard.rs`, `system_presets.rs`, `installer_guard.rs`
- `panic_hotkey.rs`, `file_locker.rs`, `drive_locker.rs`, `watchdog.rs`

### Step 5 — Generate React / TypeScript Frontend
Implement all Glassmorphic UI components listed in the Directory Structure (Section 1).

### Step 6 — Build Production Installer
```bash
npm run tauri build
```
Output: `src-tauri/target/release/bundle/msi/OmniLock_1.0.0_x64_en-US.msi`

---
*End of INSTRUCTION.md (OmniLock v1.3.0)*
