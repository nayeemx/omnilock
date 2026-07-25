# OmniLock - Windows 11 App, Folder & File Locker
## Build, Environment Setup & Deployment Specification

**Document Version:** 1.3.0  
**Product Name:** OmniLock  
**Developer / Publisher:** InnologyBD  
**Target Platform:** Windows 11 (x64 / ARM64)  

---

## 1. Automated 1-Click Environment Setup Script (`setup_environment.ps1`)

If your personal computer is missing **Rust**, **Node.js**, **Visual Studio C++ Build Tools**, or **WiX Toolchain**, simply run the automated PowerShell setup script included in `D:\nayeem\omnilock\docs\`:

```powershell
# Open PowerShell as Administrator and run:
Set-ExecutionPolicy Unrestricted -Scope Process -Force
.\setup_environment.ps1
```

### What `setup_environment.ps1` automatically does:
1. **Checks Node.js**: Verifies `node` & `npm`. If missing, installs Node.js LTS via `winget`.
2. **Checks Rust Toolchain**: Verifies `cargo` & `rustc`. If missing, automatically downloads `rustup-init.exe` and installs the `stable-x86_64-pc-windows-msvc` toolchain.
3. **Checks Visual C++ Build Tools**: Verifies MSVC compiler files. If missing, automatically triggers `winget install Microsoft.VisualStudio.2022.BuildTools`.
4. **Checks WiX / NSIS Bundlers**: Verifies installer packaging tools. If missing, installs WiX Toolchain via `winget`.

---

## 2. Windows UAC Manifest (`app.manifest`)

```xml
<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity
    version="1.0.0.0"
    processorArchitecture="*"
    name="com.innologybd.omnilock"
    type="win32"
  />
  <description>OmniLock - Windows 11 App, Folder and File Locker by InnologyBD</description>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v2">
    <security>
      <requestedPrivileges xmlns="urn:schemas-microsoft-com:asm.v3">
        <requestedExecutionLevel level="requireAdministrator" uiAccess="false" />
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}" />
    </application>
  </compatibility>
</assembly>
```

---

## 3. Tauri v2 Configuration (`src-tauri/tauri.conf.json`)

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OmniLock",
  "version": "1.0.0",
  "identifier": "com.innologybd.omnilock",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "OmniLock - Security Manager",
        "width": 1000,
        "height": 680,
        "resizable": false,
        "fullscreen": false,
        "decorations": true,
        "transparent": false,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' asset: https: data:; style-src 'self' 'unsafe-inline';"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ],
    "resources": [],
    "copyright": "Copyright © 2026 InnologyBD. All rights reserved.",
    "shortDescription": "OmniLock - Windows 11 App, Folder & File Locker by InnologyBD",
    "longDescription": "Enterprise-grade Windows 11 application, folder, file, and drive locking system by InnologyBD."
  }
}
```

---
*End of BUILD_SETUP.md (OmniLock v1.3.0)*
