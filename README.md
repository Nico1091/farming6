# Farming 6

Parciales I y II de Ingeniería de Software III, Uniremington.

## Parcial II: requisitos funcionales aprobados (HU-01 a HU-05)

- `migrations/002_porcinos.sql` completa el modelo en tercera forma normal con `productor` (1:1 con persona, como usuario), `departamento` y `municipio` (DIVIPOLA del DANE, en `003_divipola.sql`), `granja`, `corral`, `raza`, `lote` y `lote_corral` (llave compuesta), y con disparadores que imponen las reglas de capacidad, granja y estado aunque falle la interfaz. `001b_correo_opcional.sql` deja el correo de persona opcional para los productores sin perder filas.
- HU-01 Productores (`src/ui/productor.rs`): el formulario de persona con las reglas de la historia (`validaciones::productor`): teléfono de 10 dígitos, dirección y fecha de nacimiento obligatorias, mayor de 18 años, correo opcional.
- HU-02 a HU-05 se describen una sola vez en `src/models/registros.rs` (campos, columnas calculadas, filtros, bloqueos y estados). De ahí salen las consultas de `src/db/registros.rs` y la pantalla de `src/ui/registro.rs`: departamento y municipio en cascada, ocupación de corrales, imagen de raza (`src/imagen.rs`, GDI+ de Windows) y corrales del lote con su cierre.
- El Administrador ve todo. El productor gestiona sus granjas, corrales y lotes y consulta las razas activas.
- La base se consulta desde el programa (`src/db/consulta.rs` y `src/ui/consulta.rs`): en la pantalla Base de datos, el Administrador elige una de seis consultas guiadas o escribe la suya en SQL. Corre en una conexión con `query_only`, solo acepta una sentencia SELECT, WITH, VALUES o EXPLAIN que SQLite marque como de lectura, se detiene a los tres segundos, muestra hasta 500 filas y recorta las contraseñas cifradas.
- `src/pruebas/porcinos.rs` ejercita cada criterio de aceptación de HU-01 a HU-05 y la consulta.
- Smart App Control: desde las actualizaciones del 12/11/2026 bloquea en Windows los scripts de compilación y las macros que compila Rust. Por eso se compila desde WSL (ver Compilar).

Lo que sigue describe el Parcial I, que se conserva completo. Aplicación de escritorio para Windows con acceso al sistema, menú y CRUD de Perfil, Usuario y Persona sobre una base relacional SQLite.

## Qué cumple del enunciado

- Modelo relacional exacto: `perfil`, `usuario` y `persona`, sin campos adicionales. Un perfil tiene varios usuarios (1:N). Una persona tiene un solo usuario (1:1): `idusuario` es llave primaria de `usuario` y llave foránea hacia `persona`.
- Ingreso a pantalla completa, sin caja, con Ingresar, Cancelar, Recuperar y un enlace pequeño de Registro.
- Registro por pasos, de seis campos como máximo. Con la base vacía, su último paso crea el primer administrador.
- Gestión Perfil, Persona y Usuario con Crear, Consultar, Modificar, Inhabilitar, Guardar y Salir.
- Permisos de la tabla de requisitos: el perfil Persona no entra a Perfil ni a Usuario y no puede inhabilitar.
- Inhabilitar es borrado lógico: ninguna función borra filas, solo cambia `estado`.
- Recuperación por identificación o correo: envía el usuario y una contraseña temporal al correo registrado. La contraseña solo cambia si el servidor aceptó el mensaje.
- Panel principal con el tema del reto: costos y precio de venta del ganado.
- Base de datos en tiempo real, solo en su propia pantalla: un cuadro por tabla con sus relaciones y una rejilla de registros.

## Mapa en tiempo real

`src/db/vigia.rs` abre una conexión aparte y consulta `PRAGMA data_version` cinco veces por segundo. Cuando la base cambia, vuelve a leer el esquema (`sqlite_master`, `table_info`, `foreign_key_list`) y las filas, y compara con la lectura anterior. El mapa se dibuja con esos datos, no con datos escritos en el código, y también detecta cambios hechos por otro programa.

## Ejecutar

Abra `Farming6.exe`. La primera vez pide crear el administrador. La base `farming6.db` queda junto al ejecutable.

Para que la recuperación envíe correos, entre como administrador a **Correo de recuperación** y guarde una cuenta de Gmail con su clave de aplicación. La clave queda cifrada con DPAPI para ese usuario de Windows.

## Compilar

Doble clic en `compilar.bat`. Llama a `compilar-wsl.sh`, que copia el proyecto a Ubuntu (WSL), compila hacia Windows (`x86_64-pc-windows-gnu`), corre las pruebas en Windows, guarda su salida en `docs\RESULTADO-PRUEBAS.txt` y, si todas pasan, deja `Farming6.exe` en la carpeta `Farming 6`. Los scripts de compilación y las macros corren en Linux, fuera del alcance de Smart App Control; solo las pruebas y el programa, que son ejecutables de Windows, corren en Windows.

En Ubuntu necesita MinGW (`sudo apt install gcc-mingw-w64-x86-64-win32`), rustup con `rustup target add x86_64-pc-windows-gnu` y en `~/.cargo/config.toml` la línea `linker = "x86_64-w64-mingw32-gcc"` bajo `[target.x86_64-pc-windows-gnu]`. El programa usa la SQLite, GDI+ y el `curl.exe` que ya trae Windows; `build.rs` toma las DLL de `/mnt/c/Windows/System32`.

`Cargo.lock` fija `libloading`, `parking_lot`, `parking_lot_core`, `arboard` y `webbrowser` en versiones anteriores a propósito: las nuevas necesitan `dlltool` con un ensamblador, que Rust no incluye en esta variante. No lo actualice con `cargo update`.

## Estructura

```
migrations/001_init.sql   Esquema del modelo relacional
src/db/mod.rs             CRUD con consultas parametrizadas
src/db/vigia.rs           Lectura en vivo para el mapa
src/db/consulta.rs        Consultas de solo lectura desde la pantalla Base de datos
src/validaciones/mod.rs   Reglas de cada formulario
src/auth/mod.rs           Argon2id y contraseña temporal
src/auth/correo.rs        Envío del correo y cuenta cifrada
src/ui/login.rs           Ingreso, recuperación, registro por pasos y telón
src/ui/paisaje.rs         Cordilleras en capas y sol
src/ui/perfil.rs          Gestión Perfil
src/ui/persona.rs         Gestión Persona y campos del registro
src/ui/usuario.rs         Gestión Usuario
src/ui/mapa.rs            Mapa de la base de datos
src/ui/consulta.rs        Sección Consultar de la pantalla Base de datos
src/ui/dashboard.rs       Panel principal
src/pruebas.rs            Pruebas de punta a punta
```

## Smart App Control

Windows decide por reputación si un ejecutable sin firma puede correr, y el veredicto cambia con cada compilación. El 14/11/2026 dejó correr las pruebas, la variante de capturas y la versión final compiladas en WSL. Antes de reemplazar `Farming 6\Farming6.exe`, compruebe que el nuevo arranca; si Windows lo veta, conserve el que ya funciona. No borre `target` con `cargo clean` en Windows.
