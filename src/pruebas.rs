//! Pruebas de punta a punta: usan las mismas funciones de los botones contra una base SQLite
//! real, y un servidor de correo falso para comprobar el envío sin salir a internet.

use crate::auth::{self, correo};
use crate::db::vigia::{self, Accion, Foto};
use crate::db::Database;
use crate::models::*;
use crate::ui::{AppState, Modo, Pantalla, TipoMensaje};
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

static CONTADOR: AtomicU32 = AtomicU32::new(0);

struct Limpieza(PathBuf);

impl Drop for Limpieza {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// El estado va primero: al soltarlo se cierra la base antes de borrar la carpeta.
struct Entorno {
    e: AppState,
    _limpieza: Limpieza,
}

fn entorno() -> Entorno {
    let numero = CONTADOR.fetch_add(1, Ordering::SeqCst);
    let carpeta = std::env::temp_dir().join(format!("farming6-prueba-{}-{numero}", std::process::id()));
    std::fs::create_dir_all(&carpeta).unwrap();
    let base = Database::abrir(&carpeta.join("prueba.db")).unwrap();
    let foto = Arc::new(Mutex::new(Foto::default()));
    Entorno { e: AppState::nuevo(base, carpeta.clone(), foto), _limpieza: Limpieza(carpeta) }
}

fn datos(id: &str, nombre: &str, correo: &str) -> DatosPersona {
    DatosPersona {
        identificacion: id.into(),
        nom1: nombre.into(),
        nom2: String::new(),
        apell1: "Prueba".into(),
        apell2: String::new(),
        direccion: "Bogotá".into(),
        tele: String::new(),
        movil: "3001234567".into(),
        correo: correo.into(),
        fecha_nac: "1990-05-20".into(),
    }
}

const CLAVE_ADMIN: &str = "Campo2026";
const ID_ADMIN: i64 = 900_100_200;

fn crear_admin(e: &mut AppState) {
    e.inicial.datos = datos("900100200", "Ana", "admin@ejemplo.com");
    e.inicial.nombreu = "admin.farming".into();
    e.inicial.clave = CLAVE_ADMIN.into();
    e.crear_administrador_inicial();
    assert_eq!(e.pantalla, Pantalla::Acceso, "{}", texto(e));
}

fn ingresar(e: &mut AppState, usuario: &str, clave: &str) {
    e.acceso.usuario = usuario.into();
    e.acceso.clave = clave.into();
    e.ingresar();
}

fn con_admin() -> Entorno {
    let mut t = entorno();
    crear_admin(&mut t.e);
    ingresar(&mut t.e, "admin.farming", CLAVE_ADMIN);
    assert_eq!(t.e.pantalla, Pantalla::Panel);
    t
}

fn texto(e: &AppState) -> String {
    e.mensaje.as_ref().map(|m| m.texto.clone()).unwrap_or_default()
}

fn tipo(e: &AppState) -> Option<TipoMensaje> {
    e.mensaje.as_ref().map(|m| m.tipo)
}

fn guardar_persona(e: &mut AppState, d: DatosPersona) {
    e.ir_a(Pantalla::Persona);
    e.persona_crear();
    e.persona.datos = d;
    e.persona_guardar();
}

fn crear_usuario_para(e: &mut AppState, id: i64, nombre: &str, clave: &str, idperfil: i64) {
    e.ir_a(Pantalla::Usuario);
    e.usuario_crear();
    e.usuario.idpersona = Some(id);
    e.usuario.nombreu = nombre.into();
    e.usuario.contrasena = clave.into();
    e.usuario.idperfil = Some(idperfil);
    e.usuario_guardar();
}

#[test]
fn base_vacia_pide_administrador_y_no_hay_clave_quemada() {
    let mut t = entorno();
    assert_eq!(t.e.pantalla, Pantalla::Acceso, "siempre se abre en el ingreso, nunca en un registro forzado");
    assert!(t.e.es_primer_registro());
    t.e.crear_administrador_inicial();
    assert_eq!(tipo(&t.e), Some(TipoMensaje::Error));
    assert_eq!(t.e.db.total_usuarios(), Ok(0));

    crear_admin(&mut t.e);
    assert!(!t.e.es_primer_registro());
    let u = t.e.db.usuario_por_nombre("admin.farming").unwrap().unwrap();
    assert_eq!(u.idusuario, ID_ADMIN);
    assert!(u.contrasena.starts_with("$argon2id$"));
    assert_ne!(u.contrasena, CLAVE_ADMIN);
}

#[test]
fn el_registro_por_pasos_valida_cada_paso_y_crea_el_primer_administrador() {
    let mut t = entorno();
    let e = &mut t.e;
    e.abrir_registro();
    assert_eq!(e.pantalla, Pantalla::Registro);

    e.registro_siguiente();
    assert_eq!(e.inicial.paso, 0, "sin datos no avanza");
    assert_eq!(tipo(e), Some(TipoMensaje::Error));

    e.persona.datos = datos("900100200", "Ana", "admin@ejemplo.com");
    e.registro_siguiente();
    assert_eq!(e.inicial.paso, 1, "{}", texto(e));
    e.persona.datos.correo = "malo".into();
    e.registro_siguiente();
    assert_eq!(e.inicial.paso, 1);
    assert!(texto(e).contains("formato válido"));
    e.persona.datos.correo = "admin@ejemplo.com".into();
    e.registro_siguiente();
    assert_eq!(e.inicial.paso, 2, "{}", texto(e));

    e.registro_atras();
    assert_eq!(e.inicial.paso, 1);
    assert_eq!(e.persona.datos.nom1, "Ana", "volver no borra lo escrito");
    e.registro_siguiente();

    e.inicial.nombreu = "admin.farming".into();
    e.inicial.clave = CLAVE_ADMIN.into();
    e.registro_siguiente();
    assert_eq!(e.pantalla, Pantalla::Acceso, "{}", texto(e));
    assert!(e.persona.datos.identificacion.is_empty());
    ingresar(e, "admin.farming", CLAVE_ADMIN);
    assert_eq!(e.pantalla, Pantalla::Panel);
    assert!(e.es_administrador());
}

#[test]
fn con_cuentas_el_registro_tiene_dos_pasos_y_no_crea_administradores() {
    let mut t = entorno();
    crear_admin(&mut t.e);
    let e = &mut t.e;
    e.abrir_registro();
    e.persona.datos = datos("900100200", "Otra", "otra@ejemplo.com");
    e.registro_siguiente();
    assert_eq!(e.inicial.paso, 0, "la identificación repetida se avisa en el primer paso");
    assert!(texto(e).contains("ya está registrada"));

    e.persona.datos = datos("70111222", "Pedro", "pedro@ejemplo.com");
    e.registro_siguiente();
    e.registro_siguiente();
    assert_eq!(e.pantalla, Pantalla::Acceso, "{}", texto(e));
    assert!(texto(e).contains("administrador debe crear"), "{}", texto(e));
    assert_eq!(e.db.total_usuarios(), Ok(1));
}

#[test]
fn la_hoja_abre_al_elegir_y_salir_descarta_lo_escrito() {
    let mut t = con_admin();
    let e = &mut t.e;
    e.ir_a(Pantalla::Perfil);
    assert!(!e.perfil.hoja);
    let perfil = e.perfil.lista[1].clone();
    e.perfil_mostrar(&perfil);
    assert!(e.perfil.hoja);

    e.perfil_modificar();
    e.perfil.descripc = "Otra cosa".into();
    e.perfil_salir_de_hoja();
    assert_eq!(e.perfil.modo, Modo::Consulta);
    assert!(e.perfil.hoja, "salir de la edición vuelve a la consulta");
    assert_eq!(e.perfil.descripc, perfil.descripc);
    e.perfil_salir_de_hoja();
    assert!(!e.perfil.hoja);

    e.perfil_crear();
    assert!(e.perfil.hoja);
    e.perfil.descripc = "Veterinario".into();
    e.perfil_guardar();
    assert!(!e.perfil.hoja, "al guardar se cierra para ver el cambio en la lista y en el mapa");
}

#[test]
fn acceso_valida_cada_caso_con_su_mensaje() {
    let mut t = entorno();
    crear_admin(&mut t.e);
    let e = &mut t.e;

    ingresar(e, "", CLAVE_ADMIN);
    assert_eq!(texto(e), "Debe escribir el usuario.");
    ingresar(e, "admin.farming", "");
    assert_eq!(texto(e), "Debe escribir la contraseña.");
    ingresar(e, "nadie", CLAVE_ADMIN);
    assert_eq!(texto(e), "El usuario no se encuentra registrado");
    ingresar(e, "admin.farming", "Otra2026");
    assert_eq!(texto(e), "La contraseña no coincide con la registrada.");
    assert_eq!(e.pantalla, Pantalla::Acceso);

    e.acceso.usuario = "algo".into();
    e.acceso.clave = "algo".into();
    e.cancelar_acceso();
    assert!(e.acceso.usuario.is_empty() && e.acceso.clave.is_empty());

    ingresar(e, "ADMIN.FARMING", CLAVE_ADMIN);
    assert_eq!(e.pantalla, Pantalla::Panel);
    assert!(e.es_administrador());
    assert!(e.acceso.clave.is_empty());
}

#[test]
fn crud_de_perfil_completo() {
    let mut t = con_admin();
    let e = &mut t.e;
    e.ir_a(Pantalla::Perfil);
    assert_eq!(e.perfil.lista.len(), 2);

    e.perfil_guardar();
    assert_eq!(tipo(e), Some(TipoMensaje::Error));

    e.perfil_crear();
    e.perfil_guardar();
    assert!(texto(e).contains("obligatoria"));
    e.perfil.descripc = "Técnico agrícola".into();
    e.perfil_guardar();
    assert!(texto(e).contains("guardado"), "{}", texto(e));
    let id = e.perfil.id.unwrap();
    assert_eq!(e.perfil.lista.len(), 3);

    e.perfil_crear();
    e.perfil.descripc = "técnico AGRÍCOLA".into();
    e.perfil_guardar();
    assert!(texto(e).contains("Ya existe"), "las tildes también cuentan: {}", texto(e));
    assert_eq!(e.db.listar_perfiles().unwrap().len(), 3);

    e.perfil.buscar = id.to_string();
    e.perfil_consultar();
    assert_eq!(e.perfil.descripc, "Técnico agrícola");
    e.perfil_modificar();
    assert_eq!(e.perfil.modo, Modo::Edicion);
    e.perfil.descripc = "Técnico de campo".into();
    e.perfil_guardar();
    assert_eq!(e.db.perfil_por_id(id).unwrap().unwrap().descripc, "Técnico de campo");

    e.perfil_pedir_inhabilitar();
    let accion = e.confirmar.take().expect("pide confirmar").accion;
    e.ejecutar(accion);
    assert_eq!(e.db.perfil_por_id(id).unwrap().unwrap().estado, INACTIVO);

    e.perfil.buscar = "1".into();
    e.perfil_consultar();
    e.perfil_pedir_inhabilitar();
    assert!(texto(e).contains("Administrador"));
    assert!(e.confirmar.is_none());

    e.perfil_modificar();
    e.perfil.descripc = "Gerente".into();
    e.perfil_guardar();
    assert!(texto(e).contains("no se puede renombrar"), "{}", texto(e));
    assert_eq!(e.db.perfil_por_id(1).unwrap().unwrap().descripc, "Administrador");
    e.perfil_recargar();
    e.perfil_abrir();

    e.perfil.buscar = "99".into();
    e.perfil_consultar();
    assert!(texto(e).contains("No existe"));
}

#[test]
fn perfil_con_usuarios_activos_no_se_inhabilita() {
    let mut t = con_admin();
    let e = &mut t.e;
    guardar_persona(e, datos("70111222", "Pedro", "pedro@ejemplo.com"));
    crear_usuario_para(e, 70_111_222, "pedro", "Campo2026", 2);
    e.ir_a(Pantalla::Perfil);
    e.perfil.buscar = "2".into();
    e.perfil_consultar();
    e.perfil_pedir_inhabilitar();
    assert!(texto(e).contains("usuario(s) activo(s)"), "{}", texto(e));
}

#[test]
fn crud_de_persona_y_borrado_logico() {
    let mut t = con_admin();
    let e = &mut t.e;
    guardar_persona(e, datos("70111222", "Pedro", "Pedro@Ejemplo.com"));
    assert!(texto(e).contains("guardada"), "{}", texto(e));
    let p = e.db.persona_por_id(70_111_222).unwrap().unwrap();
    assert_eq!(p.correo, "pedro@ejemplo.com");

    guardar_persona(e, datos("70111222", "Otro", "otro@ejemplo.com"));
    assert!(texto(e).contains("identificación ya está"), "{}", texto(e));
    guardar_persona(e, datos("70111333", "Otro", "pedro@ejemplo.com"));
    assert!(texto(e).contains("correo ya está"), "{}", texto(e));
    guardar_persona(e, datos("70111333", "Ot7ro", "otro@ejemplo.com"));
    assert!(texto(e).contains("solo admite letras"), "{}", texto(e));

    e.ir_a(Pantalla::Persona);
    e.persona.buscar = "70111222".into();
    e.persona_consultar();
    assert_eq!(e.persona.id, Some(70_111_222));
    e.persona_modificar();
    e.persona.datos.direccion = "Corregimiento Banco de Arena".into();
    e.persona_guardar();
    assert_eq!(e.db.persona_por_id(70_111_222).unwrap().unwrap().direccion, "Corregimiento Banco de Arena");

    e.persona_pedir_inhabilitar();
    let accion = e.confirmar.take().expect("pide confirmar").accion;
    e.ejecutar(accion);
    let inhabilitada = e.db.persona_por_id(70_111_222).unwrap().expect("la fila sigue en la base");
    assert_eq!(inhabilitada.estado, INACTIVO);

    e.persona_modificar();
    e.persona.activo = true;
    e.persona_guardar();
    assert_eq!(e.db.persona_por_id(70_111_222).unwrap().unwrap().estado, ACTIVO);
    assert_eq!(e.db.listar_personas().unwrap().len(), 2);

    e.persona.buscar = String::new();
    e.persona_recargar();
    e.persona.id = Some(ID_ADMIN);
    e.persona_pedir_inhabilitar();
    assert!(texto(e).contains("inició sesión"));
}

#[test]
fn crud_de_usuario_y_relacion_uno_a_uno() {
    let mut t = con_admin();
    let e = &mut t.e;
    guardar_persona(e, datos("70111222", "Pedro", "pedro@ejemplo.com"));

    e.ir_a(Pantalla::Usuario);
    e.usuario_crear();
    assert_eq!(e.usuario.personas_libres.len(), 1);
    e.usuario_guardar();
    assert!(texto(e).contains("obligatorio"), "{}", texto(e));
    e.usuario.nombreu = "pedro".into();
    e.usuario_guardar();
    assert!(texto(e).contains("perfil"), "{}", texto(e));
    e.usuario.idperfil = Some(2);
    e.usuario_guardar();
    assert!(texto(e).contains("persona"), "{}", texto(e));
    e.usuario.idpersona = Some(70_111_222);
    e.usuario.contrasena = "corta".into();
    e.usuario_guardar();
    assert!(texto(e).contains("entre 8 y 64"), "{}", texto(e));
    e.usuario.contrasena = "Campo2026".into();
    e.usuario_guardar();
    assert!(texto(e).contains("guardado"), "{}", texto(e));

    let u = e.db.usuario_por_id(70_111_222).unwrap().expect("creado");
    assert!(auth::verificar("Campo2026", &u.contrasena));
    assert!(e.usuario.personas_libres.is_empty());

    e.usuario_crear();
    e.usuario.idpersona = Some(70_111_222);
    e.usuario.nombreu = "pedro2".into();
    e.usuario.contrasena = "Campo2026".into();
    e.usuario.idperfil = Some(2);
    e.usuario_guardar();
    assert!(texto(e).contains("ya tiene un usuario"), "{}", texto(e));

    e.usuario.buscar = "pedro".into();
    e.usuario_consultar();
    e.usuario_modificar();
    e.usuario.nombreu = "pedro.prueba".into();
    e.usuario_guardar();
    let u = e.db.usuario_por_id(70_111_222).unwrap().unwrap();
    assert_eq!(u.nombreu, "pedro.prueba");
    assert!(auth::verificar("Campo2026", &u.contrasena), "vacía conserva la contraseña");

    e.usuario_modificar();
    e.usuario.contrasena = "Potrero2026".into();
    e.usuario_guardar();
    let u = e.db.usuario_por_id(70_111_222).unwrap().unwrap();
    assert!(auth::verificar("Potrero2026", &u.contrasena));

    e.usuario_pedir_inhabilitar();
    let accion = e.confirmar.take().expect("pide confirmar").accion;
    e.ejecutar(accion);
    assert_eq!(e.db.usuario_por_id(70_111_222).unwrap().unwrap().estado, INACTIVO);

    e.cerrar_sesion();
    ingresar(e, "pedro.prueba", "Potrero2026");
    assert!(texto(e).contains("inhabilitado"), "{}", texto(e));
}

#[test]
fn la_base_impone_el_modelo_aunque_falle_la_interfaz() {
    let t = con_admin();
    let db = &t.e.db;
    let huerfano = Usuario {
        idusuario: 999_999,
        nombreu: "huerfano".into(),
        contrasena: "x".into(),
        idperfil: 1,
        estado: ACTIVO.into(),
    };
    assert!(db.crear_usuario(&huerfano).unwrap_err().contains("no existe"));
    let sin_perfil = Usuario { idusuario: ID_ADMIN, nombreu: "otro".into(), idperfil: 77, ..huerfano.clone() };
    assert!(db.crear_usuario(&sin_perfil).is_err());
    let segundo = Usuario { idusuario: ID_ADMIN, nombreu: "otro".into(), ..huerfano };
    assert!(db.crear_usuario(&segundo).unwrap_err().contains("ya tiene un usuario"));
}

#[test]
fn siempre_queda_un_administrador() {
    let mut t = con_admin();
    let e = &mut t.e;
    e.ir_a(Pantalla::Usuario);
    e.usuario.buscar = "admin.farming".into();
    e.usuario_consultar();
    e.usuario_pedir_inhabilitar();
    assert!(texto(e).contains("inició sesión"), "{}", texto(e));

    e.usuario_modificar();
    e.usuario.activo = false;
    e.usuario_guardar();
    assert_eq!(tipo(e), Some(TipoMensaje::Error));
    assert_eq!(e.db.administradores_activos(), Ok(1));

    e.usuario.activo = true;
    e.usuario.nombreu = "admin.campo".into();
    e.usuario_guardar();
    assert_eq!(e.sesion.as_ref().unwrap().usuario.nombreu, "admin.campo", "el menú muestra el dato nuevo");
}

#[test]
fn permisos_del_perfil_persona() {
    let mut t = con_admin();
    let e = &mut t.e;
    guardar_persona(e, datos("70111222", "Pedro", "pedro@ejemplo.com"));
    guardar_persona(e, datos("70111333", "Ana", "ana@ejemplo.com"));
    crear_usuario_para(e, 70_111_222, "pedro", "Campo2026", 2);
    e.cerrar_sesion();

    ingresar(e, "pedro", "Campo2026");
    assert_eq!(e.pantalla, Pantalla::Panel);
    assert!(!e.es_administrador());
    for pantalla in [Pantalla::Perfil, Pantalla::Usuario, Pantalla::Mapa, Pantalla::Correo] {
        e.ir_a(pantalla);
        assert_eq!(e.pantalla, Pantalla::Panel);
        assert!(texto(e).contains("permiso"));
    }

    e.ir_a(Pantalla::Persona);
    assert_eq!(e.persona.lista.len(), 1, "solo ve sus datos");
    assert_eq!(e.persona.id, Some(70_111_222));
    e.persona_modificar();
    e.persona.datos.direccion = "Medellín".into();
    e.persona_guardar();
    assert_eq!(e.db.persona_por_id(70_111_222).unwrap().unwrap().direccion, "Medellín");

    e.persona.buscar = "70111333".into();
    e.persona_consultar();
    assert_eq!(tipo(e), Some(TipoMensaje::Error));

    e.persona_pedir_inhabilitar();
    assert!(texto(e).contains("no puede inhabilitar"), "{}", texto(e));
    assert!(e.confirmar.is_none());
    e.persona_inhabilitar(70_111_222);
    assert_eq!(e.db.persona_por_id(70_111_222).unwrap().unwrap().estado, ACTIVO);
}

#[test]
fn registrarse_guarda_persona_y_el_administrador_crea_el_usuario() {
    let mut t = entorno();
    crear_admin(&mut t.e);
    let e = &mut t.e;

    e.persona_preparar_registro();
    assert_eq!(e.pantalla, Pantalla::Registro);
    e.persona.datos = datos("70111222", "Pedro", "correo-malo");
    e.persona_guardar_registro();
    assert_eq!(e.pantalla, Pantalla::Registro);
    assert!(texto(e).contains("formato válido"));
    e.persona.datos.correo = "pedro@ejemplo.com".into();
    e.persona_guardar_registro();
    assert_eq!(e.pantalla, Pantalla::Acceso);
    assert!(texto(e).contains("administrador debe crear"), "{}", texto(e));

    ingresar(e, "pedro", "Campo2026");
    assert_eq!(texto(e), "El usuario no se encuentra registrado");
    ingresar(e, "admin.farming", CLAVE_ADMIN);
    crear_usuario_para(e, 70_111_222, "pedro", "Campo2026", 2);
    assert!(texto(e).contains("guardado"), "{}", texto(e));
    e.cerrar_sesion();
    ingresar(e, "pedro", "Campo2026");
    assert_eq!(e.pantalla, Pantalla::Panel);
}

#[derive(Default, Debug)]
struct Capturado {
    credencial: String,
    remitente: String,
    destino: String,
    datos: String,
}

fn linea(lector: &mut BufReader<TcpStream>) -> Option<String> {
    let mut l = String::new();
    match lector.read_line(&mut l) {
        Ok(n) if n > 0 => Some(l),
        _ => None,
    }
}

/// Servidor SMTP mínimo de una sola conexión. Si `rechaza`, niega el usuario y la clave.
fn servidor_falso(rechaza: bool) -> (u16, JoinHandle<Capturado>) {
    let oyente = TcpListener::bind("127.0.0.1:0").unwrap();
    let puerto = oyente.local_addr().unwrap().port();
    let hilo = thread::spawn(move || {
        let (mut flujo, _) = oyente.accept().unwrap();
        let mut lector = BufReader::new(flujo.try_clone().unwrap());
        let mut c = Capturado::default();
        flujo.write_all(b"220 falso\r\n").unwrap();
        while let Some(l) = linea(&mut lector) {
            let orden = l.trim_end().to_string();
            let mayuscula = orden.to_uppercase();
            if mayuscula.starts_with("EHLO") {
                flujo.write_all(b"250-falso\r\n250 AUTH PLAIN\r\n").unwrap();
            } else if mayuscula.starts_with("AUTH PLAIN") {
                let mut valor = orden.split_whitespace().nth(2).unwrap_or("").to_string();
                if valor.is_empty() {
                    flujo.write_all(b"334 \r\n").unwrap();
                    valor = linea(&mut lector).unwrap_or_default().trim().to_string();
                }
                c.credencial = valor;
                if rechaza {
                    flujo.write_all(b"535 rechazado\r\n").unwrap();
                } else {
                    flujo.write_all(b"235 aceptado\r\n").unwrap();
                }
            } else if mayuscula.starts_with("MAIL FROM") {
                c.remitente = orden;
                flujo.write_all(b"250 bien\r\n").unwrap();
            } else if mayuscula.starts_with("RCPT TO") {
                c.destino = orden;
                flujo.write_all(b"250 bien\r\n").unwrap();
            } else if mayuscula.starts_with("DATA") {
                flujo.write_all(b"354 envie\r\n").unwrap();
                while let Some(d) = linea(&mut lector) {
                    if d == ".\r\n" {
                        break;
                    }
                    c.datos.push_str(&d);
                }
                flujo.write_all(b"250 guardado\r\n").unwrap();
            } else if mayuscula.starts_with("QUIT") {
                flujo.write_all(b"221 adios\r\n").unwrap();
                break;
            } else {
                flujo.write_all(b"250 bien\r\n").unwrap();
            }
        }
        c
    });
    (puerto, hilo)
}

fn esperar_envio(e: &mut AppState) {
    let limite = Instant::now() + Duration::from_secs(30);
    while e.acceso.envio.is_some() {
        e.revisar_envio();
        assert!(Instant::now() < limite, "el envío tardó demasiado");
        thread::sleep(Duration::from_millis(20));
    }
}

fn decodificar(texto: &str) -> Vec<u8> {
    const A: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut salida = Vec::new();
    let mut acumulado = 0u32;
    let mut bits = 0;
    for b in texto.bytes() {
        let Some(v) = A.iter().position(|x| *x == b) else { continue };
        acumulado = (acumulado << 6) | v as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            salida.push((acumulado >> bits) as u8);
            acumulado &= (1 << bits) - 1;
        }
    }
    salida
}

