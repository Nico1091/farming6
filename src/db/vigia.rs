//! Vigía de la base de datos: el mapa en tiempo real.
//!
//! Un hilo aparte abre su propia conexión y pregunta cinco veces por segundo por
//! `PRAGMA data_version`, que cambia en cuanto se confirma cualquier escritura, venga de
//! esta aplicación o de otro programa. Solo entonces vuelve a leer el esquema real
//! (`sqlite_master`, `table_info`, `foreign_key_list`) y las filas, y compara con la lectura
//! anterior para decir qué cambió. El mapa no se dibuja con datos escritos en el código.

use crate::reloj;
use eframe::egui;
use rusqlite::{types::ValueRef, Connection, OpenFlags};
use std::collections::{BTreeMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MAXIMO_CAMBIOS: usize = 60;
const SEGUNDOS_DE_RESALTADO: f32 = 4.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Columna {
    pub nombre: String,
    pub tipo: String,
    pub llave_primaria: bool,
    pub obligatoria: bool,
    /// Tabla a la que apunta, si la columna es llave foránea.
    pub foranea: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TablaViva {
    pub nombre: String,
    pub columnas: Vec<Columna>,
    pub filas: Vec<Vec<String>>,
    pub activos: usize,
    pub inhabilitados: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Relacion {
    pub tabla: String,
    pub columna: String,
    pub tabla_destino: String,
    pub columna_destino: String,
    /// 1:1 si la llave foránea es también llave primaria; si no, 1:N.
    pub uno_a_uno: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Accion {
    Insertar,
    Actualizar,
    Inhabilitar,
    Activar,
    Borrar,
}

impl Accion {
    pub fn texto(self) -> &'static str {
        match self {
            Accion::Insertar => "INSERT",
            Accion::Actualizar => "UPDATE",
            Accion::Inhabilitar => "INHABILITAR",
            Accion::Activar => "ACTIVAR",
            Accion::Borrar => "DELETE",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Cambio {
    pub hora: String,
    pub accion: Accion,
    pub tabla: String,
    pub clave: String,
    pub detalle: String,
    pub columnas: Vec<String>,
    pub instante: Instant,
}

#[derive(Clone, Debug, Default)]
pub struct Foto {
    pub tablas: Vec<TablaViva>,
    pub relaciones: Vec<Relacion>,
    pub cambios: VecDeque<Cambio>,
    pub ultima_lectura: String,
    pub lecturas: u64,
    pub error: Option<String>,
}

impl Foto {
    pub fn tabla(&self, nombre: &str) -> Option<&TablaViva> {
        self.tablas.iter().find(|t| t.nombre == nombre)
    }

    /// Fuerza del resaltado de una tabla, de 1 (recién cambió) a 0.
    pub fn resaltado_tabla(&self, tabla: &str) -> f32 {
        self.cambios
            .iter()
            .filter(|c| c.tabla == tabla)
            .map(|c| desvanecer(c.instante))
            .fold(0.0, f32::max)
    }

    pub fn resaltado_columna(&self, tabla: &str, columna: &str) -> f32 {
        self.cambios
            .iter()
            .filter(|c| c.tabla == tabla && c.columnas.iter().any(|n| n == columna))
            .map(|c| desvanecer(c.instante))
            .fold(0.0, f32::max)
    }

    pub fn resaltado_fila(&self, tabla: &str, clave: &str) -> f32 {
        self.cambios
            .iter()
            .filter(|c| c.tabla == tabla && c.clave == clave)
            .map(|c| desvanecer(c.instante))
            .fold(0.0, f32::max)
    }

    pub fn hay_resaltados(&self) -> bool {
        self.cambios.front().is_some_and(|c| desvanecer(c.instante) > 0.0)
    }
}

fn desvanecer(instante: Instant) -> f32 {
    (1.0 - instante.elapsed().as_secs_f32() / SEGUNDOS_DE_RESALTADO).max(0.0)
}

/// Lectura cruda: guarda el valor completo para comparar, aunque el mapa lo muestre recortado.
#[derive(Default)]
struct Lectura {
    tablas: Vec<TablaViva>,
    relaciones: Vec<Relacion>,
    crudo: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

fn texto_de(valor: ValueRef) -> String {
    match valor {
        ValueRef::Null => String::new(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(n) => n.to_string(),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(b) => format!("<{} bytes>", b.len()),
    }
}

fn mostrar(columna: &str, valor: &str) -> String {
    if columna == "contrasena" {
        let cola = valor.get(valor.len().saturating_sub(6)..).unwrap_or("");
        format!("argon2id…{cola}")
    } else {
        valor.to_string()
    }
}

fn orden_de_tabla(nombre: &str) -> usize {
    match nombre {
        "perfil" => 0,
        "usuario" => 1,
        "persona" => 2,
        _ => 3,
    }
}

fn leer(conexion: &Connection) -> rusqlite::Result<Lectura> {
    let mut nombres: Vec<String> = conexion
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")?
        .query_map([], |f| f.get(0))?
        .collect::<Result<_, _>>()?;
    nombres.sort_by_key(|n| (orden_de_tabla(n), n.clone()));

    let mut lectura = Lectura::default();
    for nombre in nombres {
        let foraneas: Vec<(String, String, String)> = conexion
            .prepare(&format!("PRAGMA foreign_key_list(\"{nombre}\")"))?
            .query_map([], |f| Ok((f.get(3)?, f.get(2)?, f.get::<_, Option<String>>(4)?.unwrap_or_default())))?
            .collect::<Result<_, _>>()?;

        let columnas: Vec<Columna> = conexion
            .prepare(&format!("PRAGMA table_info(\"{nombre}\")"))?
            .query_map([], |f| {
                let columna: String = f.get(1)?;
                let foranea = foraneas.iter().find(|x| x.0 == columna).map(|x| x.1.clone());
                Ok(Columna {
                    tipo: f.get(2)?,
                    obligatoria: f.get::<_, i64>(3)? != 0 || f.get::<_, i64>(5)? != 0,
                    llave_primaria: f.get::<_, i64>(5)? != 0,
                    foranea,
                    nombre: columna,
                })
            })?
            .collect::<Result<_, _>>()?;

        for (columna, destino, columna_destino) in foraneas {
            let uno_a_uno = columnas.iter().any(|c| c.nombre == columna && c.llave_primaria);
            lectura.relaciones.push(Relacion {
                tabla: nombre.clone(),
                columna,
                tabla_destino: destino,
                columna_destino,
                uno_a_uno,
            });
        }

        let titulos: Vec<String> = columnas.iter().map(|c| c.nombre.clone()).collect();
        let llave = columnas.iter().position(|c| c.llave_primaria).unwrap_or(0);
        let posicion_estado = titulos.iter().position(|t| t == "estado");
        let mut filas = Vec::new();
        let mut crudo = BTreeMap::new();
        let (mut activos, mut inhabilitados) = (0, 0);

        let mut consulta = conexion.prepare(&format!("SELECT * FROM \"{nombre}\" ORDER BY 1"))?;
        let mut cursor = consulta.query([])?;
        while let Some(fila) = cursor.next()? {
            let mut valores = Vec::with_capacity(titulos.len());
            for i in 0..titulos.len() {
                valores.push(texto_de(fila.get_ref(i)?));
            }
            match posicion_estado.map(|i| valores[i].as_str()) {
                Some("I") => inhabilitados += 1,
                _ => activos += 1,
            }
            filas.push(titulos.iter().zip(&valores).map(|(t, v)| mostrar(t, v)).collect());
            crudo.insert(valores[llave].clone(), valores);
        }

        lectura.crudo.insert(nombre.clone(), crudo);
        lectura.tablas.push(TablaViva { nombre, columnas, filas, activos, inhabilitados });
    }
    Ok(lectura)
}

/// Compara dos lecturas y describe lo que pasó en la base de datos.
fn comparar(antes: &Lectura, ahora: &Lectura) -> Vec<Cambio> {
    let hora = reloj::ahora().hora_texto();
    let instante = Instant::now();
    let mut cambios = Vec::new();
    for tabla in &ahora.tablas {
        let vacio = BTreeMap::new();
        let previas = antes.crudo.get(&tabla.nombre).unwrap_or(&vacio);
        let actuales = ahora.crudo.get(&tabla.nombre).unwrap_or(&vacio);
        let titulos: Vec<&str> = tabla.columnas.iter().map(|c| c.nombre.as_str()).collect();
        let mut nuevo = |accion, clave: &str, detalle: String, columnas: Vec<String>| {
            cambios.push(Cambio {
                hora: hora.clone(),
                accion,
                tabla: tabla.nombre.clone(),
                clave: clave.to_string(),
                detalle,
                columnas,
                instante,
            });
        };

        for (clave, fila) in actuales {
            match previas.get(clave) {
                None => {
                    let columnas = titulos.iter().map(|t| t.to_string()).collect();
                    nuevo(Accion::Insertar, clave, "fila nueva".into(), columnas);
                }
                Some(previa) if previa != fila => {
                    let distintas: Vec<usize> =
                        (0..titulos.len()).filter(|&i| previa.get(i) != fila.get(i)).collect();
                    let columnas: Vec<String> = distintas.iter().map(|&i| titulos[i].to_string()).collect();
                    let detalle = distintas
                        .iter()
                        .map(|&i| {
                            if titulos[i] == "contrasena" {
                                "contrasena: hash nuevo".to_string()
                            } else {
                                format!("{}: {} → {}", titulos[i], previa[i], fila[i])
                            }
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    let estado = titulos.iter().position(|t| *t == "estado");
                    let cambio_estado = estado.filter(|i| distintas.contains(i)).map(|i| fila[i].as_str());
                    let accion = match cambio_estado {
                        Some("I") => Accion::Inhabilitar,
                        Some("A") => Accion::Activar,
                        _ => Accion::Actualizar,
                    };
                    nuevo(accion, clave, detalle, columnas);
                }
                _ => {}
            }
        }
        for clave in previas.keys().filter(|c| !actuales.contains_key(*c)) {
            nuevo(Accion::Borrar, clave, "fila eliminada físicamente".into(), Vec::new());
        }
    }
    cambios
}

fn abrir_lector(ruta: &Path) -> rusqlite::Result<Connection> {
    let conexion = Connection::open_with_flags(ruta, OpenFlags::SQLITE_OPEN_READ_WRITE)?;
    conexion.busy_timeout(Duration::from_secs(2))?;
    conexion.execute_batch("PRAGMA query_only = ON;")?;
    Ok(conexion)
}

/// Una pasada del vigía: lee si la base cambió y actualiza la foto compartida.
/// Devuelve verdadero si hubo una lectura nueva.
fn pasada(conexion: &Connection, version: &mut Option<i64>, previa: &mut Option<Lectura>, foto: &Mutex<Foto>) -> bool {
    let actual: rusqlite::Result<i64> = conexion.query_row("PRAGMA data_version", [], |f| f.get(0));
    let actual = match actual {
        Ok(v) => v,
        Err(e) => {
            if let Ok(mut f) = foto.lock() {
                f.error = Some(e.to_string());
            }
            return false;
        }
    };
    if *version == Some(actual) {
        return false;
    }
    let lectura = match leer(conexion) {
        Ok(l) => l,
        Err(e) => {
            if let Ok(mut f) = foto.lock() {
                f.error = Some(e.to_string());
            }
            return false;
        }
    };
    *version = Some(actual);
    let cambios = previa.as_ref().map(|p| comparar(p, &lectura)).unwrap_or_default();
    if let Ok(mut f) = foto.lock() {
        f.tablas = lectura.tablas.clone();
        f.relaciones = lectura.relaciones.clone();
        f.ultima_lectura = reloj::ahora().hora_texto();
        f.lecturas += 1;
        f.error = None;
        for cambio in cambios {
            f.cambios.push_front(cambio);
        }
        f.cambios.truncate(MAXIMO_CAMBIOS);
    }
    *previa = Some(lectura);
    true
}

/// Arranca el vigía en su propio hilo y devuelve la foto que dibuja la interfaz.
pub fn iniciar(ruta: PathBuf, contexto: Option<egui::Context>) -> Arc<Mutex<Foto>> {
    let foto = Arc::new(Mutex::new(Foto::default()));
    let compartida = Arc::clone(&foto);
    thread::spawn(move || {
        let conexion = match abrir_lector(&ruta) {
            Ok(c) => c,
            Err(e) => {
                if let Ok(mut f) = compartida.lock() {
                    f.error = Some(e.to_string());
                }
                return;
            }
        };
        let mut version = None;
        let mut previa = None;
        loop {
            if pasada(&conexion, &mut version, &mut previa, &compartida) {
                if let Some(ctx) = &contexto {
                    ctx.request_repaint();
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
    });
    foto
}

#[cfg(test)]
pub(crate) fn pasada_de_prueba(ruta: &Path) -> impl FnMut() -> Foto {
    let conexion = abrir_lector(ruta).expect("lector de prueba");
    let foto = Mutex::new(Foto::default());
    let mut version = None;
    let mut previa = None;
    move || {
        pasada(&conexion, &mut version, &mut previa, &foto);
        foto.lock().unwrap().clone()
    }
}
