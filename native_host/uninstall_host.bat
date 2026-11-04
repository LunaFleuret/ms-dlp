@echo off
setlocal

echo Unregistering ms-dlp Native Messaging Host...

reg delete "HKCU\Software\Google\Chrome\NativeMessagingHosts\com.msdlp.downloader" /f >nul 2>&1
reg delete "HKCU\Software\Microsoft\Edge\NativeMessagingHosts\com.msdlp.downloader" /f >nul 2>&1

set "MANIFEST_PATH=%~dp0com.msdlp.downloader.json"
if exist "%MANIFEST_PATH%" (
    del "%MANIFEST_PATH%"
)

echo [OK] Unregistered successfully.
pause
endlocal
