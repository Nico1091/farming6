//! Envío real del correo de recuperación y guardado cifrado de la cuenta remitente.
//!
//! El correo sale con curl.exe, que viene con Windows y valida el certificado del servidor.
//! La clave llega a curl por una variable de entorno de ese proceso, no por la línea de
//! comandos. En disco queda cifrada con DPAPI: solo este usuario de Windows la puede abrir.

use crate::reloj;
use std::ffi::c_void;
use std::fs;
use std::io::Write;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::ptr;

pub const SERVIDOR_GMAIL: &str = "smtps://smtp.gmail.com:465";
const SIN_VENTANA: u32 = 0x0800_0000;
const CUENTA_ARCHIVO: &str = "correo.cfg";

#[derive(Clone, Debug, PartialEq)]
pub struct Cuenta {
    pub correo: String,
    pub clave: String,
}

#[repr(C)]
struct Bloque {
    largo: u32,
    datos: *mut u8,
}

#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        entrada: *const Bloque,
        descripcion: *const u16,
        entropia: *const Bloque,
        reservado: *mut c_void,
        aviso: *mut c_void,
        banderas: u32,
        salida: *mut Bloque,
    ) -> i32;
    fn CryptUnprotectData(
        entrada: *const Bloque,
        descripcion: *mut *mut u16,
        entropia: *const Bloque,
        reservado: *mut c_void,
        aviso: *mut c_void,
        banderas: u32,
        salida: *mut Bloque,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(memoria: *mut c_void) -> *mut c_void;
}

const SIN_INTERFAZ: u32 = 1;

fn copiar_y_liberar(bloque: Bloque) -> Vec<u8> {
    // SAFETY: Windows entrega un búfer válido de `largo` bytes que se libera con LocalFree.
    unsafe {
        let copia = std::slice::from_raw_parts(bloque.datos, bloque.largo as usize).to_vec();
        LocalFree(bloque.datos.cast());
        copia
    }
}

fn proteger(datos: &[u8]) -> Result<Vec<u8>, String> {
    let entrada = Bloque { largo: datos.len() as u32, datos: datos.as_ptr() as *mut u8 };
    let mut salida = Bloque { largo: 0, datos: ptr::null_mut() };
    // SAFETY: los punteros son válidos durante la llamada; Windows no modifica la entrada.
    let correcto = unsafe {
        CryptProtectData(&entrada, ptr::null(), ptr::null(), ptr::null_mut(), ptr::null_mut(), SIN_INTERFAZ, &mut salida)
    };
    if correcto == 0 {
        return Err("Windows no pudo cifrar la cuenta de correo.".into());
    }
    Ok(copiar_y_liberar(salida))
}

fn desproteger(datos: &[u8]) -> Option<Vec<u8>> {
    let entrada = Bloque { largo: datos.len() as u32, datos: datos.as_ptr() as *mut u8 };
    let mut salida = Bloque { largo: 0, datos: ptr::null_mut() };
    // SAFETY: igual que en `proteger`.
    let correcto = unsafe {
        CryptUnprotectData(&entrada, ptr::null_mut(), ptr::null(), ptr::null_mut(), ptr::null_mut(), SIN_INTERFAZ, &mut salida)
    };
    (correcto != 0).then(|| copiar_y_liberar(salida))
}

fn archivo(carpeta: &Path) -> PathBuf {
    carpeta.join(CUENTA_ARCHIVO)
}

pub fn guardar(carpeta: &Path, cuenta: &Cuenta) -> Result<(), String> {
    let cifrado = proteger(format!("{}\n{}", cuenta.correo, cuenta.clave).as_bytes())?;
    fs::write(archivo(carpeta), cifrado).map_err(|e| format!("No se pudo guardar la cuenta: {e}"))
}

