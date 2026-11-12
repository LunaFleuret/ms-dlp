@echo off
setlocal EnableDelayedExpansion

echo ===================================================
echo   ms-dlp Universal Setup & Browser Registration
echo ===================================================
echo.

set "ROOT_DIR=%~dp0"
set "BIN_DIR=%ROOT_DIR%bin"
set "HOST_DIR=%ROOT_DIR%native_host"
set "EXE_PATH=%ROOT_DIR%ms-dlp.exe"
set "MANIFEST_PATH=%HOST_DIR%\com.msdlp.downloader.json"
set "CALCULATED_ID=ijeapgljimeaenemndconhapinpmjmin"

:: [1/4] Verify base files and bundled binaries
echo [1/4] Verifying project files and bundled tools...

if not exist "%EXE_PATH%" (
    echo.
    echo [ERROR] ms-dlp.exe not found in "%ROOT_DIR%".
    echo Please make sure ms-dlp.exe is placed in this root folder.
    pause
    exit /b 1
)

set "EXT_DIR="
if exist "%ROOT_DIR%msdlp-extension" (
    set "EXT_DIR=%ROOT_DIR%msdlp-extension"
) else if exist "%ROOT_DIR%extension" (
    set "EXT_DIR=%ROOT_DIR%extension"
)

if "%EXT_DIR%"=="" (
    echo.
    echo [ERROR] Extension folder [msdlp-extension] not found.
    pause
    exit /b 1
)

if not exist "%HOST_DIR%" mkdir "%HOST_DIR%"

:: Check core tools in bin\
set "MISSING_TOOLS=0"
if not exist "%BIN_DIR%\yt-dlp.exe" set "MISSING_TOOLS=1"
if not exist "%BIN_DIR%\ffmpeg.exe" set "MISSING_TOOLS=1"

if "!MISSING_TOOLS!"=="1" (
    echo [WARNING] Some core tools were not found in bin\.
    echo           Please make sure yt-dlp.exe and ffmpeg.exe exist in bin\.
) else (
    echo   [OK] Core tools verified in bin\.
)
echo.

:: [2/4] Generate Native Messaging Host Manifest
echo [2/4] Generating native messaging host manifest...
set "ESCAPED_EXE_PATH=%EXE_PATH:\=\\%"

(
    echo {
    echo   "name": "com.msdlp.downloader",
    echo   "description": "ms-dlp Native Messaging Host",
    echo   "path": "!ESCAPED_EXE_PATH!",
    echo   "type": "stdio",
    echo   "allowed_origins": [
    echo     "chrome-extension://!CALCULATED_ID!/"
    echo   ]
    echo }
) > "%MANIFEST_PATH%"

echo   [OK] Manifest created at: %MANIFEST_PATH%
echo.

:: [3/4] Register Native Messaging Host to Web Browsers
echo [3/4] Registering host to web browsers...

:: Google Chrome
reg add "HKCU\Software\Google\Chrome\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
if !errorlevel! equ 0 (
    echo   * Google Chrome: Registered
) else (
    echo   * Google Chrome: Skipped
)

:: Microsoft Edge
reg add "HKCU\Software\Microsoft\Edge\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
reg add "HKCU\Software\Microsoft\Edge Dev\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
if !errorlevel! equ 0 (
    echo   * Microsoft Edge: Registered
) else (
    echo   * Microsoft Edge: Skipped
)

:: Brave Browser
reg add "HKCU\Software\BraveSoftware\Brave-Browser\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
if !errorlevel! equ 0 (
    echo   * Brave Browser: Registered
) else (
    echo   * Brave Browser: Skipped
)

:: Perplexity Comet
reg add "HKCU\Software\Perplexity\Comet\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
if !errorlevel! equ 0 (
    echo   * Perplexity Comet: Registered
) else (
    echo   * Perplexity Comet: Skipped
)
echo.

:: [4/4] Extension Load Guide
echo ===================================================
echo   Setup completed successfully!
echo ===================================================
echo.
echo Next steps to use ms-dlp in your browser:
echo 1. Open your browser extensions page:
echo      Chrome: chrome://extensions
echo      Edge  : edge://extensions
echo      Brave : brave://extensions
echo.
echo 2. Enable "Developer mode" (toggle in top-right corner).
echo.
echo 3. Click "Load unpacked" and select the extension folder:
echo      Folder: !EXT_DIR!
echo.
echo ===================================================
echo.

explorer.exe "%EXT_DIR%"
pause
endlocal
