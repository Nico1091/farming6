//! Identidad visual: pino, niebla y un solo acento. Los valores salen de docs/PROMPT-DISENO.md.

use eframe::egui::epaint::Shadow;
use eframe::egui::{
    self, vec2, Color32, FontData, FontDefinitions, FontFamily, FontId, Margin, Rounding, Stroke, TextStyle,
};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

pub const PINO: Color32 = Color32::from_rgb(18, 51, 43);
pub const NIEBLA: Color32 = Color32::from_rgb(243, 245, 241);
pub const HOJA: Color32 = Color32::from_rgb(251, 252, 250);
pub const SALVIA: Color32 = Color32::from_rgb(110, 129, 119);
pub const LINEA: Color32 = Color32::from_rgb(220, 226, 219);
pub const MAIZ: Color32 = Color32::from_rgb(224, 160, 48);
pub const ARCILLA: Color32 = Color32::from_rgb(181, 71, 47);

/// Colores de los controles según el fondo sobre el que están.
#[derive(Clone, Copy)]
pub struct Paleta {
    pub texto: Color32,
    pub suave: Color32,
    pub linea: Color32,
    pub foco: Color32,
    pub error: Color32,
    pub boton: Color32,
    pub boton_texto: Color32,
}

pub const SOBRE_PINO: Paleta = Paleta {
    texto: Color32::from_rgb(240, 244, 239),
    suave: Color32::from_rgb(158, 182, 170),
    linea: Color32::from_rgb(70, 106, 94),
    foco: MAIZ,
    error: Color32::from_rgb(244, 178, 156),
    boton: NIEBLA,
    boton_texto: PINO,
};

pub const SOBRE_NIEBLA: Paleta = Paleta {
    texto: PINO,
    suave: SALVIA,
    linea: LINEA,
    foco: PINO,
    error: ARCILLA,
    boton: PINO,
    boton_texto: NIEBLA,
};

static ANIMAR: AtomicBool = AtomicBool::new(true);

#[link(name = "user32")]
extern "system" {
    fn SystemParametersInfoW(accion: u32, parametro: u32, valor: *mut i32, opciones: u32) -> i32;
}

/// Lee si Windows tiene activadas las animaciones de la interfaz.
fn leer_preferencia_de_animacion() {
    const SPI_GETCLIENTAREAANIMATION: u32 = 0x1042;
    let mut activas: i32 = 1;
    // SAFETY: el puntero apunta a una variable válida durante toda la llamada.
    let correcto = unsafe { SystemParametersInfoW(SPI_GETCLIENTAREAANIMATION, 0, &mut activas, 0) };
    ANIMAR.store(correcto == 0 || activas != 0, Ordering::Relaxed);
}

pub fn animaciones() -> bool {
    ANIMAR.load(Ordering::Relaxed)
}

/// Duración de una animación; casi cero si Windows las tiene apagadas.
pub fn duracion(segundos: f32) -> f32 {
    if animaciones() {
        segundos
    } else {
        0.001
    }
}

pub fn semi(tamano: f32) -> FontId {
    FontId::new(tamano, FontFamily::Name("semi".into()))
}

pub fn liviana(tamano: f32) -> FontId {
    FontId::new(tamano, FontFamily::Name("liviana".into()))
}

fn fuentes(contexto: &egui::Context) {
    let mut definiciones = FontDefinitions::default();
    let raiz = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let carpeta = Path::new(&raiz).join("Fonts");

    if let Ok(bytes) = std::fs::read(carpeta.join("segoeui.ttf")) {
        definiciones.font_data.insert("segoe".into(), FontData::from_owned(bytes));
        if let Some(familia) = definiciones.families.get_mut(&FontFamily::Proportional) {
            familia.insert(0, "segoe".into());
        }
    }
    let base = definiciones.families.get(&FontFamily::Proportional).cloned().unwrap_or_default();
    for (familia, archivo) in [("semi", "seguisb.ttf"), ("liviana", "segoeuil.ttf")] {
        let mut lista = base.clone();
        if let Ok(bytes) = std::fs::read(carpeta.join(archivo)) {
            definiciones.font_data.insert(familia.into(), FontData::from_owned(bytes));
            lista.insert(0, familia.into());
        }
        definiciones.families.insert(FontFamily::Name(familia.into()), lista);
    }
    contexto.set_fonts(definiciones);
}

pub fn aplicar(contexto: &egui::Context) {
    leer_preferencia_de_animacion();
    fuentes(contexto);
    contexto.set_theme(egui::Theme::Light);

    let mut estilo = (*contexto.style()).clone();
    estilo.text_styles = [
        (TextStyle::Heading, liviana(34.0)),
        (TextStyle::Body, FontId::proportional(15.0)),
        (TextStyle::Button, FontId::proportional(15.0)),
        (TextStyle::Small, FontId::proportional(13.0)),
        (TextStyle::Monospace, FontId::monospace(13.0)),
    ]
    .into();

    let v = &mut estilo.visuals;
    let sin_trazo = Stroke::NONE;
    let curva = Rounding::same(6.0);
    v.override_text_color = Some(PINO);
    v.panel_fill = NIEBLA;
    v.window_fill = HOJA;
    v.window_stroke = sin_trazo;
    v.window_shadow = Shadow::NONE;
    v.popup_shadow = Shadow { offset: vec2(0.0, 6.0), blur: 20.0, spread: 0.0, color: Color32::from_black_alpha(26) };
    v.menu_rounding = curva;
    v.extreme_bg_color = Color32::TRANSPARENT;
    v.selection.bg_fill = Color32::from_rgb(214, 228, 216);
    v.selection.stroke = Stroke::new(1.5_f32, PINO);
    v.text_cursor.stroke = Stroke::new(1.5_f32, MAIZ);
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_fill = Color32::TRANSPARENT;
        w.weak_bg_fill = Color32::TRANSPARENT;
        w.bg_stroke = sin_trazo;
        w.rounding = curva;
        w.expansion = 0.0;
    }
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(232, 237, 231);
    v.widgets.active.weak_bg_fill = Color32::from_rgb(226, 232, 225);

    estilo.spacing.item_spacing = vec2(12.0, 8.0);
    estilo.spacing.button_padding = vec2(12.0, 8.0);
    estilo.spacing.window_margin = Margin::same(10.0);
    estilo.spacing.scroll.bar_width = 6.0;
    estilo.animation_time = duracion(0.14);
    contexto.set_style(estilo);
}
