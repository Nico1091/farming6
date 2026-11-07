//! Base de datos en tiempo real, en su propia pantalla. Todo sale de la foto del vigía: tablas,
//! columnas, llaves, relaciones, filas y cambios leídos de SQLite mientras se usa la aplicación.
//! Cada tabla es un cuadro con sus columnas, unido a las demás por sus relaciones, y debajo
//! van los registros en una rejilla.

use super::comun::*;
use super::theme::*;
use super::AppState;
use crate::db::vigia::{Accion, Foto, Relacion, TablaViva};
use eframe::egui::epaint::CubicBezierShape;
use eframe::egui::{
    self, pos2, vec2, Align2, Color32, FontId, Label, Painter, Pos2, Rect, RichText, Rounding, Sense, Shape, Stroke, Ui,
    UiBuilder,
};
use std::time::Duration;

const CABECERA: f32 = 40.0;
const FILA: f32 = 26.0;
const PIE: f32 = 30.0;
const LETRA: f32 = 14.0;
const ANCHO_LLAVE: f32 = 44.0;
const ALTO_LEYENDA: f32 = 24.0;
const ALTO_CELDA: f32 = 32.0;
const REDONDEO: f32 = 4.0;
const BORDE: Color32 = Color32::from_rgb(176, 188, 178);
const LLAVE: Color32 = Color32::from_rgb(156, 102, 14);
const SOBRE_PINO: Color32 = Color32::from_rgb(196, 211, 202);

/// perfil y persona quedan a los lados de usuario, que es quien las referencia.
const ORDEN: [&str; 3] = ["perfil", "usuario", "persona"];

struct Bloque<'a> {
    tabla: &'a TablaViva,
    rect: Rect,
}

impl Bloque<'_> {
    fn y_de(&self, columna: &str) -> f32 {
        let i = self.tabla.columnas.iter().position(|c| c.nombre == columna).unwrap_or(0);
        self.rect.top() + CABECERA + FILA * (i as f32 + 0.5)
    }
}

fn alto_de(t: &TablaViva) -> f32 {
    CABECERA + FILA * t.columnas.len() as f32 + PIE
}

fn tablas_en_orden(foto: &Foto) -> Vec<&TablaViva> {
    let mut tablas: Vec<&TablaViva> = foto.tablas.iter().collect();
    tablas.sort_by_key(|t| ORDEN.iter().position(|n| *n == t.nombre).unwrap_or(ORDEN.len()));
    tablas
}

fn cantidad(n: usize, singular: &str) -> String {
    if n == 1 {
        format!("1 {singular}")
    } else {
        format!("{n} {singular}s")
    }
}

fn mezclar(a: Color32, b: Color32, t: f32) -> Color32 {
    let canal = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t) as u8;
    Color32::from_rgb(canal(a.r(), b.r()), canal(a.g(), b.g()), canal(a.b(), b.b()))
}

fn solo_arriba() -> Rounding {
    Rounding { nw: REDONDEO, ne: REDONDEO, sw: 0.0, se: 0.0 }
}

