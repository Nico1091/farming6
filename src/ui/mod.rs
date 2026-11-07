//! Estado de la aplicación, navegación y permisos.

pub mod comun;
pub mod correo;
pub mod dashboard;
pub mod login;
pub mod mapa;
pub mod paisaje;
pub mod perfil;
pub mod persona;
pub mod theme;
pub mod usuario;

use crate::auth::correo as cuenta_correo;
use crate::db::vigia::Foto;
use crate::db::Database;
use crate::models::{Persona, Usuario};
use eframe::egui::{self, pos2, vec2, Align2, CursorIcon, FontId, Id, Margin, Sense, Stroke};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use theme::*;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pantalla {
    Acceso,
    Registro,
    Panel,
    Perfil,
    Persona,
    Usuario,
    Mapa,
    Correo,
}

/// Estado de un registro en la hoja lateral. Solo en Nuevo o Edición se escribe y se guarda.
#[derive(Clone, Copy, PartialEq, Debug, Default)]
pub enum Modo {
    #[default]
    Consulta,
    Nuevo,
    Edicion,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TipoMensaje {
    Exito,
    Error,
    Informacion,
}

pub struct Mensaje {
    pub texto: String,
    pub tipo: TipoMensaje,
    pub vence: Instant,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AccionPendiente {
    InhabilitarPerfil(i64),
    InhabilitarPersona(i64),
    InhabilitarUsuario(i64),
}

#[derive(Clone, Debug)]
pub struct Confirmacion {
    pub texto: String,
    pub accion: AccionPendiente,
}

pub struct Sesion {
    pub usuario: Usuario,
    pub persona: Persona,
    pub perfil: String,
    pub administrador: bool,
}

pub struct AppState {
    pub db: Database,
    pub carpeta: PathBuf,
    pub servidor_correo: String,
    pub pantalla: Pantalla,
    pub sesion: Option<Sesion>,
    pub mensaje: Option<Mensaje>,
    pub confirmar: Option<Confirmacion>,
    pub acceso: login::FormAcceso,
    pub inicial: login::FormInicial,
    pub perfil: perfil::FormPerfil,
    pub persona: persona::FormPersona,
    pub usuario: usuario::FormUsuario,
    pub correo: correo::FormCorreo,
    pub foto: Arc<Mutex<Foto>>,
    pub tabla_del_mapa: usize,
    pub apertura: Instant,
    pub cambio_de_formulario: Instant,
    pub cambio_de_vista: Instant,
    pub telon: Option<Instant>,
}

impl AppState {
    pub fn nuevo(db: Database, carpeta: PathBuf, foto: Arc<Mutex<Foto>>) -> Self {
        let ahora = Instant::now();
        Self {
            db,
            carpeta,
            servidor_correo: cuenta_correo::SERVIDOR_GMAIL.to_string(),
            pantalla: Pantalla::Acceso,
            sesion: None,
            mensaje: None,
            confirmar: None,
            acceso: Default::default(),
            inicial: Default::default(),
            perfil: Default::default(),
            persona: Default::default(),
            usuario: Default::default(),
            correo: Default::default(),
            foto,
            tabla_del_mapa: 2,
            apertura: ahora,
            cambio_de_formulario: ahora,
            cambio_de_vista: ahora,
            telon: None,
        }
    }

    fn avisar(&mut self, texto: impl Into<String>, tipo: TipoMensaje, segundos: u64) {
        self.mensaje = Some(Mensaje {
            texto: texto.into(),
            tipo,
            vence: Instant::now() + Duration::from_secs(segundos),
        });
    }

    pub fn exito(&mut self, texto: impl Into<String>) {
        self.avisar(texto, TipoMensaje::Exito, 5);
    }

    pub fn error(&mut self, texto: impl Into<String>) {
        self.avisar(texto, TipoMensaje::Error, 7);
    }

    pub fn informar(&mut self, texto: impl Into<String>) {
        self.avisar(texto, TipoMensaje::Informacion, 4);
    }

    pub fn vencer_mensaje(&mut self) {
        if self.mensaje.as_ref().is_some_and(|m| Instant::now() >= m.vence) {
            self.mensaje = None;
        }
    }

    pub fn es_administrador(&self) -> bool {
        self.sesion.as_ref().is_some_and(|s| s.administrador)
    }

    pub fn id_en_sesion(&self) -> Option<i64> {
        self.sesion.as_ref().map(|s| s.usuario.idusuario)
    }

    /// Permisos de la tabla de requisitos: Persona solo entra a sus datos.
    pub fn puede_entrar(&self, pantalla: Pantalla) -> bool {
        match pantalla {
            Pantalla::Perfil | Pantalla::Usuario | Pantalla::Mapa | Pantalla::Correo => self.es_administrador(),
            Pantalla::Panel | Pantalla::Persona => self.sesion.is_some(),
            _ => true,
        }
    }

    pub fn ir_a(&mut self, pantalla: Pantalla) {
        if !self.puede_entrar(pantalla) {
            self.error("Su perfil no tiene permiso para entrar ahí.");
            return;
        }
        self.confirmar = None;
        self.pantalla = pantalla;
        self.cambio_de_vista = Instant::now();
        match pantalla {
            Pantalla::Perfil => self.perfil_abrir(),
            Pantalla::Persona => self.persona_abrir(),
            Pantalla::Usuario => self.usuario_abrir(),
            Pantalla::Correo => self.correo_abrir(),
            _ => {}
        }
    }

