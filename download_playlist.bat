@echo off
setlocal enabledelayedexpansion
chcp 65001 >nul
set PYTHONIOENCODING=utf-8
title Universal Music Downloader - Playlist Mode

set "RUST_EXE=%~dp0ms-dlp.exe"
set "YTDLP_EXE=%~dp0bin\yt-dlp.exe"

if not exist "%RUST_EXE%" (
    echo [ERROR] ms-dlp.exe not found!
    echo Please make sure ms-dlp.exe is placed in the root folder.
    pause
    exit /b 1
)

if not exist "%YTDLP_EXE%" (
    where yt-dlp.exe >nul 2>&1
    if !errorlevel! equ 0 (
        set "YTDLP_EXE=yt-dlp.exe"
    ) else (
        echo [ERROR] yt-dlp.exe not found!
        echo Please run setup.bat first to install required tools into bin\ folder.
        pause
        exit /b 1
    )
)

set "TARGET_URL=%~1"
set "START_ARG=%~2"

if "%TARGET_URL%"=="" (
    echo ======================================================
    echo        Universal Music Downloader - Playlist Mode
    echo ======================================================
    echo.
    set /p "TARGET_URL=Enter YouTube Playlist URL (or press Enter to exit): "
)

if not defined TARGET_URL exit /b 0
for /f "tokens=* delims= " %%A in ("!TARGET_URL!") do set "TARGET_URL=%%A"
if "!TARGET_URL!"=="" exit /b 0

set "COOKIE_OPT="
if exist "%~dp0private\www.youtube.com_cookies.txt" (
    set COOKIE_OPT=--cookies "%~dp0private\www.youtube.com_cookies.txt"
) else if exist "%~dp0private\cookies.txt" (
    set COOKIE_OPT=--cookies "%~dp0private\cookies.txt"
)

echo.
echo [INFO] Target Playlist: !TARGET_URL!
echo [INFO] Fetching playlist information...

set "RAND_SUFFIX=%RANDOM%"
set "TITLE_FILE=%~dp0temp_pl_title_%RAND_SUFFIX%.txt"
if exist "%TITLE_FILE%" del /f /q "%TITLE_FILE%"
"%YTDLP_EXE%" --flat-playlist -I 1:1 --print "%%(playlist_title)s" !COOKIE_OPT! "!TARGET_URL!" > "%TITLE_FILE%" 2>nul

set "PL_NAME="
if exist "%TITLE_FILE%" (
    set /p PL_NAME=<"%TITLE_FILE%"
    del /f /q "%TITLE_FILE%"
)

if not defined PL_NAME set "PL_NAME=Playlist"
for /f "tokens=* delims= " %%A in ("!PL_NAME!") do set "PL_NAME=%%A"
if "!PL_NAME!"=="" set "PL_NAME=Playlist"

:: Sanitize playlist title for safe Windows folder naming
set "PL_NAME=!PL_NAME:/=_!"
set "PL_NAME=!PL_NAME:\=_!"
set "PL_NAME=!PL_NAME::=_!"
set "PL_NAME=!PL_NAME:*=_!"
set "PL_NAME=!PL_NAME:?=_!"
set "PL_NAME=!PL_NAME:<=_!"
set "PL_NAME=!PL_NAME:>=_!"
set "PL_NAME=!PL_NAME:|=_!"
set "PL_NAME=!PL_NAME:"=_!"

echo [INFO] Playlist Name: !PL_NAME!
echo [INFO] Fetching track list from playlist...

set "LIST_FILE=%~dp0temp_pl_urls_%RAND_SUFFIX%.txt"
if exist "%LIST_FILE%" del /f /q "%LIST_FILE%"

"%YTDLP_EXE%" --flat-playlist --print "%%(url)s" !COOKIE_OPT! "!TARGET_URL!" > "%LIST_FILE%" 2>nul

if not exist "%LIST_FILE%" (
    echo [ERROR] Failed to fetch track list from playlist.
    pause
    exit /b 1
)

set /a TOTAL=0
for /f "usebackq tokens=*" %%A in ("%LIST_FILE%") do (
    set /a TOTAL+=1
)

if %TOTAL% equ 0 (
    echo [ERROR] No tracks found in the playlist.
    if exist "%LIST_FILE%" del /f /q "%LIST_FILE%"
    pause
    exit /b 1
)

set "PLAYLIST_FOLDER=%~dp0output\!PL_NAME!"
if not exist "!PLAYLIST_FOLDER!" (
    mkdir "!PLAYLIST_FOLDER!" 2>nul
)

:: Validate target folder existence before any file movement
if not exist "!PLAYLIST_FOLDER!" (
    echo [ERROR] Failed to create output playlist folder: !PLAYLIST_FOLDER!
    if exist "%LIST_FILE%" del /f /q "%LIST_FILE%"
    pause
    exit /b 1
)

echo [INFO] Output folder: output\!PL_NAME!\

:: Count existing tracks in playlist folder for auto-resume
set /a EXISTING_COUNT=0
for %%F in ("!PLAYLIST_FOLDER!\*.m4a" "!PLAYLIST_FOLDER!\*.mp3" "!PLAYLIST_FOLDER!\*.opus" "!PLAYLIST_FOLDER!\*.flac") do (
    set /a EXISTING_COUNT+=1
)

set /a START_INDEX=1
if not "%START_ARG%"=="" (
    set /a START_INDEX=%START_ARG%
) else if %EXISTING_COUNT% gtr 0 (
    if %EXISTING_COUNT% lss %TOTAL% (
        set /a SUGGESTED_START=%EXISTING_COUNT%+1
        echo.
        echo [INFO] Found !EXISTING_COUNT! / %TOTAL% tracks already downloaded in output\!PL_NAME!\.
        set /p "USER_START=Resume from track [!SUGGESTED_START!-%TOTAL%] (Press Enter for !SUGGESTED_START!, or type 1 to restart): "
        if "!USER_START!"=="" (
            set /a START_INDEX=!SUGGESTED_START!
        ) else (
            set /a START_INDEX=!USER_START!
        )
    )
)

echo.
echo [INFO] Total tracks: %TOTAL%
echo [INFO] Starting from track: !START_INDEX!
echo ======================================================
echo.

set /a CURRENT=0
for /f "usebackq tokens=*" %%U in ("%LIST_FILE%") do (
    set /a CURRENT+=1
    if !CURRENT! geq !START_INDEX! (
        echo ------------------------------------------------------
        echo [Progress: !CURRENT! / %TOTAL%] Processing: %%U
        echo ------------------------------------------------------
        "%RUST_EXE%" "%%U"
        
        if exist "!PLAYLIST_FOLDER!" (
            move /y "%~dp0output\*.m4a" "!PLAYLIST_FOLDER!\" >nul 2>nul
            move /y "%~dp0output\*.mp3" "!PLAYLIST_FOLDER!\" >nul 2>nul
            move /y "%~dp0output\*.opus" "!PLAYLIST_FOLDER!\" >nul 2>nul
            move /y "%~dp0output\*.flac" "!PLAYLIST_FOLDER!\" >nul 2>nul
            move /y "%~dp0output\*.wav" "!PLAYLIST_FOLDER!\" >nul 2>nul
        )
        echo.
    ) else (
        echo [Skipping !CURRENT! / %TOTAL%] Already downloaded.
    )
)

if exist "%LIST_FILE%" del /f /q "%LIST_FILE%"

echo ======================================================
echo [SUCCESS] Finished processing playlist tracks!
echo Downloaded files are saved in: output\!PL_NAME!\
echo ======================================================
pause
endlocal
