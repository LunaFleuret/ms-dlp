@echo off
setlocal
chcp 65001 >nul
title YouTube Music Downloader

echo ========================================
echo   YouTube Music Downloader
echo ========================================
echo.

powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0download_music.ps1"

if %ERRORLEVEL% neq 0 (
    echo.
    echo Script ended with an error code: %ERRORLEVEL%
)

echo.
pause
