//! Reglas de validación de los formularios. Cada función explica qué falló.

use crate::models::{Campo, Clase, DatosPersona, Valor};
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

// Parcial II: productores (HU-01) y campos de granjas, corrales, razas y lotes (HU-02 a HU-05).

/// Teléfono de diez dígitos: celular que empieza por 3 o fijo que empieza por 60, como numera la
/// Comisión de Regulación de Comunicaciones desde septiembre de 2021.
pub fn telefono_diez(texto: &str, obligatorio: bool) -> Resultado {
    let t = texto.trim();
    if t.is_empty() {
        return if obligatorio { Err("El teléfono es obligatorio.".into()) } else { Ok(()) };
    }
    let diez_digitos = t.len() == 10 && t.chars().all(|c| c.is_ascii_digit());
    if !diez_digitos || !(t.starts_with('3') || t.starts_with("60")) {
        return Err("El teléfono debe tener 10 dígitos: un celular que empiece por 3 o un fijo que empiece por 60.".into());
    }
    Ok(())
}

/// Años cumplidos en la fecha `hoy`.
pub fn edad(nacimiento: Fecha, hoy: Fecha) -> i32 {
    let mut anios = hoy.anio - nacimiento.anio;
    if (hoy.mes, hoy.dia) < (nacimiento.mes, nacimiento.dia) {
        anios -= 1;
    }
    anios
}

/// Documento del productor: la llave primaria de persona, con las reglas de la identificación.
fn documento(texto: &str) -> Resultado<i64> {
    let t = texto.trim();
    if t.is_empty() {
        return Err("El documento es obligatorio.".into());
    }
    if !t.chars().all(|c| c.is_ascii_digit()) {
        return Err("El documento solo admite números.".into());
    }
    if !(6..=12).contains(&t.len()) || t.starts_with('0') {
        return Err("El documento debe tener entre 6 y 12 dígitos y no iniciar en 0.".into());
    }
    t.parse().map_err(|_| "El documento no es válido.".to_string())
}

/// HU-01: el formulario de persona con las reglas del productor. Son obligatorios el documento, el
/// primer nombre, el primer apellido, el teléfono, la dirección y la fecha de nacimiento; el
/// productor debe ser mayor de 18 años y el correo es opcional.
pub fn productor(d: &DatosPersona, hoy: Fecha) -> Resultado<i64> {
    let id = documento(&d.identificacion)?;
    nombre(&d.nom1, "El primer nombre", true)?;
    nombre(&d.nom2, "El segundo nombre", false)?;
    nombre(&d.apell1, "El primer apellido", true)?;
    nombre(&d.apell2, "El segundo apellido", false)?;
    telefono_diez(&d.movil, true)?;
    let direccion = d.direccion.trim();
    if direccion.is_empty() {
        return Err("La dirección es obligatoria.".into());
    }
    if direccion.chars().count() > 80 {
        return Err("La dirección admite máximo 80 caracteres.".into());
    }
    if direccion.chars().filter(|c| c.is_alphanumeric()).count() < 3 {
        return Err("La dirección debe tener al menos tres letras o números.".into());
    }
    if d.fecha_nac.trim().is_empty() {
        return Err("La fecha de nacimiento es obligatoria.".into());
    }
    fecha_nacimiento(&d.fecha_nac, hoy)?;
    let anios = edad(leer_fecha(&d.fecha_nac)?, hoy);
    if anios < 18 {
        return Err(format!("El productor debe ser mayor de 18 años; con esa fecha de nacimiento tiene {anios}."));
    }
    if !d.correo.trim().is_empty() {
        correo(&d.correo)?;
    }
    Ok(id)
}

/// Lee una fecha AAAA-MM-DD y comprueba que exista en el calendario.
pub fn leer_fecha(texto: &str) -> Resultado<Fecha> {
    let formato = || Err("La fecha debe tener el formato AAAA-MM-DD, por ejemplo 2026-11-12.".to_string());
    let partes: Vec<&str> = texto.trim().split('-').collect();
    if partes.len() != 3 || partes[0].len() != 4 || partes[1].len() != 2 || partes[2].len() != 2 {
        return formato();
    }
    let (Ok(anio), Ok(mes), Ok(dia)) = (partes[0].parse::<i32>(), partes[1].parse::<u32>(), partes[2].parse::<u32>()) else {
        return formato();
    };
    if dia == 0 || dia > reloj::dias_del_mes(anio, mes) {
        return Err("Esa fecha no existe en el calendario.".into());
    }
    Ok(Fecha { anio, mes, dia })
}

pub fn fecha_texto(f: Fecha) -> String {
    format!("{:04}-{:02}-{:02}", f.anio, f.mes, f.dia)
}

/// Número con coma o punto decimal: «12,5» y «12.5» valen lo mismo.
fn leer_decimal(texto: &str) -> Option<f64> {
    let t = texto.trim().replace(',', ".");
    let forma = t.strip_prefix('-').unwrap_or(&t);
    let valido = !forma.is_empty()
        && forma.chars().all(|c| c.is_ascii_digit() || c == '.')
        && forma.matches('.').count() <= 1
        && forma.chars().any(|c| c.is_ascii_digit());
    if !valido {
        return None;
    }
    t.parse::<f64>().ok().filter(|x| x.is_finite())
}

