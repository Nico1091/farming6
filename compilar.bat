@echo off
setlocal
title Compilar Farming 6

REM Compila en WSL (Ubuntu) con destino Windows, corre las pruebas en Windows y deja la version
REM final en la carpeta "Farming 6". Desde el 12/11/2026 Smart App Control bloquea en Windows los
REM programas auxiliares que Rust compila; en WSL esos auxiliares corren en Linux.
cd /d "%~dp0"
wsl -e bash ./compilar-wsl.sh
if errorlevel 1 goto FALLO
echo.
pause
exit /b 0

:FALLO
echo.
echo   No se pudo compilar. Revise el error de arriba.
echo.
pause
exit /b 1