fn contrasena_del_correo(datos: &str) -> String {
    let (_, cuerpo) = datos.split_once("\r\n\r\n").expect("mensaje con cuerpo");
    let html = String::from_utf8(decodificar(cuerpo)).unwrap();
    let (_, resto) = html.split_once("Contraseña temporal:</b> ").expect("trae la contraseña");
    resto.split('<').next().unwrap().to_string()
}

#[test]
fn recuperar_envia_correo_real_y_solo_entonces_cambia_la_clave() {
    let mut t = entorno();
    crear_admin(&mut t.e);
    let e = &mut t.e;

    e.recuperar();
    assert!(texto(e).contains("Digite"));
    e.acceso.dato = "000000000".into();
    e.recuperar();
    assert_eq!(texto(e), "El usuario no se encuentra registrado");
    e.acceso.dato = "correo-malo".into();
    e.recuperar();
    assert!(texto(e).contains("formato válido"));
    e.acceso.dato = "admin@ejemplo.com".into();
    e.recuperar();
    assert!(texto(e).contains("no está configurado"), "{}", texto(e));

    let cuenta = correo::Cuenta { correo: "remite@ejemplo.com".into(), clave: "abcdefghijklmnop".into() };
    correo::guardar(&e.carpeta, &cuenta).unwrap();

    let (puerto, hilo) = servidor_falso(true);
    e.servidor_correo = format!("smtp://127.0.0.1:{puerto}");
    e.recuperar();
    esperar_envio(e);
    hilo.join().unwrap();
    assert_eq!(tipo(e), Some(TipoMensaje::Error), "{}", texto(e));
    assert!(texto(e).contains("rechazó"), "{}", texto(e));
    let intacta = e.db.usuario_por_id(ID_ADMIN).unwrap().unwrap();
    assert!(auth::verificar(CLAVE_ADMIN, &intacta.contrasena), "si falla el envío no cambia la clave");

    let (puerto, hilo) = servidor_falso(false);
    e.servidor_correo = format!("smtp://127.0.0.1:{puerto}");
    e.acceso.dato = "900100200".into();
    e.recuperar();
    assert!(e.acceso.envio.is_some());
    esperar_envio(e);
    let c = hilo.join().unwrap();
    assert_eq!(texto(e), "Su usuario y contraseña fue enviado al correo registrado");

    assert!(c.remitente.contains("remite@ejemplo.com"), "{}", c.remitente);
    assert!(c.destino.contains("admin@ejemplo.com"), "{}", c.destino);
    let credencial = String::from_utf8(decodificar(&c.credencial)).unwrap();
    assert!(credencial.contains("remite@ejemplo.com") && credencial.contains("abcdefghijklmnop"));
    assert!(c.datos.contains("Subject: =?UTF-8?B?"));

    let (_, cuerpo) = c.datos.split_once("\r\n\r\n").unwrap();
    let html = String::from_utf8(decodificar(cuerpo)).unwrap();
    assert!(html.contains("admin.farming"), "el correo trae el usuario");
    let temporal = contrasena_del_correo(&c.datos);
    assert_eq!(temporal.len(), 12);

    ingresar(e, "admin.farming", CLAVE_ADMIN);
    assert!(texto(e).contains("no coincide"));
    ingresar(e, "admin.farming", &temporal);
    assert_eq!(e.pantalla, Pantalla::Panel, "{}", texto(e));
}

