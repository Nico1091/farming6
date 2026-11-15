//! Consultas a la base desde la pantalla Base de datos.
//!
//! El administrador elige una consulta guiada o escribe la suya, y el resultado sale en una
//! rejilla. Nada de esto puede cambiar datos: la consulta corre en una conexión aparte con
//! `PRAGMA query_only`, solo se aceptan sentencias que empiezan por SELECT, WITH, VALUES o
//! EXPLAIN y que SQLite marca como de solo lectura, una sola a la vez, y si tarda más de tres
//! segundos se interrumpe. Para crear, modificar o inhabilitar están los formularios.

use super::vigia::describir_bytes;
use rusqlite::{types::ValueRef, Batch, Connection, ErrorCode, OpenFlags, Statement};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Filas que se muestran como máximo; la consulta deja de leer al llegar aquí.
pub const MAXIMO_FILAS: usize = 500;
const TIEMPO_MAXIMO: Duration = Duration::from_secs(3);
const PERMITIDAS: [&str; 4] = ["SELECT", "WITH", "VALUES", "EXPLAIN"];

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ResultadoConsulta {
    pub columnas: Vec<String>,
    pub filas: Vec<Vec<String>>,
    /// Había más filas que `MAXIMO_FILAS`.
    pub recortada: bool,
    pub milisegundos: u128,
}

pub struct ConsultaGuiada {
    pub titulo: &'static str,
    pub sql: &'static str,
}

/// Una consulta por historia del Parcial II, más el catálogo del DANE. Las mismas reglas que usa
/// la aplicación: la ocupación de un corral suma los cupos activos de los lotes en ceba.
pub const GUIADAS: [ConsultaGuiada; 6] = [
    ConsultaGuiada {
        titulo: "Productores y sus granjas",
        sql: "SELECT p.idpersona AS documento, p.nom1 || ' ' || p.apell1 AS productor, p.movil AS telefono,
       pr.fecha_registro AS registrado, COUNT(g.codigo) AS granjas,
       CASE p.estado WHEN 'A' THEN 'Activo' ELSE 'Inhabilitado' END AS estado
FROM productor pr
JOIN persona p ON p.idpersona = pr.idproductor
LEFT JOIN granja g ON g.idproductor = pr.idproductor
GROUP BY pr.idproductor
ORDER BY productor",
    },
    ConsultaGuiada {
        titulo: "Granjas por municipio",
        sql: "SELECT g.codigo, g.nombre AS granja, p.nom1 || ' ' || p.apell1 AS productor,
       m.nombre AS municipio, d.nombre AS departamento, g.vereda,
       CASE g.estado WHEN 'A' THEN 'Activa' ELSE 'Inhabilitada' END AS estado
FROM granja g
JOIN persona p ON p.idpersona = g.idproductor
JOIN municipio m ON m.idmunicipio = g.idmunicipio
JOIN departamento d ON d.iddepartamento = m.iddepartamento
ORDER BY g.codigo",
    },
    ConsultaGuiada {
        titulo: "Ocupación de los corrales",
        sql: "SELECT c.codigo, c.nombre AS corral, g.nombre AS granja, c.capacidad,
       COALESCE(SUM(CASE WHEN l.estado = 'A' AND lc.estado = 'A' THEN lc.cupos END), 0) AS ocupados,
       c.capacidad - COALESCE(SUM(CASE WHEN l.estado = 'A' AND lc.estado = 'A' THEN lc.cupos END), 0) AS libres
FROM corral c
JOIN granja g ON g.codigo = c.codigo_granja
LEFT JOIN lote_corral lc ON lc.codigo_corral = c.codigo
LEFT JOIN lote l ON l.codigol = lc.codigol
GROUP BY c.codigo
ORDER BY g.nombre, c.nombre",
    },
    ConsultaGuiada {
        titulo: "Razas activas",
        sql: "SELECT codigor AS codigo, descripcion AS raza, imagen
FROM raza
WHERE estado = 'A'
ORDER BY descripcion",
    },
    ConsultaGuiada {
        titulo: "Lotes en ceba",
        sql: "SELECT l.codigol AS lote, g.nombre AS granja, l.cantidad AS cerdos, l.fecha_inicial, l.fecha_final,
       l.peso_objetivo, group_concat(c.nombre || ' (' || lc.cupos || ')', ', ') AS corrales
FROM lote l
JOIN granja g ON g.codigo = l.codigo_granja
LEFT JOIN lote_corral lc ON lc.codigol = l.codigol AND lc.estado = 'A'
LEFT JOIN corral c ON c.codigo = lc.codigo_corral
WHERE l.estado = 'A'
GROUP BY l.codigol
ORDER BY l.codigol",
    },
    ConsultaGuiada {
        titulo: "Municipios por departamento",
        sql: "SELECT d.iddepartamento AS codigo, d.nombre AS departamento, COUNT(m.idmunicipio) AS municipios
FROM departamento d
LEFT JOIN municipio m ON m.iddepartamento = d.iddepartamento
GROUP BY d.iddepartamento
ORDER BY d.nombre",
    },
];

/// Primera palabra de la consulta, saltando espacios, comentarios y paréntesis de apertura.
fn primera_palabra(sql: &str) -> String {
    let mut resto = sql;
    loop {
        resto = resto.trim_start_matches(|c: char| c.is_whitespace() || c == '(');
        if let Some(r) = resto.strip_prefix("--") {
            resto = r.split_once('\n').map_or("", |(_, despues)| despues);
        } else if let Some(r) = resto.strip_prefix("/*") {
            resto = r.split_once("*/").map_or("", |(_, despues)| despues);
        } else {
            break;
        }
    }
    resto.chars().take_while(|c| c.is_ascii_alphabetic()).collect::<String>().to_ascii_uppercase()
}

