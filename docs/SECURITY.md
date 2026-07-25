# OmniLock - Windows 11 App, Folder & File Locker
## Security Hardening & Threat Model Specification

**Document Version:** 1.2.0  
**Product Name:** OmniLock  
**Publisher:** InnologyBD  
**Security Standard:** Zero-Trust Enterprise Desktop Security  

---

## 1. STRIDE Threat Analysis Matrix

| Threat Category | Potential Attack Vector | Impact | OmniLock Architectural Mitigation |
|:---|:---|:---|:---|
| **Spoofing** | Attacker creates dummy process named `omnilock.exe` to impersonate IPC daemon. | High | **Dynamic Pipe Auth**: IPC over Windows Named Pipes validates binary signature, path, and token PID. |
| **Tampering** | User renames locked app (`telegram.exe` to `app.exe`) to bypass process filter. | Critical | **Triple Indexing**: Indexes apps by Path, Process Name, **AND SHA-256 Binary Hash**. |
| **Tampering** | Attacker edits `%APPDATA%\InnologyBD\OmniLock\vault.enc` on disk to unlock apps. | Critical | **Authenticated AES-256-GCM**: Tampering invalidates 128-bit authentication tag, rejecting vault. |
| **Information Disclosure** | Memory dump of OmniLock process extracted via Task Manager to scrape password. | Critical | **Zeroize Memory Containers**: Sensitive variables use Rust's `Zeroize` crate to overwrite memory buffers with `0x00`. |
| **Denial of Service** | Attacker executes `taskkill /F /IM omnilock.exe` to kill protection daemon. | Critical | **Dual-Process Watchdog Daemon**: `omnilock-guard.exe` monitors `omnilock.exe`. Surviving process revives it in <20ms. |
| **Elevation of Privilege** | Attacker runs CMD as Administrator to access locked folders/drives. | Critical | **Windows NT DACL Enforcement**: Locked folders explicitly revoke `GENERIC_ALL` for `EVERYONE`, `SYSTEM`, and `Administrators`. |

---
*End of SECURITY.md (v1.2.0)*