    pub fn cerrar_sesion(&mut self) {
        self.sesion = None;
        self.acceso = Default::default();
        self.perfil = Default::default();
        self.persona = Default::default();
        self.usuario = Default::default();
        self.correo = Default::default();
        self.mensaje = None;
        self.confirmar = None;
        self.pantalla = Pantalla::Acceso;
        self.cambio_de_formulario = Instant::now();
    }

    /// El usuario aceptó la confirmación que estaba en la hoja.
    pub fn confirmado(&mut self) {
        if let Some(c) = self.confirmar.take() {
            self.ejecutar(c.accion);
        }
    }

    pub fn ejecutar(&mut self, accion: AccionPendiente) {
        match accion {
            AccionPendiente::InhabilitarPerfil(id) => self.perfil_inhabilitar(id),
            AccionPendiente::InhabilitarPersona(id) => self.persona_inhabilitar(id),
            AccionPendiente::InhabilitarUsuario(id) => self.usuario_inhabilitar(id),
        }
    }

    pub fn foto_actual(&self) -> Foto {
        self.foto.lock().map(|f| f.clone()).unwrap_or_default()
    }
}

/// Barra de texto arriba. Una línea se desliza hasta la opción elegida.
pub fn barra(contexto: &egui::Context, estado: &mut AppState) {
    let marco = egui::Frame::none().fill(NIEBLA).inner_margin(Margin::symmetric(56.0, 0.0));
    egui::TopBottomPanel::top("barra").exact_height(84.0).frame(marco).show_separator_line(false).show(contexto, |ui| {
        let opciones: &[(&str, Pantalla)] = if estado.es_administrador() {
            &[
                ("Panel", Pantalla::Panel),
                ("Personas", Pantalla::Persona),
                ("Usuarios", Pantalla::Usuario),
                ("Perfiles", Pantalla::Perfil),
                ("Base de datos", Pantalla::Mapa),
            ]
        } else {
            &[("Panel", Pantalla::Panel), ("Mis datos", Pantalla::Persona)]
        };
        let zona = ui.max_rect();
        let centro = zona.center().y;
        ui.painter().text(pos2(zona.left(), centro), Align2::LEFT_CENTER, "Farming 6", liviana(23.0), PINO);

        let mut x = zona.left() + 150.0;
        let mut destino = None;
        let mut linea = None;
        for (texto, pantalla) in opciones {
            let elegida = estado.pantalla == *pantalla;
            let fuente = FontId::proportional(15.0);
            let galeria = ui.painter().layout_no_wrap(texto.to_string(), fuente, egui::Color32::PLACEHOLDER);
            let tamano = galeria.size();
            let rect = egui::Rect::from_min_size(pos2(x, centro - 18.0), vec2(tamano.x, 36.0));
            let respuesta = ui.interact(rect, Id::new("nav").with(texto), Sense::click());
            let color = if elegida || respuesta.hovered() { PINO } else { SALVIA };
            ui.painter().galley(pos2(x, centro - tamano.y / 2.0), galeria, color);
            if respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                destino = Some(*pantalla);
            }
            if elegida {
                linea = Some((x, tamano.x));
            }
            x += tamano.x + 32.0;
        }
        if let Some((inicio, ancho)) = linea {
            let contexto = ui.ctx();
            let x = contexto.animate_value_with_time(Id::new("nav_x"), inicio, duracion(0.22));
            let w = contexto.animate_value_with_time(Id::new("nav_w"), ancho, duracion(0.22));
            let y = centro + 17.0;
            ui.painter().line_segment([pos2(x, y), pos2(x + w, y)], Stroke::new(1.6_f32, MAIZ));
        }

        let mut derecha = zona.right();
        let mut accion_derecha = |ui: &mut egui::Ui, texto: &str, color| {
            let galeria = ui.painter().layout_no_wrap(texto.to_string(), FontId::proportional(14.0), color);
            let ancho = galeria.size().x;
            derecha -= ancho;
            let rect = egui::Rect::from_min_size(pos2(derecha, centro - 16.0), vec2(ancho, 32.0));
            let respuesta = ui.interact(rect, Id::new("nav_der").with(texto), Sense::click());
            ui.painter().galley(pos2(derecha, centro - galeria.size().y / 2.0), galeria, color);
            derecha -= 26.0;
            respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked()
        };
        let salir = accion_derecha(ui, "Salir", PINO);
        let nombre = estado.sesion.as_ref().map(|s| s.usuario.nombreu.clone()).unwrap_or_default();
        accion_derecha(ui, &nombre, SALVIA);
        let correo = estado.es_administrador() && accion_derecha(ui, "Correo", SALVIA);

        if salir {
            estado.cerrar_sesion();
            return;
        }
        if correo {
            destino = Some(Pantalla::Correo);
        }
        if let Some(pantalla) = destino {
            estado.mensaje = None;
            estado.ir_a(pantalla);
        }
    });
}
