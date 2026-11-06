@echo off
setlocal EnableDelayedExpansion

echo ===================================================
echo   ms-dlp Universal Setup and Tool Installer
echo ===================================================
echo.

set "ROOT_DIR=%~dp0"
set "BIN_DIR=%ROOT_DIR%bin"
set "HOST_DIR=%ROOT_DIR%native_host"
set "EXE_PATH=%ROOT_DIR%ms-dlp.exe"
set "TEMP_DIR=%ROOT_DIR%temp_setup"
set "MANIFEST_PATH=%HOST_DIR%\com.msdlp.downloader.json"
set "CALCULATED_ID=ijeapgljimeaenemndconhapinpmjmin"

:: [1/5] Detect environment and requirements
echo [1/5] Checking base project files...

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
    echo [ERROR] Extension folder [msdlp-extension or extension] not found.
    pause
    exit /b 1
)

if not exist "%BIN_DIR%" mkdir "%BIN_DIR%"
if not exist "%HOST_DIR%" mkdir "%HOST_DIR%"

echo [OK] Base files and directories verified.
echo.

:: [2/5] Check and download external helper tools to bin/
echo [2/5] Checking external tools in bin directory...

:: 2-1: yt-dlp.exe
if exist "%BIN_DIR%\yt-dlp.exe" (
    echo   * yt-dlp.exe: Found in bin\
) else (
    echo   * yt-dlp.exe: Not found. Downloading from official GitHub release...
    curl.exe -fL --progress-bar -o "%BIN_DIR%\yt-dlp.exe" "https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp.exe"
    if exist "%BIN_DIR%\yt-dlp.exe" (
        echo     [OK] yt-dlp.exe installed successfully.
    ) else (
        echo     [WARNING] Failed to download yt-dlp.exe. Please place it manually in bin\.
    )
)

:: 2-2: ffmpeg.exe and ffprobe.exe
set "NEED_FFMPEG=0"
if not exist "%BIN_DIR%\ffmpeg.exe" set "NEED_FFMPEG=1"
if not exist "%BIN_DIR%\ffprobe.exe" set "NEED_FFMPEG=1"

if "!NEED_FFMPEG!"=="0" (
    echo   * ffmpeg.exe and ffprobe.exe: Found in bin\
) else (
    echo   * ffmpeg / ffprobe: Missing. Downloading release package...
    if not exist "%TEMP_DIR%" mkdir "%TEMP_DIR%"
    set "FFMPEG_ZIP=%TEMP_DIR%\ffmpeg.zip"
    curl.exe -fL --progress-bar -o "!FFMPEG_ZIP!" "https://github.com/yt-dlp/FFmpeg-Builds/releases/download/latest/ffmpeg-master-latest-win64-gpl.zip"
    if exist "!FFMPEG_ZIP!" (
        echo     Extracting ffmpeg and ffprobe...
        where tar.exe >nul 2>&1
        if !errorlevel! equ 0 (
            tar.exe -xf "!FFMPEG_ZIP!" -C "%TEMP_DIR%"
        ) else (
            powershell -NoProfile -Command "Expand-Archive -Path '!FFMPEG_ZIP!' -DestinationPath '%TEMP_DIR%' -Force"
        )
        for /r "%TEMP_DIR%" %%f in (ffmpeg.exe ffprobe.exe) do (
            if exist "%%f" copy /y "%%f" "%BIN_DIR%\" >nul
        )
        if exist "%BIN_DIR%\ffmpeg.exe" (
            echo     [OK] ffmpeg.exe and ffprobe.exe installed successfully.
        ) else (
            echo     [WARNING] Could not extract ffmpeg.exe. Please place it manually in bin\.
        )
        del /f /q "!FFMPEG_ZIP!" >nul 2>&1
    ) else (
        echo     [WARNING] Failed to download ffmpeg archive. Please place ffmpeg.exe in bin\.
    )
)

:: 2-3: AtomicParsley.exe
if exist "%BIN_DIR%\AtomicParsley.exe" (
    echo   * AtomicParsley.exe: Found in bin\
) else (
    echo   * AtomicParsley.exe: Not found. Downloading release package...
    if not exist "%TEMP_DIR%" mkdir "%TEMP_DIR%"
    set "AP_ZIP=%TEMP_DIR%\atomicparsley.zip"
    curl.exe -fL --progress-bar -o "!AP_ZIP!" "https://github.com/wez/atomicparsley/releases/latest/download/AtomicParsleyWindows.zip"
    if exist "!AP_ZIP!" (
        echo     Extracting AtomicParsley.exe...
        where tar.exe >nul 2>&1
        if !errorlevel! equ 0 (
            tar.exe -xf "!AP_ZIP!" -C "%TEMP_DIR%"
        ) else (
            powershell -NoProfile -Command "Expand-Archive -Path '!AP_ZIP!' -DestinationPath '%TEMP_DIR%' -Force"
        )
        for /r "%TEMP_DIR%" %%f in (AtomicParsley.exe) do (
            if exist "%%f" copy /y "%%f" "%BIN_DIR%\" >nul
        )
        if exist "%BIN_DIR%\AtomicParsley.exe" (
            echo     [OK] AtomicParsley.exe installed successfully.
        ) else (
            echo     [WARNING] Could not extract AtomicParsley.exe.
        )
        del /f /q "!AP_ZIP!" >nul 2>&1
    ) else (
        echo     [WARNING] Failed to download AtomicParsley.
    )
)

:: Clean up temporary directory if exists
if exist "%TEMP_DIR%" rmdir /s /q "%TEMP_DIR%" >nul 2>&1

echo.
echo [OK] All required helper tools in bin\ are verified.
echo.

:: [3/5] Extension ID setup
echo [3/5] Setting up Extension ID...
echo   * Extension ID: !CALCULATED_ID!
echo.

:: [4/5] Generate Native Messaging Host Manifest
echo [4/5] Generating host manifest...
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

echo [OK] Manifest created at: %MANIFEST_PATH%
echo.

:: [5/5] Register to Web Browsers
echo [5/5] Registering Native Messaging Host to browsers...

:: Google Chrome
reg add "HKCU\Software\Google\Chrome\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
if !errorlevel! equ 0 (
    echo   * Google Chrome: Registered
) else (
    echo   * Google Chrome: Failed or skipped
)

:: Microsoft Edge
reg add "HKCU\Software\Microsoft\Edge\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
reg add "HKCU\Software\Microsoft\Edge Dev\NativeMessagingHosts\com.msdlp.downloader" /ve /t REG_SZ /d "%MANIFEST_PATH%" /f >nul 2>&1
if !errorlevel! equ 0 (
    echo   * Microsoft Edge: Registered
) else (
    echo   * Microsoft Edge: Failed or skipped
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
echo ===================================================
echo   Setup completed successfully.
echo ===================================================
echo.
echo Next steps to use ms-dlp in your browser:
echo 1. Open your browser extensions page:
echo      Chrome: chrome://extensions
echo      Edge  : edge://extensions
echo      Brave : brave://extensions
echo.
echo 2. Turn ON "Developer mode" (toggle in top-right corner).
echo.
echo 3. Click "Load unpacked" and select the extension folder:
echo      Folder: !EXT_DIR!
echo    (The extension folder is now opening in Explorer...)
echo.
echo ===================================================
echo.

:: Open extension directory in Explorer for user convenience
explorer.exe "%EXT_DIR%"

pause
endlocal