fn pintar_bloque(pintor: &Painter, b: &Bloque, foto: &Foto) {
    let (t, r) = (b.tabla, b.rect);
    let brillo = foto.resaltado_tabla(&t.nombre);
    pintor.rect_filled(r, REDONDEO, HOJA);

    let cabecera = Rect::from_min_size(r.min, vec2(r.width(), CABECERA));
    pintor.rect_filled(cabecera, solo_arriba(), PINO);
    pintor.text(pos2(r.left() + 12.0, cabecera.center().y), Align2::LEFT_CENTER, &t.nombre, liviana(20.0), HOJA);
    let cuantas = cantidad(t.filas.len(), "fila");
    pintor.text(pos2(r.right() - 12.0, cabecera.center().y), Align2::RIGHT_CENTER, cuantas, FontId::proportional(13.0), SOBRE_PINO);

    for (i, c) in t.columnas.iter().enumerate() {
        let y = r.top() + CABECERA + FILA * i as f32;
        let fila = Rect::from_min_size(pos2(r.left(), y), vec2(r.width(), FILA));
        let destello = foto.resaltado_columna(&t.nombre, &c.nombre);
        if destello > 0.0 {
            pintor.rect_filled(fila, 0.0, MAIZ.gamma_multiply(0.3 * destello));
        }
        if i > 0 {
            pintor.line_segment([fila.left_top(), fila.right_top()], Stroke::new(1.0_f32, LINEA));
        }
        let llave = match (c.llave_primaria, c.foranea.is_some()) {
            (true, true) => "PK FK",
            (true, false) => "PK",
            (false, true) => "FK",
            (false, false) => "",
        };
        let centro = fila.left_center() + vec2(ANCHO_LLAVE / 2.0, 0.0);
        pintor.text(centro, Align2::CENTER_CENTER, llave, FontId::proportional(11.5), LLAVE);
        let color = if c.llave_primaria || c.foranea.is_some() { PINO } else { SALVIA };
        let texto = fila.left_center() + vec2(ANCHO_LLAVE + 10.0, 0.0);
        pintor.text(texto, Align2::LEFT_CENTER, &c.nombre, FontId::proportional(LETRA), color);
        let tipo = fila.right_center() - vec2(12.0, 0.0);
        pintor.text(tipo, Align2::RIGHT_CENTER, c.tipo.to_lowercase(), FontId::proportional(12.0), SALVIA);
    }
    let x_llave = r.left() + ANCHO_LLAVE;
    let fin_filas = r.bottom() - PIE;
    pintor.line_segment([pos2(x_llave, cabecera.bottom()), pos2(x_llave, fin_filas)], Stroke::new(1.0_f32, LINEA));

    pintor.line_segment([pos2(r.left(), fin_filas), pos2(r.right(), fin_filas)], Stroke::new(1.0_f32, BORDE));
    let y = fin_filas + PIE / 2.0;
    pintor.text(pos2(r.left() + 12.0, y), Align2::LEFT_CENTER, cantidad(t.activos, "activo"), FontId::proportional(13.0), SALVIA);
    if t.inhabilitados > 0 {
        let texto = cantidad(t.inhabilitados, "inhabilitado");
        pintor.text(pos2(r.right() - 12.0, y), Align2::RIGHT_CENTER, texto, FontId::proportional(13.0), ARCILLA);
    }
    pintor.rect_stroke(r, REDONDEO, Stroke::new(1.0 + brillo, mezclar(BORDE, MAIZ, brillo)));
}

fn punto_de_curva(p: [Pos2; 4], t: f32) -> Pos2 {
    let u = 1.0 - t;
    let suma = p[0].to_vec2() * (u * u * u)
        + p[1].to_vec2() * (3.0 * u * u * t)
        + p[2].to_vec2() * (3.0 * u * t * t)
        + p[3].to_vec2() * (t * t * t);
    pos2(suma.x, suma.y)
}