#[test]
fn el_vigia_lee_el_esquema_real_y_detecta_cada_cambio() {
    let mut t = con_admin();
    let ruta = t.e.db.ruta.clone();
    let mut mirar = vigia::pasada_de_prueba(&ruta);
    let foto = mirar();

    let nombres: Vec<&str> = foto.tablas.iter().map(|x| x.nombre.as_str()).collect();
    assert_eq!(nombres, ["perfil", "usuario", "persona"]);
    let columnas = |tabla: &str| -> Vec<String> {
        foto.tabla(tabla).unwrap().columnas.iter().map(|c| c.nombre.clone()).collect()
    };
    assert_eq!(columnas("perfil"), ["idperfil", "descripc", "estado"]);
    assert_eq!(columnas("usuario"), ["idusuario", "nombreu", "contrasena", "idperfil", "estado"]);
    assert_eq!(
        columnas("persona"),
        ["idpersona", "nom1", "nom2", "apell1", "apell2", "direccion", "tele", "movil", "correo", "fecha_nac", "estado"]
    );

    let hacia_perfil = foto.relaciones.iter().find(|r| r.tabla_destino == "perfil").unwrap();
    assert_eq!((hacia_perfil.tabla.as_str(), hacia_perfil.columna.as_str()), ("usuario", "idperfil"));
    assert!(!hacia_perfil.uno_a_uno, "un perfil, muchos usuarios");
    let hacia_persona = foto.relaciones.iter().find(|r| r.tabla_destino == "persona").unwrap();
    assert_eq!((hacia_persona.tabla.as_str(), hacia_persona.columna.as_str()), ("usuario", "idusuario"));
    assert!(hacia_persona.uno_a_uno, "una persona, un usuario");

    let usuario = foto.tabla("usuario").unwrap();
    assert!(usuario.filas[0][2].starts_with("argon2id…"), "el mapa no muestra el hash completo");
    assert!(foto.cambios.is_empty(), "la primera lectura no es un cambio");

    let e = &mut t.e;
    guardar_persona(e, datos("70111222", "Pedro", "pedro@ejemplo.com"));
    let foto = mirar();
    let c = foto.cambios.front().expect("detecta el INSERT");
    assert_eq!((c.accion, c.tabla.as_str(), c.clave.as_str()), (Accion::Insertar, "persona", "70111222"));
    assert_eq!(foto.tabla("persona").unwrap().activos, 2);
    assert!(foto.resaltado_tabla("persona") > 0.0);

    e.persona_modificar();
    e.persona.datos.direccion = "Medellín".into();
    e.persona_guardar();
    let foto = mirar();
    let c = foto.cambios.front().expect("detecta el UPDATE");
    assert_eq!(c.accion, Accion::Actualizar);
    assert!(c.detalle.contains("Bogotá → Medellín"), "{}", c.detalle);
    assert_eq!(c.columnas, ["direccion"]);

    e.persona_inhabilitar(70_111_222);
    let foto = mirar();
    let c = foto.cambios.front().expect("detecta el borrado lógico");
    assert_eq!(c.accion, Accion::Inhabilitar);
    let tabla = foto.tabla("persona").unwrap();
    assert_eq!((tabla.activos, tabla.inhabilitados, tabla.filas.len()), (1, 1, 2));

    let afuera = rusqlite::Connection::open(&ruta).unwrap();
    afuera.execute("UPDATE perfil SET descripc = 'Productor' WHERE idperfil = 2", []).unwrap();
    let foto = mirar();
    let c = foto.cambios.front().expect("detecta cambios de otro programa");
    assert_eq!((c.tabla.as_str(), c.clave.as_str()), ("perfil", "2"));

    let antes = foto.cambios.len();
    assert_eq!(mirar().cambios.len(), antes, "sin escrituras no hay cambios nuevos");
}

