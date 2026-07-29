!include "nsExec.nsh"

Function .onInit
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /im omnilock-svc.exe 2>nul'
  nsExec::Exec '"$SYSDIR\taskkill.exe" /f /im omnilock-monitor.exe 2>nul'
  Sleep 300
FunctionEnd