fn sin_decimales(x: f64) -> String {
    crate::models::decimal(x).replace('.', ",")
}

/// Mayor código que se puede digitar: 999.999.999.
pub const CODIGO_MAXIMO: i64 = 999_999_999;

/// Valida lo digitado en un campo y lo convierte a su tipo de dato.
pub fn valor_de(campo: &Campo, texto: &str, hoy: Fecha) -> Resultado<Valor> {
    let t = texto.trim();
    let nombre = campo.etiqueta;
    if t.is_empty() {
        return if campo.obligatorio { Err(format!("Falta el dato «{nombre}».")) } else { Ok(Valor::Nulo) };
    }
    match &campo.clase {
        Clase::Codigo => {
            let solo_digitos = t.chars().all(|c| c.is_ascii_digit());
            match t.parse::<i64>() {
                Ok(n) if solo_digitos && !t.starts_with('0') && (1..=CODIGO_MAXIMO).contains(&n) => Ok(Valor::Entero(n)),
                _ => Err(format!("«{nombre}» debe ser un número entero de 1 a 999.999.999, sin ceros a la izquierda.")),
            }
        }
        Clase::TextoLibre { max } => {
            if t.chars().count() > *max {
                return Err(format!("«{nombre}» admite máximo {max} caracteres."));
            }
            if t.chars().any(char::is_control) {
                return Err(format!("«{nombre}» tiene caracteres que no se pueden guardar."));
            }
            Ok(Valor::Texto(t.to_string()))
        }
        Clase::Telefono => {
            telefono_diez(t, true)?;
            Ok(Valor::Texto(t.to_string()))
        }
        Clase::Correo => {
            correo(t)?;
            Ok(Valor::Texto(t.to_lowercase()))
        }
        Clase::Municipio => match t.parse::<i64>() {
            Ok(id) if id > 0 => Ok(Valor::Entero(id)),
            _ => Err("Elija el departamento y luego el municipio de la lista.".into()),
        },
        Clase::Imagen => Err(format!("«{nombre}» se elige con «Elegir imagen» o arrastrando el archivo.")),
        Clase::Texto { min, max } => {
            let largo = t.chars().count();
            if largo < *min || largo > *max {
                return Err(format!("«{nombre}» debe tener entre {min} y {max} caracteres."));
            }
            if !t.chars().any(char::is_alphabetic) {
                return Err(format!("«{nombre}» debe llevar letras, no solo números o signos."));
            }
            if t.chars().any(char::is_control) {
                return Err(format!("«{nombre}» tiene caracteres que no se pueden guardar."));
            }
            Ok(Valor::Texto(t.to_string()))
        }
        Clase::Entero { min, max } => {
            if !t.chars().all(|c| c.is_ascii_digit()) {
                return Err(format!("«{nombre}» solo admite números enteros, sin puntos ni letras."));
            }
            match t.parse::<i64>() {
                Ok(n) if (*min..=*max).contains(&n) => Ok(Valor::Entero(n)),
                _ => Err(format!("«{nombre}» debe estar entre {min} y {max}.")),
            }
        }
        Clase::Decimal { min, max } => {
            let Some(x) = leer_decimal(t) else {
                return Err(format!("«{nombre}» debe ser un número, por ejemplo 350 o 12,5."));
            };
            if x < *min || x > *max {
                return Err(format!("«{nombre}» debe estar entre {} y {}.", sin_decimales(*min), sin_decimales(*max)));
            }
            Ok(Valor::Real(x))
        }
        Clase::Fecha { futura } => {
            let fecha = leer_fecha(t).map_err(|e| format!("«{nombre}»: {}", minuscula_inicial(&e)))?;
            if fecha.anio < 2000 {
                return Err(format!("«{nombre}» debe ser del año 2000 en adelante."));
            }
            if !futura && fecha > hoy {
                return Err(format!("«{nombre}» no puede ser una fecha futura."));
            }
            if fecha.anio > 2100 {
                return Err(format!("«{nombre}» debe ser anterior al año 2101."));
            }
            Ok(Valor::Texto(fecha_texto(fecha)))
        }
        Clase::Opcion(_) => match t.parse::<i64>() {
            Ok(id) if id > 0 => Ok(Valor::Entero(id)),
            _ => Err(format!("Elija «{nombre}» de la lista.")),
        },
    }
}

fn minuscula_inicial(texto: &str) -> String {
    let mut letras = texto.chars();
    match letras.next() {
        Some(primera) => primera.to_lowercase().chain(letras).collect(),
        None => String::new(),
    }
}

