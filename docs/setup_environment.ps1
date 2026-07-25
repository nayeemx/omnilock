# ============================================================================
# OmniLock by InnologyBD - Automated Windows Environment & Dependency Setup
# ============================================================================
# This script automatically checks your Windows PC for all required OmniLock
# build dependencies (Node.js, Rust/Cargo, MSVC C++ Build Tools, WiX/NSIS),
# and automatically downloads & installs any missing toolchain items.
# ============================================================================

[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"

Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host "       OmniLock (by InnologyBD) - Automated Environment Setup" -ForegroundColor Cyan
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host ""

# Helper to test command availability
function Test-CommandExists {
    param ([string]$Command)
    return [bool](Get-Command $Command -ErrorAction SilentlyContinue)
}

# 1. Check & Install Node.js
Write-Host "[1/4] Checking Node.js Environment..." -NoNewline
if (Test-CommandExists "node") {
    $nodeVer = node -v
    Write-Host " INSTALLED ($nodeVer)" -ForegroundColor Green
} else {
    Write-Host " MISSING" -ForegroundColor Yellow
    Write-Host "     ==> Installing Node.js LTS via winget..." -ForegroundColor Cyan
    if (Test-CommandExists "winget") {
        winget install OpenJS.NodeJS.LTS --silent --accept-package-agreements --accept-source-agreements
    } else {
        Write-Host "     [!] winget not found. Please install Node.js manually from https://nodejs.org/" -ForegroundColor Red
    }
}

# 2. Check & Install Rust Toolchain
Write-Host "[2/4] Checking Rust & Cargo Toolchain..." -NoNewline
if (Test-CommandExists "cargo") {
    $cargoVer = cargo --version
    Write-Host " INSTALLED ($cargoVer)" -ForegroundColor Green
} else {
    Write-Host " MISSING" -ForegroundColor Yellow
    Write-Host "     ==> Downloading and installing Rust (rustup-init.exe)..." -ForegroundColor Cyan
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupInstaller = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupInstaller
    Start-Process -FilePath $rustupInstaller -ArgumentList "-y --default-toolchain stable-x86_64-pc-windows-msvc" -Wait
    Remove-Item $rustupInstaller -ErrorAction SilentlyContinue
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
    Write-Host "     [+] Rust toolchain installed successfully!" -ForegroundColor Green
}

# 3. Check C++ Build Tools
Write-Host "[3/4] Checking Microsoft Visual C++ Build Tools..." -NoNewline
$vsPath1 = "C:\Program Files (x86)\Microsoft Visual Studio"
$vsPath2 = "C:\Program Files\Microsoft Visual Studio"
if ((Test-Path $vsPath1) -or (Test-Path $vsPath2)) {
    Write-Host " INSTALLED" -ForegroundColor Green
} else {
    Write-Host " MISSING" -ForegroundColor Yellow
    Write-Host "     ==> Installing Visual Studio Build Tools..." -ForegroundColor Cyan
    if (Test-CommandExists "winget") {
        winget install Microsoft.VisualStudio.2022.BuildTools --silent --override "--passive --config $env:TEMP\vsconfig"
    } else {
        Write-Host "     [!] Please install C++ Build Tools from https://visualstudio.microsoft.com/visual-cpp-build-tools/" -ForegroundColor Red
    }
}

# 4. Check WiX / NSIS Bundler Tools
Write-Host "[4/4] Checking WiX / NSIS Installer Bundler..." -NoNewline
if ((Test-CommandExists "light") -or (Test-CommandExists "makensis")) {
    Write-Host " INSTALLED" -ForegroundColor Green
} else {
    Write-Host " MISSING" -ForegroundColor Yellow
    Write-Host "     ==> Installing WiX Toolchain via winget..." -ForegroundColor Cyan
    if (Test-CommandExists "winget") {
        winget install WiXToolset.WiXToolset --silent --accept-package-agreements --accept-source-agreements
    } else {
        Write-Host "     [!] WiX not found. Install from https://wixtoolset.org/ or `choco install wix`" -ForegroundColor Yellow
    }
}

Write-Host ""
Write-Host "========================================================================" -ForegroundColor Cyan
Write-Host "       Environment Setup Complete! You can now build OmniLock." -ForegroundColor Green
Write-Host "========================================================================" -ForegroundColor Cyan