/// Una relación real del esquema: de la llave primaria a la columna que la referencia.
fn pintar_relacion(pintor: &Painter, bloques: &[Bloque], relacion: &Relacion, foto: &Foto, tiempo: f32) {
    let origen = bloques.iter().find(|b| b.tabla.nombre == relacion.tabla_destino);
    let destino = bloques.iter().find(|b| b.tabla.nombre == relacion.tabla);
    let (Some(origen), Some(destino)) = (origen, destino) else {
        return;
    };
    let y1 = origen.y_de(&relacion.columna_destino);
    let y2 = destino.y_de(&relacion.columna);
    let (o, d) = (origen.rect, destino.rect);
    let puntos = if o.right() <= d.left() {
        let (a, b) = (pos2(o.right(), y1), pos2(d.left(), y2));
        let tension = (b.x - a.x) * 0.5;
        [a, a + vec2(tension, 0.0), b - vec2(tension, 0.0), b]
    } else {
        let (a, b) = (pos2(o.left(), y1), pos2(d.right(), y2));
        let tension = (a.x - b.x) * 0.5;
        [a, a - vec2(tension, 0.0), b + vec2(tension, 0.0), b]
    };


    let brillo = foto.resaltado_tabla(&relacion.tabla).max(foto.resaltado_tabla(&relacion.tabla_destino));
    let trazo = Stroke::new(1.2 + brillo * 0.8, mezclar(SALVIA, MAIZ, brillo));
    pintor.add(Shape::CubicBezier(CubicBezierShape::from_points_stroke(puntos, false, Color32::TRANSPARENT, trazo)));
    if brillo > 0.0 {
        pintor.circle_filled(punto_de_curva(puntos, tiempo * 0.9 % 1.0), 3.0, MAIZ);
    }

    let fuente = FontId::proportional(LETRA - 1.0);
    let cerca = |a: Pos2, b: Pos2| a + (b - a).normalized() * 12.0 - vec2(0.0, 10.0);
    pintor.text(cerca(puntos[0], puntos[1]), Align2::CENTER_CENTER, "1", fuente.clone(), SALVIA);
    let fin = if relacion.uno_a_uno { "1" } else { "∞" };
    pintor.text(cerca(puntos[3], puntos[2]), Align2::CENTER_CENTER, fin, fuente, SALVIA);
}

fn leyenda(pintor: &Painter, inicio: Pos2) {
    let fuente = || FontId::proportional(12.5);
    pintor.text(inicio, Align2::LEFT_CENTER, "PK", fuente(), LLAVE);
    pintor.text(inicio + vec2(22.0, 0.0), Align2::LEFT_CENTER, "llave primaria", fuente(), SALVIA);
    pintor.text(inicio + vec2(118.0, 0.0), Align2::LEFT_CENTER, "FK", fuente(), LLAVE);
    pintor.text(inicio + vec2(140.0, 0.0), Align2::LEFT_CENTER, "llave foránea", fuente(), SALVIA);
}

fn latido(ui: &mut Ui, foto: &Foto) {
    ui.horizontal(|ui| {
        let tiempo = ui.input(|i| i.time) as f32;
        let (rect, _) = ui.allocate_exact_size(vec2(12.0, 16.0), Sense::hover());
        let pulso = if animaciones() { 0.5 + 0.5 * (tiempo * 2.0).sin() } else { 0.0 };
        ui.painter().circle_filled(rect.center(), 3.5 + 3.0 * pulso, MAIZ.gamma_multiply(0.22));
        ui.painter().circle_filled(rect.center(), 3.5, MAIZ);
        let detalle = match &foto.error {
            Some(e) => format!("Sin lectura: {e}"),
            None => format!("En vivo, leída a las {}", foto.ultima_lectura),
        };
        ui.add(Label::new(RichText::new(detalle).size(13.0).color(SALVIA)).truncate());
    });
}

fn diagrama(ui: &mut Ui, foto: &Foto) {
    if foto.tablas.is_empty() {
        return;
    }
    let ancho = ui.available_width();
    let tablas = tablas_en_orden(foto);
    let espacio = 84.0;
    let cuantas = tablas.len() as f32;
    let ancho_bloque = ((ancho - espacio * (cuantas - 1.0)) / cuantas).clamp(150.0, 300.0);

    let (respuesta, pintor) = ui.allocate_painter(vec2(ancho, alto_del_diagrama(foto)), Sense::hover());
    let origen = respuesta.rect.min;
    let bloques: Vec<Bloque> = tablas
        .iter()
        .copied()
        .enumerate()
        .map(|(i, tabla)| {
            let esquina = origen + vec2(i as f32 * (ancho_bloque + espacio), 0.0);
            Bloque { tabla, rect: Rect::from_min_size(esquina, vec2(ancho_bloque, alto_de(tabla))) }
        })
        .collect();
    for bloque in &bloques {
        pintar_bloque(&pintor, bloque, foto);
    }
    let tiempo = ui.input(|i| i.time) as f32;
    for relacion in &foto.relaciones {
        pintar_relacion(&pintor, &bloques, relacion, foto, tiempo);
    }
    leyenda(&pintor, pos2(respuesta.rect.left(), respuesta.rect.bottom() - ALTO_LEYENDA / 2.0));
}

