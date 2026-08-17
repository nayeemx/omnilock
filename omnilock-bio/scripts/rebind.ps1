# Rebind the fingerprint sensor (VID_138A PID_00AB) from the WBF stack
# (WUDFRd, Windows Hello) to WinUSB so omnilock-bio can drive it directly.
#
# MUST be run as Administrator. Windows Hello fingerprint sign-in will stop
# working; revert with revert.ps1.

$ErrorActionPreference = "Stop"

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Not elevated. Relaunching as Administrator..." -ForegroundColor Yellow
    Start-Process powershell -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    exit
}

$inf = Join-Path $PSScriptRoot "..\windows\omnilock-sensor.inf"

Write-Host "Installing WinUSB driver for the fingerprint sensor..." -ForegroundColor Cyan
pnputil /add-driver $inf /install
if ($LASTEXITCODE -ne 0) {
    Write-Host "pnputil failed. Try: pnputil /add-driver `"$inf`" /install /force" -ForegroundColor Red
    exit 1
}

Start-Sleep -Seconds 2

Write-Host "`nSensor state:" -ForegroundColor Cyan
Get-PnpDevice | Where-Object { $_.InstanceId -match "VID_138A&PID_00AB" } | Format-List InstanceId, FriendlyName, Status, Class, Service

$svc = (Get-PnpDevice | Where-Object { $_.InstanceId -match "VID_138A&PID_00AB" -and $_.Status -eq "OK" } | Select-Object -First 1 -ExpandProperty Service)
if ($svc -eq "WinUSB") {
    Write-Host "`nOK: sensor bound to WinUSB. Run: omnilock-bio probe" -ForegroundColor Green
} else {
    Write-Host "`nWARNING: expected Service=WinUSB but found '$svc'. Try 'Scan for hardware changes' or run with /force." -ForegroundColor Yellow
}