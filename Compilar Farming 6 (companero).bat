@echo off
chcp 65001 >nul
setlocal EnableExtensions
title Compilar Farming 6 - Parcial II

REM Hace solo lo que dice el bloc «Farming 6 - Qué debe hacer el compañero.txt»:
REM   1. Instala Rust (variante GNU, sin Visual Studio) si no está.
REM   2. Descarga el código desde GitHub y lo descomprime en el Escritorio.
REM   3. Compila Farming 6 y deja el programa en la carpeta «Farming 6 - Programa» del Escritorio.
REM No pide permisos de administrador. Si se vuelve a ejecutar, conserva la base de datos.

set "REPO_ZIP=https://github.com/Nico1091/farming6/archive/refs/heads/parcial2.zip"
set "RUSTUP=https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-gnu/rustup-init.exe"
set "TOOLCHAIN=stable-x86_64-pc-windows-gnu"
set "PATH=%USERPROFILE%\.cargo\bin;%PATH%"

for /f "usebackq delims=" %%D in (`powershell -NoProfile -Command "[Environment]::GetFolderPath('Desktop')"`) do set "ESCRITORIO=%%D"
if not defined ESCRITORIO set "ESCRITORIO=%USERPROFILE%\Desktop"
set "CODIGO=%ESCRITORIO%\Farming 6 - Codigo"
set "PROGRAMA=%ESCRITORIO%\Farming 6 - Programa"

echo.
echo   FARMING 6 - PARCIAL II
echo   Este script instala Rust si hace falta, descarga el código y lo compila.
echo   Necesita internet. La primera vez tarda entre 10 y 20 minutos.
echo.
pause

echo.
echo   [1/3] Rust
where rustup >nul 2>nul
if errorlevel 1 (
    echo         Rust no está instalado. Descargando el instalador oficial...
    curl.exe -L --fail -o "%TEMP%\rustup-init.exe" "%RUSTUP%"
    if errorlevel 1 goto FALLO_DESCARGA
    "%TEMP%\rustup-init.exe" -y --default-host x86_64-pc-windows-gnu --default-toolchain stable --profile minimal
    if errorlevel 1 goto FALLO_RUST
) else (
    echo         Rust ya está instalado. Se asegura la variante GNU...
    rustup toolchain install %TOOLCHAIN% --profile minimal
    if errorlevel 1 goto FALLO_RUST
)
cargo +%TOOLCHAIN% --version
if errorlevel 1 goto FALLO_RUST

echo.
echo   [2/3] Código desde GitHub
curl.exe -L --fail -o "%TEMP%\farming6-parcial2.zip" "%REPO_ZIP%"
if errorlevel 1 goto FALLO_DESCARGA
if exist "%CODIGO%" rd /s /q "%CODIGO%"
mkdir "%CODIGO%"
tar.exe -xf "%TEMP%\farming6-parcial2.zip" -C "%CODIGO%" --strip-components=1
if errorlevel 1 goto FALLO_ZIP
echo         Listo: %CODIGO%

echo.
echo   [3/3] Compilando (la primera vez descarga las bibliotecas y tarda varios minutos)...
cd /d "%CODIGO%"
cargo +%TOOLCHAIN% build --release --locked
if errorlevel 1 goto FALLO_COMPILAR
if not exist "%PROGRAMA%" mkdir "%PROGRAMA%"
copy /y "target\release\farming6.exe" "%PROGRAMA%\Farming6.exe" >nul
if errorlevel 1 goto FALLO_COMPILAR

echo.
echo   TODO LISTO.
echo   Programa: %PROGRAMA%\Farming6.exe
echo   Código:   %CODIGO%
echo.
start "" explorer.exe "%PROGRAMA%"
start "" "%PROGRAMA%\Farming6.exe"
pause
exit /b 0

:FALLO_DESCARGA
echo.
echo   No se pudo descargar. Revise la conexión a internet y ejecute de nuevo este archivo.
goto FIN_MAL
:FALLO_RUST
echo.
echo   Rust no se pudo instalar. Cierre esta ventana, reinicie el equipo y ejecute de nuevo este archivo.
goto FIN_MAL
:FALLO_ZIP
echo.
echo   No se pudo descomprimir el código. Ejecute de nuevo este archivo.
goto FIN_MAL
:FALLO_COMPILAR
echo.
echo   La compilación falló. Tómele una foto a esta ventana y envíesela a su compañero.
goto FIN_MAL
:FIN_MAL
echo.
pause
exit /b 1
