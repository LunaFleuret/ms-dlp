@echo off
setlocal
chcp 65001 >nul
title Universal Music Downloader

set "RUST_EXE=%~dp0ms-dlp.exe"
if exist "%RUST_EXE%" (
    "%RUST_EXE%" %*
) else (
    echo [ERROR] ms-dlp.exe not found!
    echo Please run "cargo build --release" to generate the executable.
    pause
    exit /b 1
)

if %ERRORLEVEL% neq 0 (
    echo.
    echo Process ended with an error code: %ERRORLEVEL%
    pause
)
endlocal