pub fn cargar(carpeta: &Path) -> Option<Cuenta> {
    let cifrado = fs::read(archivo(carpeta)).ok()?;
    let plano = String::from_utf8(desproteger(&cifrado)?).ok()?;
    let (correo, clave) = plano.split_once('\n')?;
    Some(Cuenta { correo: correo.to_string(), clave: clave.to_string() })
}

/// Las claves de aplicación de Google se muestran en grupos separados por espacios.
pub fn limpiar_clave(clave: &str) -> String {
    clave.chars().filter(|c| !c.is_whitespace()).collect()
}

fn base64(datos: &[u8]) -> String {
    const ALFABETO: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut salida = String::with_capacity(datos.len() / 3 * 4 + 4);
    for trozo in datos.chunks(3) {
        let n = (u32::from(trozo[0]) << 16)
            | (u32::from(*trozo.get(1).unwrap_or(&0)) << 8)
            | u32::from(*trozo.get(2).unwrap_or(&0));
        salida.push(ALFABETO[(n >> 18) as usize & 63] as char);
        salida.push(ALFABETO[(n >> 12) as usize & 63] as char);
        salida.push(if trozo.len() > 1 { ALFABETO[(n >> 6) as usize & 63] as char } else { '=' });
        salida.push(if trozo.len() > 2 { ALFABETO[n as usize & 63] as char } else { '=' });
    }
    salida
}

/// Base64 cortado en líneas de 76 caracteres, como pide el formato de correo.
fn base64_en_lineas(datos: &[u8]) -> String {
    base64(datos)
        .as_bytes()
        .chunks(76)
        .map(|linea| String::from_utf8_lossy(linea).into_owned())
        .collect::<Vec<_>>()
        .join("\r\n")
}

fn fecha_de_correo() -> String {
    const DIAS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
    const MESES: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
    let m = reloj::ahora();
    format!(
        "{}, {:02} {} {} {} -0500",
        DIAS[m.dia_semana],
        m.fecha.dia,
        MESES[m.fecha.mes as usize - 1],
        m.fecha.anio,
        m.hora_texto()
    )
}

