@echo off
setlocal
title Compilar Farming 6

REM Compila la version final y la deja en target\release. Primero corre las pruebas.
cd /d "%~dp0"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"
set "RUSTUP_TOOLCHAIN=1.99.1-x86_64-pc-windows-gnu"

echo.
echo   Corriendo las pruebas...
cargo test --locked
if errorlevel 1 goto FALLO

echo.
echo   Compilando la version final...
cargo build --release --locked
if errorlevel 1 goto FALLO

echo.
echo   Listo: target\release\farming6.exe
echo.
pause
exit /b 0

:FALLO
echo.
echo   No se pudo compilar. Revise el error de arriba.
echo.
pause
exit /b 1
