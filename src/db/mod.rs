//! Capa de datos: conexión a SQLite y CRUD de las tres tablas del modelo.
//! Todas las consultas van parametrizadas y ninguna función borra filas: inhabilitar
//! solo cambia el campo estado.

pub mod vigia;

use crate::models::*;
use rusqlite::{params, Connection, ErrorCode, OptionalExtension, Row};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub type Resultado<T = ()> = Result<T, String>;

const ESQUEMA: &str = include_str!("../../migrations/001_init.sql");

pub struct Database {
    conexion: Connection,
    pub ruta: PathBuf,
}

/// Traduce los errores del motor a un mensaje para el usuario.
fn amable(error: rusqlite::Error) -> String {
    if let rusqlite::Error::SqliteFailure(fallo, detalle) = &error {
        let texto = detalle.as_deref().unwrap_or("");
        if fallo.code == ErrorCode::ConstraintViolation {
            return match texto {
                t if t.contains("persona.correo") => "Ese correo ya está registrado.".into(),
                t if t.contains("persona.idpersona") => "Esa identificación ya está registrada.".into(),
                t if t.contains("usuario.nombreu") => "Ese nombre de usuario ya existe.".into(),
                t if t.contains("usuario.idusuario") => "Esa persona ya tiene un usuario.".into(),
                t if t.contains("perfil.descripc") => "Ya existe un perfil con esa descripción.".into(),
                t if t.contains("FOREIGN KEY") => "El registro relacionado no existe.".into(),
                _ => format!("La base de datos rechazó el registro: {texto}"),
            };
        }
    }
    format!("Error de base de datos: {error}")
}

fn perfil_de(f: &Row) -> rusqlite::Result<Perfil> {
    Ok(Perfil { idperfil: f.get(0)?, descripc: f.get(1)?, estado: f.get(2)? })
}

fn persona_de(f: &Row) -> rusqlite::Result<Persona> {
    let opcional = |i: usize| -> rusqlite::Result<String> {
        Ok(f.get::<_, Option<String>>(i)?.unwrap_or_default())
    };
    Ok(Persona {
        idpersona: f.get(0)?,
        nom1: f.get(1)?,
        nom2: opcional(2)?,
        apell1: f.get(3)?,
        apell2: opcional(4)?,
        direccion: opcional(5)?,
        tele: opcional(6)?,
        movil: f.get(7)?,
        correo: f.get(8)?,
        fecha_nac: opcional(9)?,
        estado: f.get(10)?,
    })
}

fn usuario_de(f: &Row) -> rusqlite::Result<Usuario> {
    Ok(Usuario {
        idusuario: f.get(0)?,
        nombreu: f.get(1)?,
        contrasena: f.get(2)?,
        idperfil: f.get(3)?,
        estado: f.get(4)?,
    })
}

/// Guarda NULL en lugar de texto vacío en los campos opcionales.
fn vacio_a_nulo(texto: &str) -> Option<&str> {
    if texto.is_empty() {
        None
    } else {
        Some(texto)
    }
}

const CAMPOS_PERSONA: &str =
    "idpersona, nom1, nom2, apell1, apell2, direccion, tele, movil, correo, fecha_nac, estado";

impl Database {
    pub fn abrir(ruta: &Path) -> Resultado<Self> {
        let conexion = Connection::open(ruta).map_err(amable)?;
        conexion.busy_timeout(Duration::from_secs(4)).map_err(amable)?;
        conexion
            .query_row("PRAGMA journal_mode = WAL", [], |_| Ok(()))
            .map_err(amable)?;
        conexion.execute_batch("PRAGMA foreign_keys = ON;").map_err(amable)?;
        conexion.execute_batch(ESQUEMA).map_err(amable)?;
        Ok(Self { conexion, ruta: ruta.to_path_buf() })
    }

    fn contar(&self, sql: &str, parametros: impl rusqlite::Params) -> Resultado<i64> {
        self.conexion.query_row(sql, parametros, |f| f.get(0)).map_err(amable)
    }

    // Perfil

