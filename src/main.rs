#![cfg_attr(not(test), windows_subsystem = "windows")]

//! Farming 6: acceso al sistema, menú y CRUD de Perfil, Usuario y Persona.
//! Parcial I de Ingeniería de Software III.

mod auth;
#[cfg(feature = "captura")]
mod captura;
mod db;
mod models;
#[cfg(test)]
mod pruebas;
mod reloj;
mod ui;
mod validaciones;

use db::{vigia, Database};
use eframe::egui::{self, Margin};
use std::path::PathBuf;
use std::time::Duration;
use ui::theme::{self, NIEBLA};
use ui::{comun, dashboard, login, mapa, perfil, persona, usuario, AppState, Pantalla};

/// Icono de la ventana: el sol sobre la cordillera, dibujado sin archivos externos.
fn icono() -> egui::IconData {
    const LADO: u32 = 64;
    let mut rgba = Vec::with_capacity((LADO * LADO * 4) as usize);
    for y in 0..LADO {
        for x in 0..LADO {
            let (px, py) = (x as f32 + 0.5, y as f32 + 0.5);
            let dentro = (px - 32.0).hypot(py - 32.0) <= 32.0;
            let cima = 44.0 - px * 0.16 + (px * 0.17).sin() * 3.5;
            let sol = (px - 42.0).hypot(py - 24.0) <= 10.0;
            let color = match (dentro, py > cima, sol) {
                (false, _, _) => [0, 0, 0, 0],
                (true, true, _) => [44, 99, 85, 255],
                (true, false, true) => [224, 160, 48, 255],
                (true, false, false) => [18, 51, 43, 255],
            };
            rgba.extend_from_slice(&color);
        }
    }
    egui::IconData { rgba, width: LADO, height: LADO }
}

/// La base de datos vive junto al ejecutable.
fn carpeta_de_datos() -> PathBuf {
    if let Some(carpeta) = std::env::var_os("FARMING6_DATOS") {
        return PathBuf::from(carpeta);
    }
    std::env::current_exe()
        .ok()
        .and_then(|ruta| ruta.parent().map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from("."))
}

struct Aplicacion {
    estado: AppState,
    #[cfg(feature = "captura")]
    captura: captura::Guion,
}

fn interior(contexto: &egui::Context, estado: &mut AppState) {
    ui::barra(contexto, estado);
    if estado.sesion.is_none() {
        contexto.request_repaint();
        return;
    }

    let transicion = estado.cambio_de_vista.elapsed().as_secs_f32() / theme::duracion(0.18);
    let marco = egui::Frame::none().fill(NIEBLA).inner_margin(Margin { left: 56.0, right: 56.0, top: 12.0, bottom: 20.0 });
    egui::CentralPanel::default().frame(marco).show(contexto, |lienzo| {
        lienzo.set_opacity(transicion.clamp(0.0, 1.0));
        match estado.pantalla {
            Pantalla::Perfil => perfil::pantalla(lienzo, estado),
            Pantalla::Persona => persona::pantalla(lienzo, estado),
            Pantalla::Usuario => usuario::pantalla(lienzo, estado),
            Pantalla::Mapa => mapa::pantalla(lienzo, estado),
            Pantalla::Correo => ui::correo::pantalla(lienzo, estado),
            _ => dashboard::pantalla(lienzo, estado),
        }
    });
    if transicion < 1.0 {
        contexto.request_repaint();
    }

    match estado.pantalla {
        Pantalla::Perfil => perfil::hoja_lateral(contexto, estado),
        Pantalla::Persona => persona::hoja_lateral(contexto, estado),
        Pantalla::Usuario => usuario::hoja_lateral(contexto, estado),
        _ => {}
    }
    comun::aviso_flotante(contexto, estado);
}

impl eframe::App for Aplicacion {
    #[cfg(feature = "captura")]
    fn raw_input_hook(&mut self, _contexto: &egui::Context, entrada: &mut egui::RawInput) {
        self.captura.inyectar(entrada);
    }

    fn update(&mut self, contexto: &egui::Context, _marco: &mut eframe::Frame) {
        let estado = &mut self.estado;
        estado.revisar_envio();
        estado.revisar_prueba_de_correo();
        estado.vencer_mensaje();

        match estado.pantalla {
            Pantalla::Acceso => login::acceso(contexto, estado),
            Pantalla::Registro => login::registro(contexto, estado),
            _ => interior(contexto, estado),
        }
        login::telon(contexto, estado);

        let esperando = estado.acceso.envio.is_some() || estado.correo.prueba.is_some();
        if esperando {
            contexto.request_repaint_after(Duration::from_millis(100));
        } else if estado.mensaje.is_some() {
            contexto.request_repaint_after(Duration::from_millis(500));
        }

        #[cfg(feature = "captura")]
        self.captura.avanzar(contexto, &mut self.estado);
    }
}

fn main() -> eframe::Result<()> {
    let carpeta = carpeta_de_datos();
    let base = match Database::abrir(&carpeta.join("farming6.db")) {
        Ok(base) => base,
        Err(error) => panic!("No se pudo abrir la base de datos: {error}"),
    };
    let ruta = base.ruta.clone();

    let vista = egui::ViewportBuilder::default()
        .with_title("Farming 6")
        .with_icon(icono())
        .with_inner_size([1366.0, 800.0])
        .with_min_inner_size([1120.0, 680.0]);
    #[cfg(not(feature = "captura"))]
    let vista = vista.with_maximized(true);
    #[cfg(feature = "captura")]
    let vista = vista.with_position([-4000.0, 40.0]).with_active(false);
    #[cfg(feature = "captura")]
    let vista = if std::env::var_os("FARMING6_MINIMA").is_some() { vista.with_inner_size([1120.0, 680.0]) } else { vista };

    eframe::run_native(
        "Farming 6",
        eframe::NativeOptions { viewport: vista, ..Default::default() },
        Box::new(move |creacion| {
            ui::theme::aplicar(&creacion.egui_ctx);
            let foto = vigia::iniciar(ruta, Some(creacion.egui_ctx.clone()));
            Ok(Box::new(Aplicacion {
                estado: AppState::nuevo(base, carpeta, foto),
                #[cfg(feature = "captura")]
                captura: captura::Guion::nuevo(),
            }))
        }),
    )
}
