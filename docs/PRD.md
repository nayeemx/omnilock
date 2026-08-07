# OmniLock - Windows 11 App, Folder & File Locker
## Product Requirements Document (PRD)

**Document Version:** 1.3.0  
**Product Name:** OmniLock  
**Developer / Publisher:** InnologyBD (Featured in About dialog & Footer)  
**Status:** Approved for Architectural Design & Implementation  
**Target Platform:** Windows 11 (64-bit / ARM64)  
**Core Technologies:** Rust, Tauri v2, React 18, Windows API (Win32/NT), Argon2id, AES-256-GCM (vault + file encryption), RFC 6238 TOTP, Windows Biometric Framework (direct fingerprint)  

> **Implementation note (v0.0.35):** file/folder protection is implemented with **AES-256-GCM in-place encryption** (`.omnilock` blobs), NOT Windows NT ACL lockdown — the ACL approach permanently removed the owner's access and was replaced. Windows NT ACLs remain only in the legacy service daemon for backward compatibility; new locks bypass it. See AGENTS.md.

---

## 1. Executive Summary & Product Vision

**OmniLock** is a sleek, ultra-modern desktop security application for Windows 11 developed by **InnologyBD**. Designed to provide mobile-like application, folder, file, and drive locking capabilities, OmniLock implements a **zero-trust, kernel/user-mode hybrid protection model**.

It features:
1. **Application Protection**: Deep process monitoring and window hooking that prevents unauthorized launching, execution, or interaction with specified `.exe` executables.
2. **System Lockdown Presets**: 1-click protection profiles for sensitive Windows utilities (Task Manager, Control Panel, Registry Editor, PowerShell, CMD, System Settings).
3. **Software Installer Guard**: Intercepts `msiexec.exe`, `setup.exe`, `install.exe`, and self-extracting installers to prevent unauthorized software installations.
4. **Folder, File & Drive Protection**: Advanced access control layer that locks directories, files, and entire drive volumes (`D:\`, `E:\`) using Windows NT ACL security descriptors.
5. **Emergency Panic Hotkey & Screen Blanking**: `Win + Alt + L` global shortcut that instantly blanks screen display, mutes system audio, and enforces session lockout.
6. **Multi-Factor Authentication (MFA/2FA)**: Mandatory or optional RFC 6238 TOTP support (compatible with Google Authenticator, Authy, Microsoft Authenticator, 1Password).
7. **Resilient Password Recovery**: Multi-tier password recovery workflow eliminating lockout risks without creating backdoor vulnerabilities.
8. **Anti-Bypass & Anti-Tamper Engine**: Dual-process watchdog daemon preventing unauthorized termination via Task Manager, process kill commands, executable renaming, or safe-mode exploitation.

---

## 2. Branding & Footer Metadata

- **Application Display Name**: **OmniLock**
- **Footer Text**: `Powered by InnologyBD © 2026. All rights reserved.`
- **About Window**: Displays "OmniLock v1.0.0 by InnologyBD - Enterprise Desktop Security Solution".

---

## 3. Functional Requirements (FR)

### 3.1 Authentication & Vault Infrastructure (FR-AUTH)

| ID | Requirement Description | Priority |
|:---|:---|:---|
| **FR-AUTH-01** | **Setup Wizard**: On first launch, force user to create a Master Password (min 8 chars, including uppercase, lowercase, numbers, and symbols). | **P0 (Critical)** |
| **FR-AUTH-02** | **Argon2id Password Hashing**: Master Password MUST be hashed using Argon2id with unique 16-byte random salt, high memory cost (64MB), and time cost (3 iterations). | **P0 (Critical)** |
| **FR-AUTH-03** | **AES-256-GCM Storage**: Config stored in encrypted vault (`vault.enc`) under `%APPDATA%\InnologyBD\OmniLock\` with AES-256-GCM. | **P0 (Critical)** |
| **FR-AUTH-04** | **Two-Factor Authentication (TOTP)**: RFC 6238 TOTP engine with QR code generator & 2FA enforcement. | **P0 (Critical)** |
| **FR-AUTH-05** | **Session Auto-Lock**: Configurable auto-lock timer (Immediate, 1 min, 5 min, 15 min, Screen Lock / Sleep). | **P0 (Critical)** |
| **FR-AUTH-06** | **Auto-Update**: Check for and install updates from GitHub Releases with signed artifacts. | **P1 (High)** |

### 3.2 System Lockdown Presets & Installer Guard (FR-SYS-PRESET)

| ID | Requirement Description | Priority |
|:---|:---|:---|
| **FR-SYS-04** | **System Lockdown Presets**: 1-click toggles to lock Task Manager (`taskmgr.exe`), Control Panel (`control.exe`), Registry Editor (`regedit.exe`), Command Prompt (`cmd.exe`), PowerShell (`powershell.exe`), and System Restore (`rstrui.exe`). | **P0 (Critical)** |
| **FR-SYS-05** | **Software Installer Guard**: Intercepts `msiexec.exe`, `setup.exe`, `install.exe`, and self-extracting archive installers, blocking unauthorized software installations. | **P0 (Critical)** |
| **FR-SYS-06** | **Emergency Panic Hotkey**: Global hotkey (`Win + Alt + L`) registered via Windows `RegisterHotKey`. Instantly blank screens, mute audio, and require 2FA authentication. | **P0 (Critical)** |

### 3.3 Folder, File & Drive Volume Protection (FR-FILE-VOL)

| ID | Requirement Description | Priority |
|:---|:---|:---|
| **FR-FILE-01** | **Folder & File Selection**: Add files or folders via Drag-and-Drop or Native Dialog. | **P0 (Critical)** |
| **FR-FILE-02** | **Drive Volume Protection**: Ability to lock entire drive volumes (e.g. `D:\`, `E:\`) via `NoDrives` Explorer policy (+ drive-root encryption — see AGENTS.md design concern). | **P0 (Critical)** |
| **FR-FILE-03** | **File/Folder Protection**: Encrypt file contents with AES-256-GCM in place (`original` → `original.omnilock`) so they are unreadable without the vault key; folder stays browsable. (Supersedes the old ACL lockdown, which permanently removed owner access.) | **P0 (Critical)** |

---

## 4. User Interface (UI) Specifications

- **App Name**: OmniLock
- **Theme**: Dark Glassmorphism aesthetic (`#09090B`), vibrant neon cyan/violet accents (`#06B6D4`, `#8B5CF6`), frosted glass blur panels (`backdrop-filter: blur(16px)`).
- **Footer**: `OmniLock Security System • Developed by InnologyBD`
- **Tabs**:
  1. **Application Locker**: Searchable app list, SHA-256 status, "Add App" auto-scanner.
  2. **System & Installer Presets**: 1-click preset switches for Task Manager, Control Panel, Registry, PowerShell/CMD, and Software Installers.
  3. **File, Folder & Drive Vault**: File tree view, Drive volume list (`D:\`, `E:\`), DACL status, Stealth toggle.
  4. **Security & 2FA Settings**: TOTP QR code manager, 3-Tier Recovery Key viewer, Panic Hotkey customizer, Auto-lock timer.

---
*End of PRD.md (OmniLock v1.2.0)*