/// Pruebas de la interfaz sin ventana: egui recibe eventos reales de ratón y calcula
/// cada cuadro igual que en la aplicación, solo que sin dibujar en pantalla.
mod interfaz {
    use super::*;
    use crate::ui::{login, theme};
    use eframe::egui::{self, pos2, vec2, Context, Event, Modifiers, PointerButton, Pos2, RawInput, Rect, Vec2};
    use std::thread::sleep;
    use std::time::Duration;

    const NORMAL: Vec2 = vec2(1366.0, 800.0);
    const MINIMA: Vec2 = vec2(1120.0, 680.0);

    struct Lienzo {
        contexto: Context,
        tiempo: f64,
        tamano: Vec2,
    }

    impl Lienzo {
        fn nuevo(tamano: Vec2) -> Self {
            let contexto = Context::default();
            theme::aplicar(&contexto);
            Self { contexto, tiempo: 0.0, tamano }
        }

        fn cuadro(&mut self, e: &mut AppState, eventos: Vec<Event>) {
            self.tiempo += 1.0 / 60.0;
            let entrada = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, self.tamano)),
                time: Some(self.tiempo),
                predicted_dt: 1.0 / 60.0,
                focused: true,
                events: eventos,
                ..Default::default()
            };
            let _ = self.contexto.run(entrada, |c| {
                match e.pantalla {
                    Pantalla::Acceso => login::acceso(c, e),
                    Pantalla::Registro => login::registro(c, e),
                    _ => crate::interior(c, e),
                }
                login::telon(c, e);
            });
        }

        /// Deja terminar las entradas escalonadas, que corren con el reloj real.
        fn asentar(&mut self, e: &mut AppState) {
            for _ in 0..3 {
                self.cuadro(e, Vec::new());
            }
            sleep(Duration::from_millis(720));
            for _ in 0..3 {
                self.cuadro(e, Vec::new());
            }
        }

        fn clic(&mut self, e: &mut AppState, x: f32, y: f32) {
            let pos = pos2(x, y);
            let boton = |pressed| Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            };
            self.cuadro(e, vec![Event::PointerMoved(pos)]);
            self.cuadro(e, vec![boton(true)]);
            self.cuadro(e, vec![boton(false)]);
            self.cuadro(e, Vec::new());
        }

        /// Lado izquierdo de la hoja lateral en el último cuadro.
        fn borde_de_hoja(&self, nombre: &str) -> Option<f32> {
            let id = egui::Id::new(nombre).with("hoja");
            self.contexto.memory(|m| m.area_rect(id)).map(|r| r.left())
        }
    }

    #[test]
    fn el_enlace_registro_del_ingreso_responde_al_clic() {
        let mut t = entorno();
        crear_admin(&mut t.e);
        let mut l = Lienzo::nuevo(NORMAL);
        l.asentar(&mut t.e);
        assert_eq!(t.e.pantalla, Pantalla::Acceso);
        l.clic(&mut t.e, 148.0, 539.0);
        assert_eq!(t.e.pantalla, Pantalla::Registro);
    }

    #[test]
    fn la_hoja_entra_deslizandose_y_vuelve_a_salir() {
        let mut t = con_admin();
        t.e.ir_a(Pantalla::Usuario);
        t.e.telon = None;
        let mut l = Lienzo::nuevo(NORMAL);
        l.asentar(&mut t.e);
        let vista = t.e.usuario.lista[0].clone();
        t.e.usuario_mostrar(&vista);

        let mut bordes = Vec::new();
        for _ in 0..30 {
            l.cuadro(&mut t.e, Vec::new());
            bordes.extend(l.borde_de_hoja("hoja_usuario"));
        }
        let (primero, ultimo) = (bordes[0], *bordes.last().unwrap());
        assert!(primero > ultimo + 100.0, "entró de golpe: {bordes:?}");
        assert!(bordes.windows(2).all(|p| p[1] <= p[0]), "retrocede: {bordes:?}");
        assert!((ultimo - (NORMAL.x - 460.0)).abs() < 1.0, "{ultimo}");
        let distintos = bordes.windows(2).filter(|p| p[1] < p[0]).count();
        assert!(distintos >= 8, "muy pocos pasos intermedios: {distintos}");

        t.e.usuario_salir_de_hoja();
        for _ in 0..30 {
            l.cuadro(&mut t.e, Vec::new());
        }
        assert!(!t.e.usuario.hoja);
    }

    #[test]
    fn el_telon_se_levanta_solo_al_ingresar() {
        let mut t = entorno();
        crear_admin(&mut t.e);
        ingresar(&mut t.e, "admin.farming", CLAVE_ADMIN);
        assert!(t.e.telon.is_some());
        let mut l = Lienzo::nuevo(NORMAL);
        l.cuadro(&mut t.e, Vec::new());
        assert!(t.e.telon.is_some(), "debe seguir subiendo en el primer cuadro");
        sleep(Duration::from_millis(460));
        l.cuadro(&mut t.e, Vec::new());
        assert!(t.e.telon.is_none());
    }

    #[test]
    fn ninguna_pantalla_se_sale_de_la_ventana_minima() {
        let mut t = con_admin();
        let mut l = Lienzo::nuevo(MINIMA);
        t.e.telon = None;
        let pantalla = Rect::from_min_size(Pos2::ZERO, MINIMA);
        for destino in [Pantalla::Panel, Pantalla::Persona, Pantalla::Usuario, Pantalla::Perfil, Pantalla::Mapa, Pantalla::Correo] {
            t.e.ir_a(destino);
            l.asentar(&mut t.e);
            let usado = l.contexto.used_rect();
            assert!(pantalla.expand(1.0).contains_rect(usado), "{destino:?}: {usado:?}");
        }
        t.e.cerrar_sesion();
        for registro in [false, true] {
            if registro {
                t.e.abrir_registro();
            }
            l.asentar(&mut t.e);
            let usado = l.contexto.used_rect();
            assert!(pantalla.expand(1.0).contains_rect(usado), "registro {registro}: {usado:?}");
        }
    }
}
