# OmniLock - Windows 11 App, Folder & File Locker
## Build, Environment Setup & Deployment Specification

**Document Version:** 2.0.0  
**Product Name:** OmniLock  
**Developer / Publisher:** InnologyBD  
**Target Platform:** Windows 11 (x64 / ARM64)  

---

## 1. Automated 1-Click Environment Setup Script (`setup_environment.ps1`)

If your personal computer is missing **Rust**, **Node.js**, **Visual Studio C++ Build Tools**, or **WiX Toolchain**, simply run the automated PowerShell setup script included in `D:\projects\code\omnilock\docs\`:

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
  "plugins": {
    "updater": {
      "pubkey": "<public key from src-tauri/update.key.pub>",
      "endpoints": [
        "https://api.github.com/repos/nayeemx/omnilock/releases/latest"
      ]
    }
  },
  "app": {
    "windows": [
      {
        "title": "OmniLock - Enterprise Desktop Security",
        "width": 1280,
        "height": 800,
        "minWidth": 1024,
        "minHeight": 680,
        "resizable": true,
        "fullscreen": false,
        "decorations": true,
        "transparent": false,
        "center": true
      }
    ],
    "security": {
      "csp": "default-src 'self'; img-src 'self' asset: https: data:; style-src 'self' 'unsafe-inline'; script-src 'self' 'unsafe-inline';"
    }
  },
  "bundle": {
    "active": true,
    "targets": ["msi", "nsis"],
    "createUpdaterArtifacts": true,
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/icon.ico"
    ],
    "resources": [],
    "copyright": "Copyright © 2026 InnologyBD. All rights reserved.",
    "shortDescription": "OmniLock - Enterprise Desktop Security by InnologyBD",
    "longDescription": "Zero-trust enterprise desktop security tool. Application, folder, file & drive locker with Argon2id, AES-256-GCM, TOTP 2FA, and dual-process watchdog. Windows 10/11."
  }
}
```

---

## 4. Signing Key Setup (Required for Auto-Updates)

### Generate Key Pair
```powershell
cd D:\projects\code\omnilock
npx tauri signer generate -w src-tauri\update.key
# Enter a password when prompted (e.g., 229689)
# Public key will be at src-tauri\update.key.pub
```

### Environment Variables for Signed Builds
```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content src-tauri\update.key -Raw
$env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "229689"
npx tauri build
```

### Key Files
- **Private key:** `src-tauri/update.key` (KEEP SECRET!)
- **Public key:** `src-tauri/update.key.pub` (goes in tauri.conf.json)
- **Signatures:** `.sig` files generated during signed builds

---

## 5. GitHub Releases Setup

### Create Release
```powershell
# Using GitHub CLI
gh release create v1.0.0 --repo nayeemx/omnilock \
  --title "OmniLock v1.0.0" \
  --notes "Release notes here" \
  --prerelease \
  src-tauri/target/release/omnilock.exe \
  src-tauri/target/release/bundle/msi/OmniLock_1.0.0_x64_en-US.msi \
  src-tauri/target/release/bundle/nsis/OmniLock_1.0.0_x64-setup.exe \
  src-tauri/target/release/bundle/msi/OmniLock_1.0.0_x64_en-US.msi.sig \
  src-tauri/target/release/bundle/nsis/OmniLock_1.0.0_x64-setup.exe.sig
```

### Update Flow
1. Bump version in `tauri.conf.json`
2. Run signed build with env vars set
3. Create GitHub Release with assets
4. Users click "Check for Updates" in Security page

---

*End of BUILD_SETUP.md (OmniLock v2.0.0)*
