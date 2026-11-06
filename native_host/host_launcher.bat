@echo off
setlocal
set "RUST_EXE=%~dp0..\ms-dlp.exe"
if exist "%RUST_EXE%" (
    "%RUST_EXE%" --native-host
) else (
    echo [ERROR] ms-dlp.exe not found! >&2
    exit /b 1
)
endlocal
