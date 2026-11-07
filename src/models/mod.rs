//! Estructuras del modelo relacional del parcial: Perfil, Persona y usuario.

pub const ACTIVO: &str = "A";
pub const INACTIVO: &str = "I";

pub fn estado_de(activo: bool) -> &'static str {
    if activo {
        ACTIVO
    } else {
        INACTIVO
    }
}

pub fn estado_texto(estado: &str) -> &'static str {
    if estado == ACTIVO {
        "Activo"
    } else {
        "Inhabilitado"
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Perfil {
    pub idperfil: i64,
    pub descripc: String,
    pub estado: String,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Persona {
    pub idpersona: i64,
    pub nom1: String,
    pub nom2: String,
    pub apell1: String,
    pub apell2: String,
    pub direccion: String,
    pub tele: String,
    pub movil: String,
    pub correo: String,
    pub fecha_nac: String,
    pub estado: String,
}

impl Persona {
    pub fn nombre_completo(&self) -> String {
        [&self.nom1, &self.nom2, &self.apell1, &self.apell2]
            .iter()
            .filter(|parte| !parte.is_empty())
            .map(|parte| parte.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Usuario {
    /// Es la misma identificación de la persona: así queda la relación 1:1 del modelo.
    pub idusuario: i64,
    pub nombreu: String,
    pub contrasena: String,
    pub idperfil: i64,
    pub estado: String,
}

/// Fila del formulario Gestión Usuario, con los nombres de sus tablas relacionadas.
#[derive(Debug, Clone, PartialEq)]
pub struct UsuarioVista {
    pub usuario: Usuario,
    pub perfil: String,
    pub persona: String,
}

/// Campos de texto del formulario Persona, tal como los digita el usuario.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DatosPersona {
    pub identificacion: String,
    pub nom1: String,
    pub nom2: String,
    pub apell1: String,
    pub apell2: String,
    pub direccion: String,
    pub tele: String,
    pub movil: String,
    pub correo: String,
    pub fecha_nac: String,
}

impl DatosPersona {
    pub fn desde(p: &Persona) -> Self {
        Self {
            identificacion: p.idpersona.to_string(),
            nom1: p.nom1.clone(),
            nom2: p.nom2.clone(),
            apell1: p.apell1.clone(),
            apell2: p.apell2.clone(),
            direccion: p.direccion.clone(),
            tele: p.tele.clone(),
            movil: p.movil.clone(),
            correo: p.correo.clone(),
            fecha_nac: p.fecha_nac.clone(),
        }
    }

    pub fn a_persona(&self, idpersona: i64, estado: &str) -> Persona {
        Persona {
            idpersona,
            nom1: self.nom1.trim().to_string(),
            nom2: self.nom2.trim().to_string(),
            apell1: self.apell1.trim().to_string(),
            apell2: self.apell2.trim().to_string(),
            direccion: self.direccion.trim().to_string(),
            tele: self.tele.trim().to_string(),
            movil: self.movil.trim().to_string(),
            correo: self.correo.trim().to_lowercase(),
            fecha_nac: self.fecha_nac.trim().to_string(),
            estado: estado.to_string(),
        }
    }
}