fn alto_del_diagrama(foto: &Foto) -> f32 {
    foto.tablas.iter().map(alto_de).fold(0.0, f32::max) + ALTO_LEYENDA + 10.0
}

fn lista_de_cambios(ui: &mut Ui, foto: &Foto, maximo: usize) {
    if foto.cambios.is_empty() {
        ui.label(RichText::new("Cada cambio guardado aparece aquí al instante.").size(13.0).color(SALVIA));
        return;
    }
    ui.spacing_mut().item_spacing.y = 1.0;
    for cambio in foto.cambios.iter().take(maximo) {
        let color = if cambio.accion == Accion::Inhabilitar || cambio.accion == Accion::Borrar { ARCILLA } else { PINO };
        let titulo = format!("{} {} {}", cambio.accion.texto(), cambio.tabla, cambio.clave);
        ui.add(Label::new(RichText::new(titulo).size(14.0).color(color)).truncate());
        let pie = format!("{}  {}", cambio.hora, cambio.detalle);
        ui.add(Label::new(RichText::new(pie).size(12.5).color(SALVIA)).truncate());
        ui.add_space(9.0);
    }
}

fn pedir_repintado(ui: &Ui, foto: &Foto) {
    if foto.hay_resaltados() {
        ui.ctx().request_repaint();
    } else {
        ui.ctx().request_repaint_after(Duration::from_millis(120));
    }
}


fn peso_de(columna: &str) -> f32 {
    match columna {
        "correo" | "contrasena" | "descripc" => 2.4,
        "direccion" | "nombreu" => 1.6,
        "estado" => 1.0,
        _ => 1.2,
    }
}

/// Registros de una tabla en una rejilla, con línea entre cada fila y cada columna.
fn rejilla(ui: &mut Ui, t: &TablaViva, foto: &Foto) {
    let ancho = ui.available_width();
    let total: f32 = t.columnas.iter().map(|c| peso_de(&c.nombre)).sum();
    let anchos: Vec<f32> = t.columnas.iter().map(|c| peso_de(&c.nombre) / total * ancho).collect();
    let renglones = t.filas.len().max(1) as f32;
    let (rect, _) = ui.allocate_exact_size(vec2(ancho, ALTO_CELDA * (renglones + 1.0)), Sense::hover());
    let pintor = ui.painter();
    pintor.rect_filled(rect, REDONDEO, HOJA);
    pintor.rect_filled(Rect::from_min_size(rect.min, vec2(ancho, ALTO_CELDA)), solo_arriba(), PINO);

    let renglon = |i: usize| Rect::from_min_size(rect.min + vec2(0.0, ALTO_CELDA * (i as f32 + 1.0)), vec2(ancho, ALTO_CELDA));
    for (i, fila) in t.filas.iter().enumerate() {
        let r = renglon(i);
        if i % 2 == 1 {
            pintor.rect_filled(r, 0.0, NIEBLA);
        }
        let destello = foto.resaltado_fila(&t.nombre, &fila[0]);
        if destello > 0.0 {
            pintor.rect_filled(r, 0.0, MAIZ.gamma_multiply(0.28 * destello));
        }
        if i > 0 {
            pintor.line_segment([r.left_top(), r.right_top()], Stroke::new(1.0_f32, LINEA));
        }
    }

    let mut x = rect.left();
    for (j, (columna, ancho_col)) in t.columnas.iter().zip(&anchos).enumerate() {
        let hueco = ancho_col - 16.0;
        let g = linea_de_texto(ui, &columna.nombre, FontId::proportional(13.5), HOJA, hueco);
        pintor.galley(pos2(x + 8.0, rect.top() + (ALTO_CELDA - g.size().y) / 2.0), g, HOJA);
        for (i, fila) in t.filas.iter().enumerate() {
            let valor = fila.get(j).map(String::as_str).unwrap_or("");
            let g = linea_de_texto(ui, valor, FontId::proportional(14.0), PINO, hueco);
            pintor.galley(pos2(x + 8.0, renglon(i).center().y - g.size().y / 2.0), g, PINO);
        }
        if j > 0 {
            pintor.line_segment([pos2(x, rect.top()), pos2(x, rect.bottom())], Stroke::new(1.0_f32, LINEA));
        }
        x += ancho_col;
    }
    if t.filas.is_empty() {
        pintor.text(renglon(0).center(), Align2::CENTER_CENTER, "Sin registros.", FontId::proportional(14.0), SALVIA);
    }
    pintor.rect_stroke(rect, REDONDEO, Stroke::new(1.0_f32, BORDE));
}