fn abrir(ruta: &Path) -> rusqlite::Result<Connection> {
    let conexion = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)?;
    conexion.busy_timeout(Duration::from_secs(2))?;
    conexion.execute_batch("PRAGMA query_only = ON;")?;
    Ok(conexion)
}

/// Texto de una celda. Las contraseñas cifradas se recortan aunque la consulta las renombre.
fn celda(valor: ValueRef) -> String {
    match valor {
        ValueRef::Null => String::new(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(n) => n.to_string(),
        ValueRef::Text(t) => {
            let texto = String::from_utf8_lossy(t);
            if texto.starts_with("$argon2") {
                let cola = texto.get(texto.len().saturating_sub(6)..).unwrap_or("");
                format!("argon2id…{cola}")
            } else {
                texto.into_owned()
            }
        }
        ValueRef::Blob(b) => describir_bytes(b),
    }
}

/// Mensaje para el usuario a partir del error del motor, que se deja al final para quien sabe SQL.
fn traducir(error: rusqlite::Error) -> String {
    if let rusqlite::Error::SqliteFailure(fallo, detalle) = &error {
        if fallo.code == ErrorCode::OperationInterrupted {
            return format!("La consulta tardó más de {} segundos y se detuvo.", TIEMPO_MAXIMO.as_secs());
        }
        let texto = detalle.as_deref().unwrap_or("");
        if let Some(tabla) = texto.strip_prefix("no such table: ") {
            return format!("No existe la tabla «{tabla}». Las tablas están en las pestañas de abajo.");
        }
        if let Some(columna) = texto.strip_prefix("no such column: ") {
            return format!("No existe la columna «{columna}».");
        }
        if texto.contains("syntax error") {
            return format!("La consulta tiene un error de escritura ({texto}).");
        }
        if texto.contains("readonly") || texto.contains("read-only") {
            return "La consulta intentó escribir y la base solo deja leer desde aquí.".into();
        }
        return format!("La base no pudo responder: {texto}");
    }
    match error {
        rusqlite::Error::InvalidParameterCount(..) => "La consulta lleva parámetros (?): escriba los valores en su lugar.".into(),
        otro => format!("La base no pudo responder: {otro}"),
    }
}

/// Corre una consulta de lectura sobre la base de `ruta` y devuelve sus columnas y filas.
pub fn consultar(ruta: &Path, sql: &str) -> Result<ResultadoConsulta, String> {
    let sql = sql.trim();
    if sql.is_empty() {
        return Err("Escriba una consulta o elija una de las guiadas.".into());
    }
    if !PERMITIDAS.contains(&primera_palabra(sql).as_str()) {
        return Err("Aquí solo se consulta: la sentencia debe empezar por SELECT, WITH, VALUES o EXPLAIN. \
                    Para crear, modificar o inhabilitar use los formularios."
            .into());
    }
    let conexion = abrir(ruta).map_err(|e| format!("No se pudo abrir la base de datos: {e}"))?;
    let mut lote = Batch::new(&conexion, sql);
    let mut sentencia = match lote.next() {
        Ok(Some(s)) => s,
        Ok(None) => return Err("Escriba una consulta o elija una de las guiadas.".into()),
        Err(e) => return Err(traducir(e)),
    };
    if !sentencia.readonly() {
        return Err("Aquí solo se consulta: esa sentencia cambiaría datos.".into());
    }
    if !matches!(lote.next(), Ok(None)) {
        return Err("Escriba una sola consulta a la vez.".into());
    }

    // Vigilante: si la consulta sigue corriendo al cumplirse el tiempo, la interrumpe.
    let terminada = Arc::new(AtomicBool::new(false));
    let manija = conexion.get_interrupt_handle();
    let aviso = Arc::clone(&terminada);
    thread::spawn(move || {
        let inicio = Instant::now();
        while inicio.elapsed() < TIEMPO_MAXIMO {
            if aviso.load(Ordering::Acquire) {
                return;
            }
            thread::sleep(Duration::from_millis(25));
        }
        if !aviso.load(Ordering::Acquire) {
            manija.interrupt();
        }
    });

    let inicio = Instant::now();
    let leido = leer(&mut sentencia);
    terminada.store(true, Ordering::Release);
    let mut resultado = leido.map_err(traducir)?;
    resultado.milisegundos = inicio.elapsed().as_millis();
    Ok(resultado)
}

fn leer(sentencia: &mut Statement) -> rusqlite::Result<ResultadoConsulta> {
    let columnas: Vec<String> = sentencia.column_names().into_iter().map(String::from).collect();
    let cuantas = columnas.len();
    let mut resultado = ResultadoConsulta { columnas, ..Default::default() };
    let mut filas = sentencia.query([])?;
    while let Some(fila) = filas.next()? {
        if resultado.filas.len() == MAXIMO_FILAS {
            resultado.recortada = true;
            break;
        }
        resultado.filas.push((0..cuantas).map(|i| fila.get_ref(i).map(celda)).collect::<rusqlite::Result<_>>()?);
    }
    Ok(resultado)
}

#[cfg(test)]
mod pruebas {
    use super::primera_palabra;

    #[test]
    fn la_primera_palabra_salta_comentarios_y_parentesis() {
        assert_eq!(primera_palabra("  select 1"), "SELECT");
        assert_eq!(primera_palabra("-- productores\nSELECT * FROM productor"), "SELECT");
        assert_eq!(primera_palabra("/* nota */ (WITH x AS (SELECT 1) SELECT * FROM x)"), "WITH");
        assert_eq!(primera_palabra("delete from raza"), "DELETE");
        assert_eq!(primera_palabra("-- solo un comentario"), "");
    }
}
