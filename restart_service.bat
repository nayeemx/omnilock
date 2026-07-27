@echo off
echo ==========================================
echo  OmniLock Service Restart
echo  Run as Administrator:
echo    Right-click this file ^> Run as administrator
echo ==========================================
echo.
echo Stopping OmniLockService...
sc stop OmniLockService
timeout /t 3 /nobreak >nul

echo Starting OmniLockService...
sc start OmniLockService
timeout /t 2 /nobreak >nul

echo.
sc query OmniLockService
echo.
echo Done! You can close this window.
pause