    pub fn listar_perfiles(&self) -> Resultado<Vec<Perfil>> {
        let mut consulta = self
            .conexion
            .prepare("SELECT idperfil, descripc, estado FROM perfil ORDER BY idperfil")
            .map_err(amable)?;
        let filas = consulta.query_map([], perfil_de).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    pub fn perfil_por_id(&self, id: i64) -> Resultado<Option<Perfil>> {
        self.conexion
            .query_row("SELECT idperfil, descripc, estado FROM perfil WHERE idperfil = ?1", [id], perfil_de)
            .optional()
            .map_err(amable)
    }

    pub fn perfil_por_descripcion(&self, descripc: &str) -> Resultado<Option<Perfil>> {
        self.conexion
            .query_row(
                "SELECT idperfil, descripc, estado FROM perfil WHERE descripc = ?1",
                [descripc],
                perfil_de,
            )
            .optional()
            .map_err(amable)
    }

    pub fn crear_perfil(&self, descripc: &str, estado: &str) -> Resultado<i64> {
        self.conexion
            .execute("INSERT INTO perfil (descripc, estado) VALUES (?1, ?2)", params![descripc, estado])
            .map_err(amable)?;
        Ok(self.conexion.last_insert_rowid())
    }

    pub fn modificar_perfil(&self, id: i64, descripc: &str, estado: &str) -> Resultado {
        self.conexion
            .execute(
                "UPDATE perfil SET descripc = ?1, estado = ?2 WHERE idperfil = ?3",
                params![descripc, estado, id],
            )
            .map_err(amable)?;
        Ok(())
    }

    pub fn cambiar_estado_perfil(&self, id: i64, estado: &str) -> Resultado {
        self.conexion
            .execute("UPDATE perfil SET estado = ?1 WHERE idperfil = ?2", params![estado, id])
            .map_err(amable)?;
        Ok(())
    }

    pub fn usuarios_activos_de_perfil(&self, id: i64) -> Resultado<i64> {
        self.contar("SELECT COUNT(*) FROM usuario WHERE idperfil = ?1 AND estado = 'A'", [id])
    }

    // Persona

    pub fn listar_personas(&self) -> Resultado<Vec<Persona>> {
        let sql = format!("SELECT {CAMPOS_PERSONA} FROM persona ORDER BY apell1, nom1");
        let mut consulta = self.conexion.prepare(&sql).map_err(amable)?;
        let filas = consulta.query_map([], persona_de).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    pub fn persona_por_id(&self, id: i64) -> Resultado<Option<Persona>> {
        let sql = format!("SELECT {CAMPOS_PERSONA} FROM persona WHERE idpersona = ?1");
        self.conexion.query_row(&sql, [id], persona_de).optional().map_err(amable)
    }

    pub fn persona_por_correo(&self, correo: &str) -> Resultado<Option<Persona>> {
        let sql = format!("SELECT {CAMPOS_PERSONA} FROM persona WHERE correo = ?1");
        self.conexion.query_row(&sql, [correo], persona_de).optional().map_err(amable)
    }

    pub fn crear_persona(&self, p: &Persona) -> Resultado {
        self.conexion
            .execute(
                &format!("INSERT INTO persona ({CAMPOS_PERSONA}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"),
                params![
                    p.idpersona,
                    p.nom1,
                    vacio_a_nulo(&p.nom2),
                    p.apell1,
                    vacio_a_nulo(&p.apell2),
                    vacio_a_nulo(&p.direccion),
                    vacio_a_nulo(&p.tele),
                    p.movil,
                    p.correo,
                    vacio_a_nulo(&p.fecha_nac),
                    p.estado
                ],
            )
            .map_err(amable)?;
        Ok(())
    }

    pub fn modificar_persona(&self, p: &Persona) -> Resultado {
        self.conexion
            .execute(
                "UPDATE persona SET nom1 = ?2, nom2 = ?3, apell1 = ?4, apell2 = ?5, direccion = ?6, tele = ?7,
                 movil = ?8, correo = ?9, fecha_nac = ?10, estado = ?11 WHERE idpersona = ?1",
                params![
                    p.idpersona,
                    p.nom1,
                    vacio_a_nulo(&p.nom2),
                    p.apell1,
                    vacio_a_nulo(&p.apell2),
                    vacio_a_nulo(&p.direccion),
                    vacio_a_nulo(&p.tele),
                    p.movil,
                    p.correo,
                    vacio_a_nulo(&p.fecha_nac),
                    p.estado
                ],
            )
            .map_err(amable)?;
        Ok(())
    }

    pub fn cambiar_estado_persona(&self, id: i64, estado: &str) -> Resultado {
        self.conexion
            .execute("UPDATE persona SET estado = ?1 WHERE idpersona = ?2", params![estado, id])
            .map_err(amable)?;
        Ok(())
    }

    /// Personas activas que todavía no tienen usuario.
    pub fn personas_sin_usuario(&self) -> Resultado<Vec<Persona>> {
        let sql = format!(
            "SELECT {CAMPOS_PERSONA} FROM persona
             WHERE estado = 'A' AND idpersona NOT IN (SELECT idusuario FROM usuario)
             ORDER BY apell1, nom1"
        );
        let mut consulta = self.conexion.prepare(&sql).map_err(amable)?;
        let filas = consulta.query_map([], persona_de).map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    // Usuario

    pub fn listar_usuarios(&self) -> Resultado<Vec<UsuarioVista>> {
        let mut consulta = self
            .conexion
            .prepare(
                "SELECT u.idusuario, u.nombreu, u.contrasena, u.idperfil, u.estado, f.descripc,
                        p.nom1 || ' ' || p.apell1
                 FROM usuario u
                 JOIN perfil f ON f.idperfil = u.idperfil
                 JOIN persona p ON p.idpersona = u.idusuario
                 ORDER BY u.nombreu",
            )
            .map_err(amable)?;
        let filas = consulta
            .query_map([], |f| {
                Ok(UsuarioVista { usuario: usuario_de(f)?, perfil: f.get(5)?, persona: f.get(6)? })
            })
            .map_err(amable)?;
        filas.collect::<Result<_, _>>().map_err(amable)
    }

    pub fn usuario_por_id(&self, id: i64) -> Resultado<Option<Usuario>> {
        self.conexion
            .query_row(
                "SELECT idusuario, nombreu, contrasena, idperfil, estado FROM usuario WHERE idusuario = ?1",
                [id],
                usuario_de,
            )
            .optional()
            .map_err(amable)
    }

    pub fn usuario_por_nombre(&self, nombreu: &str) -> Resultado<Option<Usuario>> {
        self.conexion
            .query_row(
                "SELECT idusuario, nombreu, contrasena, idperfil, estado FROM usuario WHERE nombreu = ?1",
                [nombreu],
                usuario_de,
            )
            .optional()
            .map_err(amable)
    }

    pub fn crear_usuario(&self, u: &Usuario) -> Resultado {
        self.conexion
            .execute(
                "INSERT INTO usuario (idusuario, nombreu, contrasena, idperfil, estado)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![u.idusuario, u.nombreu, u.contrasena, u.idperfil, u.estado],
            )
            .map_err(amable)?;
        Ok(())
    }

    pub fn modificar_usuario(&self, id: i64, nombreu: &str, idperfil: i64, estado: &str) -> Resultado {
        self.conexion
            .execute(
                "UPDATE usuario SET nombreu = ?1, idperfil = ?2, estado = ?3 WHERE idusuario = ?4",
                params![nombreu, idperfil, estado, id],
            )
            .map_err(amable)?;
        Ok(())
    }

    pub fn cambiar_contrasena(&self, id: i64, hash: &str) -> Resultado {
        self.conexion
            .execute("UPDATE usuario SET contrasena = ?1 WHERE idusuario = ?2", params![hash, id])
            .map_err(amable)?;
        Ok(())
    }

    pub fn cambiar_estado_usuario(&self, id: i64, estado: &str) -> Resultado {
        self.conexion
            .execute("UPDATE usuario SET estado = ?1 WHERE idusuario = ?2", params![estado, id])
            .map_err(amable)?;
        Ok(())
    }

    pub fn total_usuarios(&self) -> Resultado<i64> {
        self.contar("SELECT COUNT(*) FROM usuario", [])
    }

    /// Administradores que hoy pueden entrar: usuario, persona y perfil activos.
    pub fn administradores_activos(&self) -> Resultado<i64> {
        self.contar(
            "SELECT COUNT(*) FROM usuario u
             JOIN perfil f ON f.idperfil = u.idperfil
             JOIN persona p ON p.idpersona = u.idusuario
             WHERE u.estado = 'A' AND p.estado = 'A' AND f.estado = 'A'
               AND f.descripc = 'Administrador'",
            [],
        )
    }

    /// Crea persona y usuario juntos: si uno falla, no queda ninguno.
    pub fn crear_persona_con_usuario(&mut self, p: &Persona, u: &Usuario) -> Resultado {
        let transaccion = self.conexion.transaction().map_err(amable)?;
        transaccion
            .execute(
                &format!("INSERT INTO persona ({CAMPOS_PERSONA}) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)"),
                params![
                    p.idpersona,
                    p.nom1,
                    vacio_a_nulo(&p.nom2),
                    p.apell1,
                    vacio_a_nulo(&p.apell2),
                    vacio_a_nulo(&p.direccion),
                    vacio_a_nulo(&p.tele),
                    p.movil,
                    p.correo,
                    vacio_a_nulo(&p.fecha_nac),
                    p.estado
                ],
            )
            .map_err(amable)?;
        transaccion
            .execute(
                "INSERT INTO usuario (idusuario, nombreu, contrasena, idperfil, estado)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![u.idusuario, u.nombreu, u.contrasena, u.idperfil, u.estado],
            )
            .map_err(amable)?;
        transaccion.commit().map_err(amable)
    }
}
