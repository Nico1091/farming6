//! Reglas de validación de los formularios. Cada función explica qué falló.

use crate::models::DatosPersona;
use crate::reloj::{self, Fecha};

pub type Resultado<T = ()> = Result<T, String>;

pub fn identificacion(texto: &str) -> Resultado<i64> {
    let t = texto.trim();
    if t.is_empty() {
        return Err("La identificación es obligatoria.".into());
    }
    if !t.chars().all(|c| c.is_ascii_digit()) {
        return Err("La identificación solo admite números.".into());
    }
    if !(6..=12).contains(&t.len()) || t.starts_with('0') {
        return Err("La identificación debe tener entre 6 y 12 dígitos y no iniciar en 0.".into());
    }
    t.parse().map_err(|_| "La identificación no es válida.".to_string())
}

fn solo_letras(texto: &str) -> bool {
    texto.chars().all(|c| c.is_alphabetic() || c == ' ' || c == '\'' || c == '-')
}

fn nombre(texto: &str, campo: &str, obligatorio: bool) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return if obligatorio { Err(format!("{campo} es obligatorio.")) } else { Ok(()) };
    }
    if !solo_letras(t) {
        return Err(format!("{campo} solo admite letras."));
    }
    if !(2..=40).contains(&t.chars().count()) {
        return Err(format!("{campo} debe tener entre 2 y 40 letras."));
    }
    Ok(())
}

pub fn telefono(texto: &str) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return Ok(());
    }
    if !t.chars().all(|c| c.is_ascii_digit()) || !(7..=10).contains(&t.len()) {
        return Err("El teléfono debe tener entre 7 y 10 dígitos.".into());
    }
    Ok(())
}

pub fn movil(texto: &str) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return Err("El móvil es obligatorio.".into());
    }
    if t.len() != 10 || !t.starts_with('3') || !t.chars().all(|c| c.is_ascii_digit()) {
        return Err("El móvil debe tener 10 dígitos e iniciar en 3.".into());
    }
    Ok(())
}

pub fn correo(texto: &str) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return Err("El correo es obligatorio.".into());
    }
    let invalido = || Err("El correo no tiene un formato válido, por ejemplo nombre@dominio.com.".to_string());
    if t.len() > 80 || t.chars().any(char::is_whitespace) {
        return invalido();
    }
    let Some((local, dominio)) = t.split_once('@') else {
        return invalido();
    };
    let permitido = |c: char| c.is_ascii_alphanumeric() || "._-+".contains(c);
    if local.is_empty()
        || dominio.contains('@')
        || !local.chars().all(permitido)
        || !dominio.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
        || !dominio.contains('.')
        || dominio.starts_with('.')
        || dominio.ends_with('.')
        || dominio.contains("..")
    {
        return invalido();
    }
    Ok(())
}

pub fn fecha_nacimiento(texto: &str, hoy: Fecha) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return Ok(());
    }
    let formato = || Err("La fecha debe tener el formato AAAA-MM-DD.".to_string());
    let partes: Vec<&str> = t.split('-').collect();
    if partes.len() != 3 || partes[0].len() != 4 || partes[1].len() != 2 || partes[2].len() != 2 {
        return formato();
    }
    let (Ok(anio), Ok(mes), Ok(dia)) =
        (partes[0].parse::<i32>(), partes[1].parse::<u32>(), partes[2].parse::<u32>())
    else {
        return formato();
    };
    if dia == 0 || dia > reloj::dias_del_mes(anio, mes) {
        return Err("Esa fecha no existe en el calendario.".into());
    }
    if anio < 1900 {
        return Err("El año de nacimiento debe ser 1900 o posterior.".into());
    }
    if (Fecha { anio, mes, dia }) > hoy {
        return Err("La fecha de nacimiento no puede ser futura.".into());
    }
    Ok(())
}

/// Primer paso del registro: quién es la persona.
pub fn identidad(d: &DatosPersona, hoy: Fecha) -> Resultado<i64> {
    let id = identificacion(&d.identificacion)?;
    nombre(&d.nom1, "El primer nombre", true)?;
    nombre(&d.nom2, "El segundo nombre", false)?;
    nombre(&d.apell1, "El primer apellido", true)?;
    nombre(&d.apell2, "El segundo apellido", false)?;
    fecha_nacimiento(&d.fecha_nac, hoy)?;
    Ok(id)
}

/// Segundo paso del registro: cómo contactarla.
pub fn contacto(d: &DatosPersona) -> Resultado {
    if d.direccion.trim().chars().count() > 80 {
        return Err("La dirección admite máximo 80 caracteres.".into());
    }
    telefono(&d.tele)?;
    movil(&d.movil)?;
    correo(&d.correo)
}

/// Valida todo el formulario Persona y devuelve la identificación ya convertida.
pub fn persona(d: &DatosPersona, hoy: Fecha) -> Resultado<i64> {
    let id = identidad(d, hoy)?;
    contacto(d)?;
    Ok(id)
}

pub fn descripcion_perfil(texto: &str) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return Err("La descripción del perfil es obligatoria.".into());
    }
    if !solo_letras(t) {
        return Err("La descripción del perfil solo admite letras.".into());
    }
    if !(3..=30).contains(&t.chars().count()) {
        return Err("La descripción debe tener entre 3 y 30 letras.".into());
    }
    Ok(())
}