fn escapar_html(texto: &str) -> String {
    texto.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn cuerpo_recuperacion(nombre: &str, usuario: &str, temporal: &str) -> String {
    format!(
        r#"<div style="font-family:Segoe UI,Arial,sans-serif;background:#f5f0e6;padding:24px">
<div style="max-width:520px;margin:auto;background:#ffffff;border-radius:12px;overflow:hidden">
<div style="background:#1e4623;color:#ffffff;padding:20px 24px">
<div style="font-size:22px;font-weight:600">Farming 6</div>
<div style="font-size:14px">Costos y precio de venta del ganado</div></div>
<div style="padding:24px;color:#372819;font-size:15px;line-height:1.5">
<p>Hola, {nombre}.</p>
<p>Recibimos una solicitud para recuperar su acceso al sistema.</p>
<p style="background:#eef5ea;border-left:4px solid #38703c;padding:12px 16px">
<b>Usuario:</b> {usuario}<br><b>Contraseña temporal:</b> {temporal}</p>
<p>Esta contraseña reemplaza a la anterior. Si usted no pidió este cambio, avise al administrador.</p>
</div></div></div>"#,
        nombre = escapar_html(nombre),
        usuario = escapar_html(usuario),
        temporal = escapar_html(temporal),
    )
}

/// Arma el mensaje completo en el formato de correo de internet.
pub fn mensaje_recuperacion(remitente: &str, destino: &str, nombre: &str, usuario: &str, temporal: &str) -> String {
    let asunto = base64("Farming 6: recuperación de usuario y contraseña".as_bytes());
    let cuerpo = base64_en_lineas(cuerpo_recuperacion(nombre, usuario, temporal).as_bytes());
    format!(
        "From: Farming 6 <{remitente}>\r\nTo: <{destino}>\r\nSubject: =?UTF-8?B?{asunto}?=\r\nDate: {fecha}\r\n\
         MIME-Version: 1.0\r\nContent-Type: text/html; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{cuerpo}\r\n",
        fecha = fecha_de_correo(),
    )
}

pub fn mensaje_de_prueba(remitente: &str, destino: &str) -> String {
    let asunto = base64("Farming 6: prueba de correo".as_bytes());
    let cuerpo = base64_en_lineas("Si recibió este mensaje, el correo de Farming 6 quedó bien configurado.".as_bytes());
    format!(
        "From: Farming 6 <{remitente}>\r\nTo: <{destino}>\r\nSubject: =?UTF-8?B?{asunto}?=\r\nDate: {fecha}\r\n\
         MIME-Version: 1.0\r\nContent-Type: text/plain; charset=UTF-8\r\nContent-Transfer-Encoding: base64\r\n\r\n{cuerpo}\r\n",
        fecha = fecha_de_correo(),
    )
}

fn curl() -> PathBuf {
    let raiz = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    Path::new(&raiz).join("System32").join("curl.exe")
}

/// Envía el mensaje y devuelve el error real si el servidor no lo aceptó.
pub fn enviar(servidor: &str, cuenta: Option<&Cuenta>, remitente: &str, destino: &str, mensaje: &str) -> Result<(), String> {
    let mut orden = Command::new(curl());
    orden
        .creation_flags(SIN_VENTANA)
        .args(["--silent", "--show-error", "--max-time", "30", "--url", servidor])
        .args(["--mail-from", remitente, "--mail-rcpt", destino, "--upload-file", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    if servidor.starts_with("smtps://") {
        orden.arg("--ssl-reqd");
    }
    if let Some(c) = cuenta {
        orden
            .env("FARMING6_CREDENCIAL", format!("{}:{}", c.correo, c.clave))
            .args(["--variable", "%FARMING6_CREDENCIAL", "--expand-user", "{{FARMING6_CREDENCIAL}}"]);
    }

    let mut proceso = orden.spawn().map_err(|e| format!("No se pudo iniciar curl.exe: {e}"))?;
    if let Some(mut entrada) = proceso.stdin.take() {
        entrada
            .write_all(mensaje.as_bytes())
            .map_err(|e| format!("No se pudo entregar el mensaje a curl.exe: {e}"))?;
    }
    let salida = proceso.wait_with_output().map_err(|e| format!("curl.exe falló: {e}"))?;
    if salida.status.success() {
        return Ok(());
    }
    Err(match salida.status.code() {
        Some(67) => "Gmail rechazó la cuenta o la clave de aplicación.".to_string(),
        Some(6 | 7 | 28) => "No hubo conexión con el servidor de correo. Revise internet.".to_string(),
        Some(55 | 56) => "El servidor de correo cortó la conexión. Intente de nuevo.".to_string(),
        _ => format!("El correo no salió: {}", String::from_utf8_lossy(&salida.stderr).trim()),
    })
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn base64_coincide_con_el_estandar() {
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64("campaña".as_bytes()), "Y2FtcGHDsWE=");
    }

    #[test]
    fn la_cuenta_queda_cifrada_en_disco() {
        let carpeta = std::env::temp_dir().join(format!("farming6-dpapi-{}", std::process::id()));
        fs::create_dir_all(&carpeta).unwrap();
        let cuenta = Cuenta { correo: "remite@ejemplo.com".into(), clave: "abcdefghijklmnop".into() };
        guardar(&carpeta, &cuenta).unwrap();
        let bytes = fs::read(archivo(&carpeta)).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("abcdefghijklmnop"));
        assert_eq!(cargar(&carpeta), Some(cuenta));
        fs::remove_dir_all(&carpeta).unwrap();
    }

    #[test]
    fn limpia_los_espacios_de_la_clave() {
        assert_eq!(limpiar_clave("abcd efgh ijkl mnop"), "abcdefghijklmnop");
    }

    #[test]
    fn el_correo_escapa_html() {
        assert!(cuerpo_recuperacion("<b>", "u", "t").contains("&lt;b&gt;"));
    }
}
