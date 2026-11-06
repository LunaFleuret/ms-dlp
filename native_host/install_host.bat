@echo off
setlocal EnableDelayedExpansion

echo ===================================================
echo  ms-dlp Native Messaging Host Installer
echo ===================================================
echo.

set "SCRIPT_DIR=%~dp0"
set "WORKSPACE_DIR=%SCRIPT_DIR%..\"
set "MANIFEST_PATH=%SCRIPT_DIR%com.msdlp.downloader.json"
set "LAUNCHER_PATH=%SCRIPT_DIR%host_launcher.bat"

echo [1/3] Checking executable and dependencies...
if not exist "%WORKSPACE_DIR%ms-dlp.exe" (
    echo.
    echo [ERROR] ms-dlp.exe not found!
    echo Please compile the project first using "cargo build --release".
    pause
    exit /b 1
)
echo [OK] ms-dlp.exe is ready.
echo.

:: Escape backslashes for JSON
set "ESCAPED_LAUNCHER_PATH=%LAUNCHER_PATH:\=\\%"

echo [2/3] Generating host manifest...
(
    echo {
    echo   "name": "com.msdlp.downloader",
    echo   "description": "ms-dlp Native Messaging Host",
    echo   "path": "!ESCAPED_LAUNCHER_PATH!",
    echo   "type": "stdio",
    echo   "allowed_origins": [
    echo     "chrome-extension://pdaiillmonnbnkjfncclpkebijgalafg/",
    echo     "chrome-extension://okjhdfkigofpbbadadokbfdijknjocla/",
    echo     "chrome-extension://iblkegnjnjoclaokjhdfkigofpbbadad/"
    echo   ]
    echo }
) > "%MANIFEST_PATH%"

echo Manifest generated at: %MANIFEST_PATH%
echo.

echo [3/3] Registering Native Messaging Host to browsers...
:: Register to Google Chrome
echo Registering to Google Chrome...
reg add "HKCU\Software\Google\Chrome\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul
if !errorlevel! equ 0 (
    echo [OK] Registered to Chrome.
) else (
    echo [ERROR] Failed to register to Chrome.
)

:: Register to Microsoft Edge
echo Registering to Microsoft Edge...
reg add "HKCU\Software\Microsoft\Edge\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul
reg add "HKCU\Software\Microsoft\Edge Dev\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul
if !errorlevel! equ 0 (
    echo [OK] Registered to Edge.
) else (
    echo [ERROR] Failed to register to Edge.
)

:: Register to Brave
echo Registering to Brave...
reg add "HKCU\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul

:: Register to Perplexity Comet
echo Registering to Perplexity Comet...
reg add "HKCU\Software\Perplexity\Comet\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul

echo.
echo ===================================================
echo  Installation completed successfully!
echo ===================================================
echo Next steps:
echo 1. Open your browser extensions page (e.g., chrome://extensions or edge://extensions)
echo 2. Enable "Developer mode"
echo 3. Click "Load unpacked" and select the "extension" folder of this project.
echo ===================================================
pause
endlocal