pub fn nombre_usuario(texto: &str) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return Err("El nombre de usuario es obligatorio.".into());
    }
    if !(4..=20).contains(&t.len()) {
        return Err("El usuario debe tener entre 4 y 20 caracteres.".into());
    }
    let valido = t.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c));
    if !valido || !t.starts_with(|c: char| c.is_ascii_alphabetic()) {
        return Err("El usuario debe iniciar con una letra y solo admite letras, números, punto, guion y guion bajo.".into());
    }
    Ok(())
}

pub fn contrasena(texto: &str) -> Resultado {
    if texto.is_empty() {
        return Err("La contraseña es obligatoria.".into());
    }
    if !(8..=64).contains(&texto.chars().count()) {
        return Err("La contraseña debe tener entre 8 y 64 caracteres.".into());
    }
    let mayuscula = texto.chars().any(char::is_uppercase);
    let minuscula = texto.chars().any(char::is_lowercase);
    let numero = texto.chars().any(|c| c.is_ascii_digit());
    if !(mayuscula && minuscula && numero) {
        return Err("La contraseña debe tener mayúscula, minúscula y número.".into());
    }
    Ok(())
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const HOY: Fecha = Fecha { anio: 2026, mes: 11, dia: 5 };

    fn persona_valida() -> DatosPersona {
        DatosPersona {
            identificacion: "900100200".into(),
            nom1: "Ana".into(),
            nom2: "Ángel".into(),
            apell1: "Prueba".into(),
            apell2: "Núñez".into(),
            direccion: "Bogotá".into(),
            tele: "".into(),
            movil: "3001234567".into(),
            correo: "campesino@ejemplo.com".into(),
            fecha_nac: "2004-08-03".into(),
        }
    }

    #[test]
    fn acepta_persona_correcta() {
        assert_eq!(persona(&persona_valida(), HOY), Ok(900_100_200));
    }

    #[test]
    fn rechaza_identificacion_mala() {
        for mala in ["", "12a456", "12345", "0123456", "1234567890123"] {
            assert!(identificacion(mala).is_err(), "{mala}");
        }
    }

    #[test]
    fn rechaza_nombres_con_numeros() {
        let mut d = persona_valida();
        d.nom1 = "Ni3olas".into();
        assert!(persona(&d, HOY).unwrap_err().contains("solo admite letras"));
    }

    #[test]
    fn exige_los_obligatorios() {
        for (campo, pista) in [("nom1", "primer nombre"), ("apell1", "primer apellido"), ("movil", "móvil"), ("correo", "correo")] {
            let mut d = persona_valida();
            match campo {
                "nom1" => d.nom1.clear(),
                "apell1" => d.apell1.clear(),
                "movil" => d.movil.clear(),
                _ => d.correo.clear(),
            }
            assert!(persona(&d, HOY).unwrap_err().contains(pista), "{campo}");
        }
    }

    #[test]
    fn valida_correos() {
        for bueno in ["a@b.co", "nombre.apellido+x@dominio.edu.co"] {
            assert!(correo(bueno).is_ok(), "{bueno}");
        }
        for malo in ["", "sinarroba.com", "a@b", "a@@b.co", "a b@c.co", "@b.co", "a@.co", "a@b.", "a@b..co"] {
            assert!(correo(malo).is_err(), "{malo}");
        }
    }

    #[test]
    fn valida_fechas() {
        assert!(fecha_nacimiento("", HOY).is_ok());
        assert!(fecha_nacimiento("2004-02-29", HOY).is_ok());
        assert!(fecha_nacimiento("2005-02-29", HOY).unwrap_err().contains("no existe"));
        assert!(fecha_nacimiento("2004-13-01", HOY).unwrap_err().contains("no existe"));
        assert!(fecha_nacimiento("03/08/2004", HOY).unwrap_err().contains("formato"));
        assert!(fecha_nacimiento("2030-01-01", HOY).unwrap_err().contains("futura"));
        assert!(fecha_nacimiento("1800-01-01", HOY).unwrap_err().contains("1900"));
    }

    #[test]
    fn valida_telefonos() {
        assert!(movil("3001234567").is_ok());
        assert!(movil("2001234567").is_err());
        assert!(movil("300123").is_err());
        assert!(telefono("").is_ok());
        assert!(telefono("5551234").is_ok());
        assert!(telefono("55a1234").is_err());
    }

    #[test]
    fn valida_usuario_y_contrasena() {
        assert!(nombre_usuario("ana.prueba").is_ok());
        assert!(nombre_usuario("abc").is_err());
        assert!(nombre_usuario("1abc").is_err());
        assert!(nombre_usuario("con espacio").is_err());
        assert!(contrasena("Campo2026").is_ok());
        assert!(contrasena("corta1A").is_err());
        assert!(contrasena("sinmayuscula1").is_err());
        assert!(contrasena("SINMINUSCULA1").is_err());
        assert!(contrasena("SinNumeroAqui").is_err());
    }

    #[test]
    fn valida_perfil() {
        assert!(descripcion_perfil("Técnico agrícola").is_ok());
        assert!(descripcion_perfil("  ").is_err());
        assert!(descripcion_perfil("ab").is_err());
        assert!(descripcion_perfil("Perfil 2").is_err());
    }
}
