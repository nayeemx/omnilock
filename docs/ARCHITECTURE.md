# OmniLock - Windows 11 App, Folder & File Locker
## System Architecture & Technical Specification

**Document Version:** 1.2.0  
**Product Name:** OmniLock  
**Developer / Publisher:** InnologyBD  
**Target Platform:** Windows 11 x64  
**Primary Stack:** Rust (Backend Guard & Daemon), Tauri v2 (IPC & Native Shell), React 18 / TypeScript (Frontend Dashboard)  

---

## 1. Architectural Overview & System Topology

OmniLock adopts a **Defense-in-Depth, Dual-Process Watchdog Architecture**.

```
+-----------------------------------------------------------------------------------+
|                                  WINDOWS 11 OS                                    |
+-----------------------------------------------------------------------------------+
                                          |
     +------------------------------------+-----------------------------------+
     |                                                                        |
     v                                                                        v
+------------------------------------+                    +------------------------------------+
|  GUI & Dashboard Process           |                    |  Watchdog & Core Daemon Service    |
|  (omnilock.exe)                    |                    |  (omnilock-guard.exe)              |
|                                    |    IPC Pipe        |                                    |
|  +------------------------------+  |  (Named Pipe)      |  +------------------------------+  |
|  | System Lockdown Presets      |  |                    |  | Real-Time Process Monitor    |  |
|  +------------------------------+  |<==================>|  +------------------------------+  |
|  | Software Installer Guard     |  |                    |  | Windows DACL File & Drive    |  |
|  +------------------------------+  |                    |  +------------------------------+  |
|  | Panic Hotkey Listener        |  |                    |  | Argon2id / AES-256 Vault     |  |
|  +------------------------------+  |                    |  +------------------------------+  |
+------------------------------------+                    +------------------------------------+
     |                                                                        |
     +------------------------------------+-----------------------------------+
                                          |
                                          v
                  +----------------------------------------------+
                  |  Encrypted Vault & Config                    |
                  |  (%APPDATA%\InnologyBD\OmniLock\vault.enc)   |
                  +----------------------------------------------+
```

---

## 2. Component Subsystems Architecture

### 2.1 Core Vault (`omnilock-vault`)
- Encrypted file: `%APPDATA%\InnologyBD\OmniLock\vault.enc`
- Argon2id KDF + AES-256-GCM authenticated encryption.
- Header Magic: `OMNI` (4 Bytes).

### 2.2 Application Interception (`omnilock-process`)
- Real-time Win32 process enumeration & image path checking.
- Triple Indexing: Binary Path + Process Name + SHA-256 Hash.
- Thread suspension primitive: `NtSuspendProcess` dynamic binding.

### 2.3 System Presets & Installer Guard (`omnilock-presets`)
- Intercepts Task Manager (`taskmgr.exe`), Control Panel (`control.exe`), Registry Editor (`regedit.exe`), PowerShell/CMD, and Installer executables (`msiexec.exe`, `setup.exe`).

### 2.4 Panic Hotkey Engine (`omnilock-panic`)
- Win32 Hotkey listener (`Win + Alt + L`).
- Blank display canvas overlay + master audio output mute + 2FA session lock.

### 2.5 Dual-Process Watchdog (`omnilock-watchdog`)
- `omnilock.exe` and `omnilock-guard.exe` monitor each other via process handle synchronization.
- Surviving process restarts terminated instance within **< 20ms**.

---

## 3. Tauri IPC Command Specification

| Command Identifier | Arguments | Return Type | Description |
|:---|:---|:---|:---|
| `cmd_get_vault_status` | None | `VaultStatusDto` | Returns vault status & InnologyBD publisher metadata. |
| `cmd_setup_vault` | `SetupPayload` | `Result<(), String>` | Setup master password, security Q&A, TOTP 2FA. |
| `cmd_unlock_session` | `AuthPayload` | `Result<SessionToken, String>` | Authenticates Master Password and TOTP. |
| `cmd_toggle_system_preset` | `preset_id: String, enabled: bool` | `Result<(), String>` | Enables/disables system utility lock presets. |
| `cmd_toggle_installer_guard` | `enabled: bool` | `Result<(), String>` | Activates/deactivates software installer blocking. |
| `cmd_trigger_panic_lock` | None | `Result<(), String>` | Triggers emergency screen blanking and session lock. |
| `cmd_add_locked_drive` | `drive_letter: String` | `Result<(), String>` | Applies DACL lock & NoDrives policy to drive. |

---
*End of ARCHITECTURE.md (v1.2.0)*
