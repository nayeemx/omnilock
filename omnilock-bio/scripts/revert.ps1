# Restore the original Synaptics WBF driver (WUDFRd / Windows Hello) for the
# fingerprint sensor. MUST be run as Administrator.

$ErrorActionPreference = "Stop"

if (-not ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    Write-Host "Not elevated. Relaunching as Administrator..." -ForegroundColor Yellow
    Start-Process powershell -Verb RunAs -ArgumentList "-NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`""
    exit
}

Write-Host "Removing the omnilock-bio WinUSB binding..." -ForegroundColor Cyan
$oem = pnputil /enum-drivers | Out-String
$match = [regex]::Match($oem, "oem\d+\.inf[\s\S]*?omnilock-sensor\.inf", [System.Text.RegularExpressions.RegexOptions]::IgnoreCase)
if ($match.Success) {
    $infName = [regex]::Match($match.Value, "oem\d+\.inf").Value
    Write-Host "Found installed driver: $infName" -ForegroundColor Cyan
    pnputil /delete-driver $infName /uninstall
    Write-Host "Driver removed. Rescanning for hardware..." -ForegroundColor Cyan
    pnputil /scan-devices
    Start-Sleep -Seconds 3
} else {
    Write-Host "No omnilock-sensor.inf driver found in the driver store." -ForegroundColor Yellow
}

Write-Host "`nSensor state:" -ForegroundColor Cyan
Get-PnpDevice | Where-Object { $_.InstanceId -match "VID_138A&PID_00AB" } | Format-List InstanceId, FriendlyName, Status, Class, Service

$svc = (Get-PnpDevice | Where-Object { $_.InstanceId -match "VID_138A&PID_00AB" -and $_.Status -eq "OK" } | Select-Object -First 1 -ExpandProperty Service)
if ($svc -ne "WinUSB") {
    Write-Host "`nOK: sensor is back under the WBF stack (Service=$svc)." -ForegroundColor Green
} else {
    Write-Host "`nStill bound to WinUSB. If Windows did not auto-reinstall the Synaptics driver, open Device Manager, uninstall the device (check 'Delete the driver software'), then scan for hardware changes." -ForegroundColor Yellow
}