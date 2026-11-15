//! Parcial II: CRUD de granjas, corrales, razas y lotes, y lo que necesitan sus pantallas
//! (municipios por departamento, corrales disponibles, asignaciones de un lote, imagen de una
//! raza). Las consultas se arman con los nombres de tabla y de columna escritos en
//! models/registros.rs; lo que digita el usuario viaja siempre como parámetro.
//! Ninguna función borra filas: inhabilitar cambia el campo estado.

use super::{amable, Database, Resultado};
use crate::models::*;
use rusqlite::types::{ToSqlOutput, ValueRef};
use rusqlite::{params, params_from_iter, OptionalExtension, Row, ToSql};

impl ToSql for Valor {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(match self {
            Valor::Nulo => ToSqlOutput::from(rusqlite::types::Null),
            Valor::Entero(n) => ToSqlOutput::from(*n),
            Valor::Real(x) => ToSqlOutput::from(*x),
            Valor::Texto(t) => ToSqlOutput::from(t.as_str()),
            Valor::Bytes(b) => ToSqlOutput::from(b.as_slice()),
        })
    }
}

fn texto_de(valor: ValueRef) -> String {
    match valor {
        ValueRef::Null => String::new(),
        ValueRef::Integer(n) => n.to_string(),
        ValueRef::Real(x) => decimal(x),
        ValueRef::Text(t) => String::from_utf8_lossy(t).into_owned(),
        ValueRef::Blob(_) => String::new(),
    }
}

/// Formato y peso de la imagen, sin leer la imagen completa: «PNG · 84 KB».
const DESCRIPCION_DE_IMAGEN: &str = "CASE WHEN t.imagen IS NULL THEN ''
    ELSE (CASE WHEN hex(substr(t.imagen, 1, 4)) = '89504E47' THEN 'PNG' ELSE 'JPG' END)
         || ' · ' || ((length(t.imagen) + 1023) / 1024) || ' KB' END";

const NOMBRE_DEL_MUNICIPIO: &str = "(SELECT m.nombre FROM municipio m WHERE m.idmunicipio = t.idmunicipio)";

fn seleccion_de(campo: &Campo) -> String {
    match campo.clase {
        Clase::Imagen => DESCRIPCION_DE_IMAGEN.to_string(),
        _ => format!("t.{}", campo.columna),
    }
}

/// Texto de cada llave foránea, en el orden de los campos.
fn rotulos_foraneos(e: &Entidad) -> Vec<&'static str> {
    e.campos
        .iter()
        .filter_map(|c| match &c.clase {
            Clase::Opcion(fuente) => Some(fuente.rotulo),
            Clase::Municipio => Some(NOMBRE_DEL_MUNICIPIO),
            _ => None,
        })
        .collect()
}

/// SELECT de una entidad: llave, estado, cada campo, el texto de cada llave foránea y las
/// columnas calculadas.
fn consulta_de(e: &Entidad, una_sola: bool) -> String {
    let mut columnas: Vec<String> = vec![format!("t.{}", e.pk), "t.estado".into()];
    columnas.extend(e.campos.iter().map(seleccion_de));
    columnas.extend(rotulos_foraneos(e).into_iter().map(String::from));
    columnas.extend(e.calculadas.iter().map(|c| c.to_string()));
    let mut sql = format!("SELECT {} FROM {} t WHERE (?1 IS NULL OR {})", columnas.join(", "), e.tabla, e.propio);
    if una_sola {
        sql.push_str(&format!(" AND t.{} = ?2", e.pk));
    } else {
        sql.push_str(&format!(" ORDER BY {}", e.orden));
    }
    sql
}

fn registro_de(e: &Entidad, f: &Row) -> rusqlite::Result<Registro> {
    let cuantos = e.campos.len();
    let mut valores = Vec::with_capacity(cuantos);
    let mut rotulos = Vec::with_capacity(cuantos);
    let mut siguiente_rotulo = 2 + cuantos;
    for (i, campo) in e.campos.iter().enumerate() {
        let crudo = texto_de(f.get_ref(2 + i)?);
        let rotulo = match campo.clase {
            Clase::Opcion(_) | Clase::Municipio => {
                let texto = texto_de(f.get_ref(siguiente_rotulo)?);
                siguiente_rotulo += 1;
                texto
            }
            _ => rotulo_de(campo, &crudo),
        };
        valores.push(crudo);
        rotulos.push(rotulo);
    }
    let extras = (0..e.calculadas.len()).map(|i| f.get_ref(siguiente_rotulo + i).map(texto_de)).collect::<Result<_, _>>()?;
    Ok(Registro { id: f.get(0)?, estado: f.get(1)?, valores, rotulos, extras })
}

