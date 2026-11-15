#!/bin/bash
# Compila Farming 6 en WSL (Ubuntu) con destino Windows.
#
# Desde las actualizaciones del 12/11/2026, Smart App Control bloquea en Windows los programas
# auxiliares que Rust compila (scripts de compilación y macros). Aquí esos auxiliares corren en
# Linux; solo las pruebas y el programa final, que son ejecutables de Windows, corren en Windows.
#
# Requisitos en Ubuntu (una sola vez):
#   sudo apt install gcc-mingw-w64-x86-64-win32
#   rustup (https://rustup.rs) con: rustup target add x86_64-pc-windows-gnu
#   y en ~/.cargo/config.toml:  [target.x86_64-pc-windows-gnu]  linker = "x86_64-w64-mingw32-gcc"
set -euo pipefail

DESTINO=x86_64-pc-windows-gnu
PROYECTO="$(cd "$(dirname "$0")" && pwd)"
COPIA="$HOME/farming6"
export PATH="$HOME/.cargo/bin:$PATH"
# build.rs copia las DLL de SQLite, shlwapi y GDI+ desde el System32 de Windows.
export SystemRoot=/mnt/c/Windows

command -v cargo >/dev/null || { echo "Falta Rust en Ubuntu: instale rustup."; exit 1; }
command -v x86_64-w64-mingw32-gcc >/dev/null || { echo "Falta MinGW: sudo apt install gcc-mingw-w64-x86-64-win32"; exit 1; }
rustup target list --installed | grep -qx "$DESTINO" || rustup target add "$DESTINO"

# Se compila en el disco de Linux, mucho más rápido que sobre /mnt/c.
echo "  Copiando el proyecto a $COPIA..."
mkdir -p "$COPIA"
(cd "$PROYECTO" && tar --exclude=./target -cf - .) | (cd "$COPIA" && tar -xf -)
cd "$COPIA"

echo "  Compilando las pruebas..."
REGISTRO="$(mktemp)"
cargo test --no-run --target "$DESTINO" >"$REGISTRO" 2>&1 || { cat "$REGISTRO"; exit 1; }
grep -Ev '^\s+Compiling ' "$REGISTRO" || true
PRUEBAS="$(sed -n 's/.*Executable unittests src\/main.rs (\(.*\))/\1/p' "$REGISTRO")"
rm -f "$REGISTRO"
[ -n "$PRUEBAS" ] || { echo "No se encontró el ejecutable de pruebas."; exit 1; }

echo "  Corriendo las pruebas en Windows..."
TEMPORAL="$(wslpath "$(cmd.exe /c 'echo %TEMP%' 2>/dev/null | tr -d '\r')")/farming6-pruebas"
mkdir -p "$TEMPORAL"
cp "$PRUEBAS" "$TEMPORAL/pruebas.exe"
RESULTADO="$PROYECTO/../docs/RESULTADO-PRUEBAS.txt"
{
    echo "Pruebas de Farming 6 · $(TZ=America/Bogota date '+%d/%m/%Y %H:%M') (hora de Colombia)"
    echo "Compiladas en WSL con $(rustc --version) para $DESTINO y ejecutadas en Windows."
    echo
} > "$RESULTADO"
set +e
(cd "$TEMPORAL" && ./pruebas.exe --test-threads=4) 2>&1 | tr -d '\r' | tee -a "$RESULTADO"
FALLO=${PIPESTATUS[0]}
set -e
[ "$FALLO" -eq 0 ] || { echo "  Hay pruebas que fallan: no se compila la versión final."; exit 1; }

echo "  Compilando la versión final..."
cargo build --release --target "$DESTINO" 2>&1 | grep -Ev '^\s+Compiling ' || true
mkdir -p "$PROYECTO/../Farming 6"
cp "target/$DESTINO/release/farming6.exe" "$PROYECTO/../Farming 6/Farming6.exe"
echo
echo "  Listo: Documents\\parcial2-software3\\Farming 6\\Farming6.exe"
echo "  Resultado de las pruebas: docs\\RESULTADO-PRUEBAS.txt"
