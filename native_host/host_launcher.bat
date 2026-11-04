@echo off
setlocal
set "PORTABLE_PY=%~dp0..\bin\python\python.exe"
if exist "%PORTABLE_PY%" (
    "%PORTABLE_PY%" "%~dp0host.py"
) else (
    python "%~dp0host.py"
)
endlocal