/// Convierte un valor ya guardado sin volver a validarlo. Sirve para los campos que no se
/// modifican, cuyo valor se lee de la base.
pub fn valor_guardado(campo: &Campo, texto: &str) -> Valor {
    let t = texto.trim();
    if t.is_empty() {
        return Valor::Nulo;
    }
    match campo.clase {
        Clase::Codigo | Clase::Entero { .. } | Clase::Opcion(_) | Clase::Municipio => {
            t.parse::<i64>().map(Valor::Entero).unwrap_or_else(|_| Valor::Texto(t.to_string()))
        }
        Clase::Decimal { .. } => t.parse::<f64>().map(Valor::Real).unwrap_or_else(|_| Valor::Texto(t.to_string())),
        _ => Valor::Texto(t.to_string()),
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    const HOY: Fecha = Fecha { anio: 2026, mes: 11, dia: 5 };

    fn persona_valida() -> DatosPersona {
        DatosPersona {
            identificacion: "1091352918".into(),
            nom1: "Nicolás".into(),
            nom2: "Ángel".into(),
            apell1: "Rojas".into(),
            apell2: "Yáñez".into(),
            direccion: "Cúcuta".into(),
            tele: "".into(),
            movil: "3001234567".into(),
            correo: "campesino@ejemplo.com".into(),
            fecha_nac: "2004-08-03".into(),
        }
    }

    #[test]
    fn acepta_persona_correcta() {
        assert_eq!(persona(&persona_valida(), HOY), Ok(1_091_352_918));
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
        assert!(nombre_usuario("nicolas.rojas").is_ok());
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

    fn productor_valido() -> DatosPersona {
        DatosPersona {
            identificacion: "88123456".into(),
            nom1: "Ramiro".into(),
            apell1: "Peñaranda".into(),
            movil: "3157778899".into(),
            direccion: "Vereda Agua Linda, finca El Porvenir".into(),
            fecha_nac: "1980-06-15".into(),
            ..Default::default()
        }
    }

    #[test]
    fn hu01_el_productor_sin_correo_ni_segundos_nombres_es_valido() {
        assert_eq!(productor(&productor_valido(), HOY), Ok(88_123_456));
        let mut con_correo = productor_valido();
        con_correo.correo = "ramiro@ejemplo.com".into();
        assert!(productor(&con_correo, HOY).is_ok());
        con_correo.correo = "ramiro@".into();
        assert!(productor(&con_correo, HOY).unwrap_err().contains("correo"));
    }

    #[test]
    fn hu01_exige_documento_nombre_apellido_telefono_direccion_y_fecha() {
        let casos: [(&str, fn(&mut DatosPersona), &str); 6] = [
            ("documento", |d| d.identificacion.clear(), "documento es obligatorio"),
            ("nombre1", |d| d.nom1.clear(), "primer nombre"),
            ("apellido1", |d| d.apell1.clear(), "primer apellido"),
            ("teléfono", |d| d.movil.clear(), "teléfono es obligatorio"),
            ("dirección", |d| d.direccion.clear(), "dirección es obligatoria"),
            ("fecha_nac", |d| d.fecha_nac.clear(), "fecha de nacimiento es obligatoria"),
        ];
        for (campo, vaciar, pista) in casos {
            let mut d = productor_valido();
            vaciar(&mut d);
            let error = productor(&d, HOY).unwrap_err();
            assert!(error.contains(pista), "{campo}: {error}");
        }
    }

    #[test]
    fn hu01_el_productor_debe_ser_mayor_de_18() {
        let mut d = productor_valido();
        d.fecha_nac = "2008-11-06".into();
        assert!(productor(&d, HOY).unwrap_err().contains("mayor de 18"), "cumple 18 mañana");
        d.fecha_nac = "2008-11-05".into();
        assert!(productor(&d, HOY).is_ok(), "cumple 18 hoy");
        d.fecha_nac = "2030-01-01".into();
        assert!(productor(&d, HOY).unwrap_err().contains("futura"));
        assert_eq!(edad(Fecha { anio: 2004, mes: 2, dia: 29 }, Fecha { anio: 2022, mes: 2, dia: 28 }), 17);
        assert_eq!(edad(Fecha { anio: 2004, mes: 2, dia: 29 }, Fecha { anio: 2022, mes: 3, dia: 1 }), 18);
    }

    #[test]
    fn hu01_el_telefono_tiene_diez_digitos() {
        for bueno in ["3001234567", "6075712345"] {
            assert!(telefono_diez(bueno, true).is_ok(), "{bueno}");
        }
        for malo in ["300123456", "30012345678", "5551234567", "300-123-45", "7012345678"] {
            assert!(telefono_diez(malo, true).is_err(), "{malo}");
        }
        assert!(telefono_diez("", false).is_ok());
        assert!(telefono_diez("", true).is_err());
    }

    #[test]
    fn hu01_el_documento_solo_admite_numeros() {
        let mut d = productor_valido();
        d.identificacion = "88-123".into();
        assert!(productor(&d, HOY).unwrap_err().contains("solo admite números"));
        d.identificacion = "012345".into();
        assert!(productor(&d, HOY).unwrap_err().contains("no iniciar en 0"));
    }
}
