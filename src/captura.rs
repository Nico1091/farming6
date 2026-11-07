//! Solo con la función `captura`: recorre las pantallas con datos de ejemplo en una base
//! temporal y guarda su propio cuadro. Sirve para revisar el diseño; no va en el ejecutable final.

use crate::auth::correo;
use crate::models::DatosPersona;
use crate::ui::{AppState, Pantalla};
use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const NOMBRES: [&str; 21] = [
    "00_apertura",
    "01_ingreso_base_vacia",
    "02_registro_paso1",
    "03_registro_cuenta",
    "04_ingreso_error",
    "05_recuperar",
    "06_panel",
    "07_personas",
    "08_persona_hoja",
    "09_persona_edicion_error",
    "10_usuario_nuevo",
    "11_perfiles",
    "12_mapa",
    "13_mapa_con_cambio",
    "14_confirmar",
    "15_correo",
    "16_panel_campesino",
    "17_mis_datos",
    "18_telon",
    "19_personas_antes",
    "20_hoja_entrando",
];

/// Espera antes de cada foto. Las cortas toman la animación a mitad de camino.
fn espera(paso: usize) -> u64 {
    match paso {
        0 => 550,
        1 => 2200,
        18 | 20 => 110,
        _ => 1500,
    }
}

enum Accion {
    Clic(f32, f32),
    Texto(&'static str),
    Tecla(egui::Key),
    Comprobar(&'static str, fn(&AppState) -> bool),
}

/// Prueba con eventos reales de ratón y teclado, en puntos de una ventana de 1366 por 800.
fn guion_de_eventos() -> Vec<Accion> {
    use egui::Key;
    use Accion::*;
    vec![
        Comprobar("abre en el ingreso", |e| e.pantalla == Pantalla::Acceso && e.sesion.is_none()),
        Clic(300.0, 250.0),
        Texto("admin.farming"),
        Comprobar("escribe en Usuario", |e| e.acceso.usuario == "admin.farming"),
        Tecla(Key::Tab),
        Texto("Potrero2026"),
        Comprobar("el tabulador pasa a Contraseña", |e| e.acceso.clave == "Potrero2026"),
        Tecla(Key::Enter),
        Comprobar("Enter ingresa", |e| e.pantalla == Pantalla::Panel),
        Clic(395.0, 43.0),
        Comprobar("la barra lleva a Usuarios", |e| e.pantalla == Pantalla::Usuario),
        Clic(240.0, 278.0),
        Comprobar("la fila abre la hoja", |e| e.usuario.hoja && e.usuario.id.is_some()),
        Clic(200.0, 600.0),
        Comprobar("pulsar fuera cierra la hoja", |e| !e.usuario.hoja),
        Clic(240.0, 278.0),
        Tecla(Key::Escape),
        Comprobar("Escape cierra la hoja", |e| !e.usuario.hoja),
        Clic(1246.0, 119.0),
        Comprobar("Crear abre la hoja vacía", |e| e.usuario.hoja && e.usuario.id.is_none()),
        Clic(1111.0, 470.0),
        Comprobar("Salir cierra la hoja", |e| !e.usuario.hoja),
        Clic(1298.0, 43.0),
        Comprobar("Salir de la barra cierra la sesión", |e| e.pantalla == Pantalla::Acceso && e.sesion.is_none()),
        Clic(148.0, 539.0),
        Comprobar("Registro abre el registro", |e| e.pantalla == Pantalla::Registro),
    ]
}

pub struct Guion {
    paso: usize,
    desde: Instant,
    pidio: bool,
    salida: PathBuf,
    eventos: std::collections::VecDeque<Accion>,
    fase_del_clic: u8,
    ultimo_evento: Instant,
    informe: String,
    verificar: bool,
}

fn persona(id: &str, nom1: &str, apell1: &str, correo: &str, movil: &str) -> DatosPersona {
    DatosPersona {
        identificacion: id.into(),
        nom1: nom1.into(),
        apell1: apell1.into(),
        direccion: "Corregimiento Banco de Arena".into(),
        movil: movil.into(),
        correo: correo.into(),
        fecha_nac: "1988-03-14".into(),
        ..Default::default()
    }
}

fn nueva_persona(e: &mut AppState, d: DatosPersona) {
    e.ir_a(Pantalla::Persona);
    e.persona_crear();
    e.persona.datos = d;
    e.persona_guardar();
}

fn nuevo_usuario(e: &mut AppState, id: i64, nombre: &str, perfil: i64) {
    e.ir_a(Pantalla::Usuario);
    e.usuario_crear();
    e.usuario.idpersona = Some(id);
    e.usuario.nombreu = nombre.into();
    e.usuario.contrasena = "Potrero2026".into();
    e.usuario.idperfil = Some(perfil);
    e.usuario_guardar();
}

fn preparar(paso: usize, e: &mut AppState) {
    match paso {
        2 => {
            e.abrir_registro();
            e.persona.datos = persona("1090000001", "Carolina", "Mendoza", "carolina@ejemplo.com", "3001112233");
        }
        3 => {
            e.registro_siguiente();
            e.registro_siguiente();
            e.inicial.nombreu = "admin.farming".into();
            e.inicial.clave = "Potrero2026".into();
        }
        4 => {
            e.registro_siguiente();
            e.acceso.usuario = "pedro".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
        }
        5 => {
            e.acceso = Default::default();
            e.mensaje = None;
            e.alternar_recuperar();
            e.acceso.dato = "pedro@ejemplo.com".into();
        }
        6 => {
            e.acceso = Default::default();
            e.acceso.usuario = "admin.farming".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
            nueva_persona(e, persona("1090000002", "Pedro", "Contreras", "pedro@ejemplo.com", "3102223344"));
            nueva_persona(e, persona("1090000003", "Luz Marina", "Duarte", "luz@ejemplo.com", "3203334455"));
            nuevo_usuario(e, 1_090_000_002, "pedro.contreras", 2);
            e.ir_a(Pantalla::Perfil);
            e.perfil_crear();
            e.perfil.descripc = "Técnico agrícola".into();
            e.perfil_guardar();
            e.mensaje = None;
            e.ir_a(Pantalla::Panel);
        }
        7 => e.ir_a(Pantalla::Persona),
        8 => {
            e.persona.buscar = "1090000003".into();
            e.persona_consultar();
            e.mensaje = None;
        }
        9 => {
            e.persona_modificar();
            e.persona.datos.correo = "correo-malo".into();
            e.persona_guardar();
        }
        10 => {
            e.mensaje = None;
            e.ir_a(Pantalla::Usuario);
            e.usuario_crear();
            e.usuario.idpersona = Some(1_090_000_003);
            e.usuario.nombreu = "luz.duarte".into();
            e.usuario.idperfil = Some(2);
        }
        11 => {
            e.ir_a(Pantalla::Perfil);
            e.mensaje = None;
        }
        12 => e.ir_a(Pantalla::Mapa),
        13 => {
            e.ir_a(Pantalla::Persona);
            e.persona.buscar = "1090000002".into();
            e.persona_consultar();
            e.persona_modificar();
            e.persona.datos.direccion = "Vereda La Floresta".into();
            e.persona_guardar();
            e.ir_a(Pantalla::Mapa);
        }
        14 => {
            e.ir_a(Pantalla::Persona);
            e.persona.buscar = "1090000003".into();
            e.persona_consultar();
            e.mensaje = None;
            e.persona_pedir_inhabilitar();
        }
        15 => {
            e.confirmar = None;
            e.ir_a(Pantalla::Correo);
            e.correo.correo = "farming6@ejemplo.com".into();
            e.correo.configurada = correo::cargar(&e.carpeta).is_some();
        }
        16 => {
            e.cerrar_sesion();
            e.acceso.usuario = "pedro.contreras".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
        }
        17 => e.ir_a(Pantalla::Persona),
        18 => {
            e.cerrar_sesion();
            e.acceso.usuario = "admin.farming".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
        }
        19 => e.ir_a(Pantalla::Persona),
        20 => {
            let primera = e.persona.lista[0].clone();
            e.persona_mostrar(&primera);
        }
        _ => {}
    }
}

impl Guion {
    pub fn nuevo() -> Self {
        let salida = std::env::var_os("FARMING6_FOTOS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        Self {
            paso: 0,
            desde: Instant::now(),
            pidio: false,
            salida,
            eventos: guion_de_eventos().into(),
            fase_del_clic: 0,
            ultimo_evento: Instant::now(),
            informe: String::new(),
            verificar: false,
        }
    }

    /// Entrega a egui los eventos de la prueba como si vinieran de Windows.
    pub fn inyectar(&mut self, entrada: &mut egui::RawInput) {
        entrada.focused = true;
        if !self.verificar || self.ultimo_evento.elapsed() < Duration::from_millis(320) {
            return;
        }
        let modificadores = egui::Modifiers::NONE;
        match self.eventos.front() {
            Some(Accion::Clic(x, y)) => {
                let pos = egui::pos2(*x, *y);
                let boton = |presionado| egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: presionado,
                    modifiers: modificadores,
                };
                match self.fase_del_clic {
                    0 => entrada.events.push(egui::Event::PointerMoved(pos)),
                    1 => entrada.events.push(boton(true)),
                    _ => {
                        entrada.events.push(boton(false));
                        self.eventos.pop_front();
                    }
                }
                self.fase_del_clic = (self.fase_del_clic + 1) % 3;
            }
            Some(Accion::Texto(texto)) => {
                entrada.events.push(egui::Event::Text((*texto).to_string()));
                self.eventos.pop_front();
            }
            Some(Accion::Tecla(tecla)) => {
                for presionada in [true, false] {
                    entrada.events.push(egui::Event::Key {
                        key: *tecla,
                        physical_key: None,
                        pressed: presionada,
                        repeat: false,
                        modifiers: modificadores,
                    });
                }
                self.eventos.pop_front();
            }
            _ => return,
        }
        self.ultimo_evento = Instant::now();
    }

    fn comprobar(&mut self, contexto: &egui::Context, estado: &AppState) {
        if self.ultimo_evento.elapsed() < Duration::from_millis(320) {
            return;
        }
        while let Some(Accion::Comprobar(nombre, prueba)) = self.eventos.front() {
            let bien = prueba(estado);
            self.informe.push_str(&format!("{} {nombre}\n", if bien { "BIEN" } else { "FALLA" }));
            self.eventos.pop_front();
        }
        if self.eventos.is_empty() {
            let _ = fs::write(self.salida.join("eventos.txt"), &self.informe);
            contexto.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub fn avanzar(&mut self, contexto: &egui::Context, estado: &mut AppState) {
        contexto.request_repaint_after(Duration::from_millis(30));
        if self.verificar {
            return self.comprobar(contexto, estado);
        }
        let imagen = contexto.input(|i| {
            i.events.iter().find_map(|evento| match evento {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(imagen) = imagen {
            let mut bytes = format!("{} {}\n", imagen.size[0], imagen.size[1]).into_bytes();
            bytes.extend_from_slice(imagen.as_raw());
            let _ = fs::write(self.salida.join(format!("{}.rgba", NOMBRES[self.paso])), bytes);
            self.paso += 1;
            self.pidio = false;
            self.desde = Instant::now();
            if self.paso >= NOMBRES.len() {
                estado.cerrar_sesion();
                self.verificar = true;
                self.ultimo_evento = Instant::now();
            } else {
                preparar(self.paso, estado);
            }
            return;
        }
        let lista = Duration::from_millis(espera(self.paso));
        if !self.pidio && self.paso < NOMBRES.len() && self.desde.elapsed() > lista {
            self.pidio = true;
            contexto.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
    }
}
