//! Cuenta desde la que sale el correo de recuperación (solo Administrador).

use super::comun::*;
use super::theme::*;
use super::{AppState, Pantalla};
use crate::auth::correo::{self as cuenta_correo, Cuenta};
use crate::validaciones;
use eframe::egui::{self, RichText};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;

#[derive(Default)]
pub struct FormCorreo {
    pub correo: String,
    pub clave: String,
    pub ver_clave: bool,
    pub configurada: bool,
    pub prueba: Option<Receiver<Result<(), String>>>,
}

impl AppState {
    pub fn correo_abrir(&mut self) {
        self.correo = FormCorreo::default();
        if let Some(cuenta) = cuenta_correo::cargar(&self.carpeta) {
            self.correo.correo = cuenta.correo;
            self.correo.configurada = true;
        }
    }

    pub fn correo_guardar(&mut self) {
        let correo = self.correo.correo.trim().to_lowercase();
        if let Err(e) = validaciones::correo(&correo) {
            return self.error(e);
        }
        let clave = cuenta_correo::limpiar_clave(&self.correo.clave);
        if clave.len() != 16 {
            return self.error("La clave de aplicación de Google tiene 16 letras.");
        }
        match cuenta_correo::guardar(&self.carpeta, &Cuenta { correo, clave }) {
            Ok(()) => {
                self.correo.clave.clear();
                self.correo.configurada = true;
                self.exito("Cuenta guardada y cifrada para este usuario de Windows.");
            }
            Err(e) => self.error(e),
        }
    }

    /// Envía un correo de prueba a la misma cuenta para comprobar la conexión.
    pub fn correo_probar(&mut self) {
        let Some(cuenta) = cuenta_correo::cargar(&self.carpeta) else {
            return self.error("Primero guarde la cuenta.");
        };
        let servidor = self.servidor_correo.clone();
        let (emisor, receptor) = mpsc::channel();
        thread::spawn(move || {
            let mensaje = cuenta_correo::mensaje_de_prueba(&cuenta.correo, &cuenta.correo);
            let resultado = cuenta_correo::enviar(&servidor, Some(&cuenta), &cuenta.correo, &cuenta.correo, &mensaje);
            let _ = emisor.send(resultado);
        });
        self.correo.prueba = Some(receptor);
    }

    pub fn revisar_prueba_de_correo(&mut self) {
        let Some(receptor) = &self.correo.prueba else {
            return;
        };
        let resultado = match receptor.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("La prueba se interrumpió.".to_string()),
        };
        self.correo.prueba = None;
        match resultado {
            Ok(()) => self.exito("El servidor aceptó el correo de prueba. Revise la bandeja de entrada."),
            Err(e) => self.error(e),
        }
    }
}

pub fn pantalla(ui: &mut egui::Ui, estado: &mut AppState) {
    cabecera(ui, "Correo de recuperación", None);
    ui.allocate_ui(egui::vec2(440.0, ui.available_height()), |ui| formulario(ui, estado));
}

fn formulario(ui: &mut egui::Ui, estado: &mut AppState) {
    let (texto, color) = if estado.correo.configurada {
        ("Hay una cuenta guardada. La clave no se muestra nunca.", SALVIA)
    } else {
        ("Sin una cuenta aquí, la recuperación no puede enviar correos.", ARCILLA)
    };
    ui.label(RichText::new(texto).color(color));
    ui.add_space(14.0);

    let form = &mut estado.correo;
    campo(ui, SOBRE_NIEBLA, "Cuenta de Gmail", &mut form.correo, true);
    campo_clave(ui, SOBRE_NIEBLA, "Clave de aplicación de Google", &mut form.clave, &mut form.ver_clave, true);
    ui.label(RichText::new("Se crea en la cuenta de Google, en Seguridad. No es la contraseña normal.").size(13.0).color(SALVIA));
    ui.add_space(18.0);

    let probando = estado.correo.prueba.is_some();
    let puede_probar = estado.correo.configurada && !probando;
    ui.horizontal(|ui| {
        if boton(ui, SOBRE_NIEBLA, "Guardar", true) {
            estado.correo_guardar();
        }
        ui.add_space(14.0);
        let prueba = if probando { "Enviando prueba" } else { "Enviar prueba" };
        if enlace(ui, PINO, prueba, puede_probar, 15.0) {
            estado.correo_probar();
        }
        ui.add_space(14.0);
        if enlace(ui, SALVIA, "Salir", true, 15.0) {
            estado.mensaje = None;
            estado.ir_a(Pantalla::Panel);
        }
    });
}
