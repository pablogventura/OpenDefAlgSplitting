@echo off
REM Compilar en release (Windows). Requiere: rustup, cargo en el PATH.
cargo build --release
if %ERRORLEVEL% neq 0 exit /b %ERRORLEVEL%
echo.
echo Binario: target\release\opendefalgsplitting.exe
echo Ejecutar: target\release\opendefalgsplitting.exe tu_modelo.model