fn pestanas(ui: &mut Ui, foto: &Foto, elegida: &mut usize) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 22.0;
        for (i, t) in foto.tablas.iter().enumerate() {
            let color = if *elegida == i { PINO } else { SALVIA };
            let (rect, respuesta) = {
                let g = ui.painter().layout_no_wrap(t.nombre.clone(), FontId::proportional(15.0), color);
                let (rect, respuesta) = ui.allocate_exact_size(g.size() + vec2(0.0, 8.0), Sense::click());
                ui.painter().galley(rect.left_top(), g, color);
                (rect, respuesta)
            };
            if *elegida == i {
                let y = rect.bottom() - 1.0;
                ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.6_f32, MAIZ));
            }
            if respuesta.on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
                *elegida = i;
            }
        }
    });
}

/// Pantalla Base de datos: el modelo, los cambios y las filas de cada tabla.
pub fn pantalla(ui: &mut Ui, estado: &mut AppState) {
    let foto = estado.foto_actual();
    egui::ScrollArea::vertical().id_salt("pagina_mapa").auto_shrink([false, false]).show(ui, |ui| {
        ui.set_width(ui.available_width() - 8.0);
        ui.horizontal(|ui| {
            ui.label(RichText::new("Base de datos").font(liviana(34.0)).color(PINO));
            ui.add_space(14.0);
            latido(ui, &foto);
        });
        ui.add_space(22.0);

        let ancho = ui.available_width();
        let (zona, _) = ui.allocate_exact_size(vec2(ancho, alto_del_diagrama(&foto)), Sense::hover());
        let izquierda = Rect::from_min_size(zona.min, vec2(ancho - 332.0, zona.height()));
        let derecha = Rect::from_min_max(pos2(zona.right() - 276.0, zona.top()), zona.max);
        ui.allocate_new_ui(UiBuilder::new().max_rect(izquierda), |ui| diagrama(ui, &foto));
        ui.allocate_new_ui(UiBuilder::new().max_rect(derecha), |ui| {
            ui.label(RichText::new("Últimos cambios").size(13.0).color(SALVIA));
            ui.add_space(6.0);
            egui::ScrollArea::vertical().id_salt("cambios_pagina").auto_shrink([false, false]).show(ui, |ui| {
                lista_de_cambios(ui, &foto, 30);
            });
        });

        ui.add_space(26.0);
        pestanas(ui, &foto, &mut estado.tabla_del_mapa);
        ui.add_space(8.0);
        if let Some(t) = foto.tablas.get(estado.tabla_del_mapa) {
            rejilla(ui, t, &foto);
        }
    });
    pedir_repintado(ui, &foto);
}
