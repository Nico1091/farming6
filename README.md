# Farming 6

Aplicación de escritorio para Windows, escrita en Rust con egui y SQLite: acceso al sistema, menú y CRUD de Perfil, Persona y Usuario sobre una base relacional, con costos y precio de venta del ganado como tema del panel. Trabajo académico de Ingeniería de Software III.

## Qué incluye

- Modelo relacional `perfil`, `usuario` y `persona`. Un perfil tiene varios usuarios (1:N) y una persona tiene un solo usuario (1:1).
- Ingreso, recuperación por correo y registro por pasos. Con la base vacía, el registro crea el primer administrador.
- Gestión Perfil, Persona y Usuario con Crear, Consultar, Modificar, Inhabilitar, Guardar y Salir.
- Inhabilitar es borrado lógico: ninguna función borra filas, solo cambia `estado`.
- Base de datos en tiempo real: un cuadro por tabla, sus relaciones y una rejilla de registros.

## Ejecutar

La base `farming6.db` se crea junto al ejecutable la primera vez, vacía. Este repositorio no incluye datos ni credenciales.

Para que la recuperación envíe correos, entre como administrador a Correo y guarde una cuenta de Gmail con su clave de aplicación. La clave queda cifrada con DPAPI para ese usuario de Windows.

## Compilar

Doble clic en `compilar.bat`, o bien:

```
rustup toolchain install 1.99.1-x86_64-pc-windows-gnu --profile minimal
cargo +1.99.1-x86_64-pc-windows-gnu build --release --locked
```

Solo necesita Rust para Windows en su variante GNU. Usa la SQLite y el `curl.exe` que ya trae Windows, así que no hace falta compilador de C. La variante MSVC no sirve para este proyecto.

`Cargo.lock` fija varias dependencias en versiones anteriores a propósito. No lo actualice con `cargo update`.

## Estructura

```
migrations/001_init.sql   Esquema del modelo relacional
src/db/mod.rs             CRUD con consultas parametrizadas
src/db/vigia.rs           Lectura en vivo para el mapa
src/validaciones/mod.rs   Reglas de cada formulario
src/auth/mod.rs           Argon2id y contraseña temporal
src/auth/correo.rs        Envío del correo y cuenta cifrada
src/ui/                   Pantallas
src/pruebas.rs            Pruebas de punta a punta
instalador/               Indicaciones y guiones del equipo
```