/// Liga solo los parámetros que la consulta usa: `?1` la persona en sesión y `?2` el otro dato.
fn ligar(consulta: &mut rusqlite::Statement, persona: Option<i64>, otro: Option<i64>) -> rusqlite::Result<()> {
    let cuantos = consulta.parameter_count();
    if cuantos >= 1 {
        consulta.raw_bind_parameter(1, persona)?;
    }
    if cuantos >= 2 {
        consulta.raw_bind_parameter(2, otro)?;
    }
    Ok(())
}

impl Database {
    /// `persona` limita la lectura a lo que puede ver esa persona; sin ella se leen todos.
    pub fn listar_registros(&self, e: &Entidad, persona: Option<i64>) -> Resultado<Vec<Registro>> {
        let mut consulta = self.conexion.prepare(&consulta_de(e, false)).map_err(amable)?;
        let filas = consulta.query_map([persona], |f| registro_de(e, f)).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    pub fn registro_por_id(&self, e: &Entidad, id: i64, persona: Option<i64>) -> Resultado<Option<Registro>> {
        self.conexion
            .query_row(&consulta_de(e, true), params![persona, id], |f| registro_de(e, f))
            .optional()
            .map_err(amable)
    }

    /// INSERT con todos los campos; el estado inicial es Activo (En ceba en el lote). Si llegan
    /// corrales, el lote y sus asignaciones se guardan en la misma transacción.
    pub fn crear_registro(&self, e: &Entidad, valores: &[Valor], corrales: Option<&[(i64, i64)]>, hoy: &str) -> Resultado<i64> {
        let id = valores.first().and_then(Valor::entero).ok_or("Falta el código.")?;
        let columnas: Vec<&str> = e.campos.iter().map(|c| c.columna).collect();
        let marcas: Vec<String> = (1..=columnas.len()).map(|i| format!("?{i}")).collect();
        let sql = format!("INSERT INTO {} ({}, estado) VALUES ({}, 'A')", e.tabla, columnas.join(", "), marcas.join(", "));
        let transaccion = self.conexion.unchecked_transaction().map_err(amable)?;
        transaccion.execute(&sql, params_from_iter(valores.iter())).map_err(amable)?;
        if let Some(corrales) = corrales {
            self.asignar_corrales(id, corrales, hoy)?;
        }
        transaccion.commit().map_err(amable)?;
        Ok(id)
    }

    /// UPDATE solo de los campos que se pueden modificar y del estado.
    pub fn modificar_registro(&self, e: &Entidad, id: i64, valores: &[Valor], estado: &str, corrales: Option<&[(i64, i64)]>, hoy: &str) -> Resultado {
        let cambian: Vec<usize> = (0..e.campos.len()).filter(|&i| e.campos[i].cambia == Cambia::Siempre).collect();
        let mut cambios: Vec<String> = cambian.iter().enumerate().map(|(n, &i)| format!("{} = ?{}", e.campos[i].columna, n + 1)).collect();
        cambios.push(format!("estado = ?{}", cambian.len() + 1));
        let sql = format!("UPDATE {} SET {} WHERE {} = ?{}", e.tabla, cambios.join(", "), e.pk, cambian.len() + 2);
        let mut parametros: Vec<Valor> = cambian.iter().map(|&i| valores[i].clone()).collect();
        parametros.push(Valor::Texto(estado.to_string()));
        parametros.push(Valor::Entero(id));
        let transaccion = self.conexion.unchecked_transaction().map_err(amable)?;
        transaccion.execute(&sql, params_from_iter(parametros.iter())).map_err(amable)?;
        if let Some(corrales) = corrales {
            self.asignar_corrales(id, corrales, hoy)?;
        }
        transaccion.commit().map_err(amable)
    }

    pub fn cambiar_estado_registro(&self, e: &Entidad, id: i64, estado: &str) -> Resultado {
        let sql = format!("UPDATE {} SET estado = ?1 WHERE {} = ?2", e.tabla, e.pk);
        self.conexion.execute(&sql, params![estado, id]).map_err(amable)?;
        Ok(())
    }

    /// Opciones de una llave foránea: código y texto.
    pub fn opciones(&self, fuente: &Fuente, persona: Option<i64>, actual: Option<i64>) -> Resultado<Vec<(i64, String)>> {
        let mut consulta = self.conexion.prepare(fuente.opciones).map_err(amable)?;
        let filas = consulta.query_map(params![persona, actual], |f| Ok((f.get(0)?, f.get(1)?))).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    /// Opciones de un filtro de la lista, como texto para compararlas con los datos de cada fila.
    pub fn opciones_de_filtro(&self, filtro: &Filtro, persona: Option<i64>, depende: Option<i64>) -> Resultado<Vec<(String, String)>> {
        match &filtro.opciones {
            OpcionesFiltro::Fijas(lista) => Ok(lista.iter().map(|(c, n)| (c.to_string(), n.to_string())).collect()),
            OpcionesFiltro::Consulta(sql) => {
                let mut consulta = self.conexion.prepare(sql).map_err(amable)?;
                ligar(&mut consulta, persona, depende).map_err(amable)?;
                let mut filas = consulta.raw_query();
                let mut opciones = Vec::new();
                while let Some(f) = filas.next().map_err(amable)? {
                    opciones.push((texto_de(f.get_ref(0).map_err(amable)?), texto_de(f.get_ref(1).map_err(amable)?)));
                }
                Ok(opciones)
            }
        }
    }

    /// Primera regla que impide inhabilitar el registro, con su mensaje.
    pub fn bloqueo_para_inhabilitar(&self, e: &Entidad, id: i64) -> Resultado<Option<String>> {
        for bloqueo in e.bloqueos {
            let cuantos: i64 = self.conexion.query_row(bloqueo.cuenta, [id], |f| f.get(0)).map_err(amable)?;
            if cuantos > 0 {
                return Ok(Some(bloqueo.mensaje.replace("{n}", &cuantos.to_string())));
            }
        }
        Ok(None)
    }

    // Departamento y municipio (HU-02)

    pub fn departamentos(&self) -> Resultado<Vec<(i64, String)>> {
        let mut consulta = self.conexion.prepare("SELECT iddepartamento, nombre FROM departamento ORDER BY nombre").map_err(amable)?;
        let filas = consulta.query_map([], |f| Ok((f.get(0)?, f.get(1)?))).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    pub fn municipios_de(&self, departamento: i64) -> Resultado<Vec<(i64, String)>> {
        let mut consulta = self
            .conexion
            .prepare("SELECT idmunicipio, nombre FROM municipio WHERE iddepartamento = ?1 ORDER BY nombre")
            .map_err(amable)?;
        let filas = consulta.query_map([departamento], |f| Ok((f.get(0)?, f.get(1)?))).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    pub fn departamento_de_municipio(&self, municipio: i64) -> Resultado<Option<i64>> {
        self.conexion
            .query_row("SELECT iddepartamento FROM municipio WHERE idmunicipio = ?1", [municipio], |f| f.get(0))
            .optional()
            .map_err(amable)
    }

    // Corrales de un lote (HU-03 y HU-05)

    /// Corrales activos de una granja con los cupos que ya ocupan otros lotes en ceba. Incluye los
    /// que hoy tiene asignados el lote `lote`.
    pub fn corrales_disponibles(&self, granja: i64, lote: Option<i64>) -> Resultado<Vec<CorralDisponible>> {
        let mut consulta = self
            .conexion
            .prepare(
                "SELECT c.codigo, c.nombre, c.capacidad,
                        (SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc JOIN lote l ON l.codigol = lc.codigol
                          WHERE lc.codigo_corral = c.codigo AND lc.estado = 'A' AND l.estado = 'A' AND lc.codigol IS NOT ?2),
                        c.estado
                 FROM corral c
                 WHERE c.codigo_granja = ?1
                   AND (c.estado = 'A' OR c.codigo IN (SELECT codigo_corral FROM lote_corral WHERE codigol IS ?2 AND estado = 'A'))
                 ORDER BY c.nombre COLLATE NOCASE",
            )
            .map_err(amable)?;
        let filas = consulta
            .query_map(params![granja, lote], |f| {
                Ok(CorralDisponible {
                    codigo: f.get(0)?,
                    nombre: f.get(1)?,
                    capacidad: f.get(2)?,
                    ocupados: f.get(3)?,
                    activo: f.get::<_, String>(4)? == ACTIVO,
                })
            })
            .map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    /// Corrales que tiene asignados hoy un lote.
    pub fn asignaciones_de_lote(&self, lote: i64) -> Resultado<Vec<Asignacion>> {
        let mut consulta = self
            .conexion
            .prepare(
                "SELECT lc.codigo_corral, c.nombre, c.capacidad, lc.cupos, lc.fecha_asignacion
                 FROM lote_corral lc JOIN corral c ON c.codigo = lc.codigo_corral
                 WHERE lc.codigol = ?1 AND lc.estado = 'A' ORDER BY c.nombre COLLATE NOCASE",
            )
            .map_err(amable)?;
        let filas = consulta
            .query_map([lote], |f| {
                Ok(Asignacion { codigo_corral: f.get(0)?, nombre: f.get(1)?, capacidad: f.get(2)?, cupos: f.get(3)?, fecha: f.get(4)? })
            })
            .map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    /// Deja al lote con exactamente estos corrales y cupos. Primero libera y reduce, después
    /// aumenta y agrega, para que ningún paso intermedio pase la cantidad del lote ni la capacidad
    /// de un corral. Una asignación nueva o reactivada toma la fecha `hoy`; la que sigue conserva
    /// la suya. Se llama dentro de la transacción de crear o modificar el lote.
    fn asignar_corrales(&self, lote: i64, elegidos: &[(i64, i64)], hoy: &str) -> Resultado {
        let mut consulta = self
            .conexion
            .prepare("SELECT codigo_corral, cupos, estado FROM lote_corral WHERE codigol = ?1")
            .map_err(amable)?;
        let actuales: Vec<(i64, i64, String)> = consulta
            .query_map([lote], |f| Ok((f.get(0)?, f.get(1)?, f.get(2)?)))
            .map_err(amable)?
            .collect::<Result<_, _>>()
            .map_err(amable)?;
        let elegido = |corral: i64| elegidos.iter().find(|(c, _)| *c == corral).map(|(_, cupos)| *cupos);
        let ejecutar = |sql: &str, p: &[&dyn ToSql]| self.conexion.execute(sql, p).map(|_| ()).map_err(amable);

        for (corral, cupos, estado) in &actuales {
            if estado != ACTIVO {
                continue;
            }
            match elegido(*corral) {
                None => ejecutar("UPDATE lote_corral SET estado = 'I' WHERE codigol = ?1 AND codigo_corral = ?2", &[&lote, corral])?,
                Some(nuevos) if nuevos < *cupos => ejecutar(
                    "UPDATE lote_corral SET cupos = ?3 WHERE codigol = ?1 AND codigo_corral = ?2",
                    &[&lote, corral, &nuevos],
                )?,
                _ => {}
            }
        }
        for &(corral, cupos) in elegidos {
            match actuales.iter().find(|(c, _, _)| *c == corral) {
                Some((_, antes, estado)) if estado == ACTIVO => {
                    if cupos > *antes {
                        ejecutar("UPDATE lote_corral SET cupos = ?3 WHERE codigol = ?1 AND codigo_corral = ?2", &[&lote, &corral, &cupos])?;
                    }
                }
                Some(_) => ejecutar(
                    "UPDATE lote_corral SET estado = 'A', cupos = ?3, fecha_asignacion = ?4 WHERE codigol = ?1 AND codigo_corral = ?2",
                    &[&lote, &corral, &cupos, &hoy],
                )?,
                None => ejecutar(
                    "INSERT INTO lote_corral (codigol, codigo_corral, fecha_asignacion, cupos, estado) VALUES (?1, ?2, ?3, ?4, 'A')",
                    &[&lote, &corral, &hoy, &cupos],
                )?,
            }
        }
        Ok(())
    }

    /// HU-05: cierra un lote en ceba con su fecha final real.
    pub fn cerrar_lote(&self, lote: i64, fecha: &str) -> Resultado {
        let cambiadas = self
            .conexion
            .execute("UPDATE lote SET estado = 'C', fecha_cierre = ?2 WHERE codigol = ?1 AND estado = 'A'", params![lote, fecha])
            .map_err(amable)?;
        if cambiadas == 0 {
            return Err("Solo se cierra un lote que está en ceba.".into());
        }
        Ok(())
    }

    // Imagen de la raza (HU-04)

    pub fn imagen_de_raza(&self, codigor: i64) -> Resultado<Option<Vec<u8>>> {
        self.conexion
            .query_row("SELECT imagen FROM raza WHERE codigor = ?1", [codigor], |f| f.get::<_, Option<Vec<u8>>>(0))
            .optional()
            .map(Option::flatten)
            .map_err(amable)
    }
}
