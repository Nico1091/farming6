//! Controles de la interfaz: campos de línea, botones, hoja lateral, avisos y listas.

use super::theme::*;
use super::{AppState, Modo, TipoMensaje};
use eframe::egui::emath::easing;
use eframe::egui::epaint::text::{LayoutJob, TextWrapping};
use eframe::egui::{
    self, lerp, pos2, vec2, Align, Align2, Area, Color32, CursorIcon, FontId, Frame, Galley, Id, Key, Layout, Margin, Order, Pos2,
    Rect, Response, RichText, Sense, Stroke, TextEdit, Ui,
};
use std::sync::Arc;
use std::time::Instant;

pub const ALTO_FILA: f32 = 46.0;
pub const ANCHO_HOJA: f32 = 460.0;

fn mezclar(a: Color32, b: Color32, t: f32) -> Color32 {
    let canal = |x: u8, y: u8| (f32::from(x) + (f32::from(y) - f32::from(x)) * t) as u8;
    Color32::from_rgb(canal(a.r(), b.r()), canal(a.g(), b.g()), canal(a.b(), b.b()))
}

/// Entrada escalonada: opacidad y un pequeño desplazamiento, una sola vez.
pub fn entrada<R>(ui: &mut Ui, desde: Instant, orden: usize, contenido: impl FnOnce(&mut Ui) -> R) -> R {
    let segundos = desde.elapsed().as_secs_f32() - orden as f32 * 0.06;
    let p = if animaciones() { easing::cubic_out((segundos / 0.38).clamp(0.0, 1.0)) } else { 1.0 };
    if p < 1.0 {
        ui.ctx().request_repaint();
    }
    let salto = 10.0 * (1.0 - p);
    ui.scope(|ui| {
        ui.set_opacity(p);
        ui.add_space(salto);
        let r = contenido(ui);
        ui.add_space(10.0 - salto);
        r
    })
    .inner
}

fn campo_base(
    ui: &mut Ui,
    p: Paleta,
    etiqueta: &str,
    valor: &mut String,
    activo: bool,
    oculto: bool,
    reserva: f32,
) -> Response {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 58.0), Sense::hover());
    let id = ui.id().with(etiqueta);
    let caja = Rect::from_min_max(pos2(rect.left(), rect.top() + 24.0), pos2(rect.right() - reserva, rect.bottom() - 8.0));
    let editor = TextEdit::singleline(valor)
        .id(id)
        .frame(false)
        .password(oculto)
        .font(FontId::proportional(16.0))
        .text_color(p.texto)
        .margin(Margin::ZERO)
        .desired_width(caja.width())
        .interactive(activo);
    let respuesta = ui.put(caja, editor);

    let contexto = ui.ctx().clone();
    let foco = contexto.animate_bool_with_time(id.with("foco"), respuesta.has_focus(), duracion(0.14));
    let lleno = !valor.is_empty();
    let flota = contexto.animate_bool_with_time(id.with("flota"), respuesta.has_focus() || lleno, duracion(0.14));
    let y = lerp((caja.center().y)..=(rect.top() + 11.0), flota);
    let tamano = lerp(16.0..=12.5, flota);
    ui.painter().text(pos2(rect.left(), y), Align2::LEFT_CENTER, etiqueta, FontId::proportional(tamano), p.suave);

    if activo {
        let y = rect.bottom() - 3.0;
        ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, p.linea));
        if foco > 0.0 {
            let fin = lerp(rect.left()..=rect.right(), foco);
            ui.painter().line_segment([pos2(rect.left(), y), pos2(fin, y)], Stroke::new(1.6_f32, p.foco));
        }
    }
    respuesta
}

pub fn campo(ui: &mut Ui, p: Paleta, etiqueta: &str, valor: &mut String, activo: bool) -> Response {
    campo_base(ui, p, etiqueta, valor, activo, false, 0.0)
}

pub fn campo_clave(ui: &mut Ui, p: Paleta, etiqueta: &str, valor: &mut String, ver: &mut bool, activo: bool) -> Response {
    let respuesta = campo_base(ui, p, etiqueta, valor, activo, !*ver, 54.0);
    let texto = if *ver { "Ocultar" } else { "Ver" };
    let centro = pos2(respuesta.rect.right() + 54.0, respuesta.rect.center().y);
    let zona = Rect::from_center_size(centro - vec2(24.0, 0.0), vec2(52.0, 26.0));
    let boton = ui.interact(zona, respuesta.id.with("ver"), Sense::click()).on_hover_cursor(CursorIcon::PointingHand);
    if boton.clicked() {
        *ver = !*ver;
    }
    ui.painter().text(centro, Align2::RIGHT_CENTER, texto, FontId::proportional(13.0), p.suave);
    respuesta
}

/// El único botón sólido de cada vista.
pub fn boton(ui: &mut Ui, p: Paleta, texto: &str, activo: bool) -> bool {
    let sentido = if activo { Sense::click() } else { Sense::hover() };
    let galeria = ui.painter().layout_no_wrap(texto.to_string(), semi(15.0), p.boton_texto);
    let tamano = vec2((galeria.size().x + 44.0).max(128.0), 44.0);
    let (rect, respuesta) = ui.allocate_exact_size(tamano, sentido);
    let id = respuesta.id;
    let encima = ui.ctx().animate_bool_with_time(id.with("encima"), respuesta.hovered(), duracion(0.12));
    let presion = ui.ctx().animate_bool_with_time(id.with("presion"), respuesta.is_pointer_button_down_on(), duracion(0.08));
    let rect = rect.shrink(presion * 1.2);
    let fondo = if activo { p.boton } else { p.boton.gamma_multiply(0.4) };
    ui.painter().rect_filled(rect, 6.0, fondo);
    if encima > 0.0 {
        ui.painter().rect_filled(rect, 6.0, p.foco.gamma_multiply(0.16 * encima));
    }
    let origen = rect.center() - galeria.size() / 2.0;
    ui.painter().galley(origen, galeria, p.boton_texto);
    if respuesta.has_focus() {
        ui.painter().rect_stroke(rect.expand(2.5), 8.0, Stroke::new(1.5_f32, MAIZ));
    }
    respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Acción de texto. La línea crece al pasar el puntero.
pub fn enlace(ui: &mut Ui, color: Color32, texto: &str, activo: bool, tamano: f32) -> bool {
    let sentido = if activo { Sense::click() } else { Sense::hover() };
    let color = if activo { color } else { color.gamma_multiply(0.45) };
    let galeria = ui.painter().layout_no_wrap(texto.to_string(), FontId::proportional(tamano), color);
    let (rect, respuesta) = ui.allocate_exact_size(galeria.size() + vec2(0.0, 6.0), sentido);
    ui.painter().galley(rect.left_top(), galeria, color);
    let marcado = respuesta.hovered() || respuesta.has_focus();
    let encima = ui.ctx().animate_bool_with_time(respuesta.id.with("linea"), marcado, duracion(0.14));
    if encima > 0.0 {
        let y = rect.bottom() - 2.0;
        let fin = lerp(rect.left()..=rect.right(), encima);
        ui.painter().line_segment([pos2(rect.left(), y), pos2(fin, y)], Stroke::new(1.0_f32, color));
    }
    respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

/// Interruptor del campo estado.
pub fn interruptor(ui: &mut Ui, p: Paleta, activo: &mut bool, habilitado: bool) {
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        let sentido = if habilitado { Sense::click() } else { Sense::hover() };
        let (rect, respuesta) = ui.allocate_exact_size(vec2(38.0, 20.0), sentido);
        if respuesta.clicked() {
            *activo = !*activo;
        }
        let avance = ui.ctx().animate_bool_with_time(respuesta.id, *activo, duracion(0.14));
        let riel = mezclar(ARCILLA, p.foco, avance);
        ui.painter().rect_filled(rect, 10.0, if habilitado { riel } else { riel.gamma_multiply(0.45) });
        let x = lerp((rect.left() + 10.0)..=(rect.right() - 10.0), avance);
        ui.painter().circle_filled(pos2(x, rect.center().y), 7.0, Color32::WHITE);
        if habilitado {
            respuesta.on_hover_cursor(CursorIcon::PointingHand);
        }
        let (texto, color) = if *activo { ("Activo", p.texto) } else { ("Inhabilitado", p.error) };
        ui.label(RichText::new(texto).color(color));
    });
    ui.add_space(6.0);
}

/// Línea de resultado dentro del formulario de ingreso. Reserva su alto para no mover nada.
pub fn mensaje_en_linea(ui: &mut Ui, p: Paleta, estado: &AppState) {
    let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 44.0), Sense::hover());
    if let Some(m) = &estado.mensaje {
        let color = if m.tipo == TipoMensaje::Error { p.error } else { p.texto };
        let g = ui.painter().layout(m.texto.clone(), FontId::proportional(14.0), color, rect.width());
        ui.painter().galley(pos2(rect.left(), rect.center().y - g.size().y / 2.0), g, color);
    }
}

/// Aviso del interior: entra desde abajo y se va solo.
pub fn aviso_flotante(contexto: &egui::Context, estado: &AppState) {
    let visible = estado.mensaje.is_some();
    let p = contexto.animate_bool_with_time(Id::new("aviso"), visible, duracion(0.2));
    let Some(m) = &estado.mensaje else {
        return;
    };
    let color = match m.tipo {
        TipoMensaje::Exito => MAIZ,
        TipoMensaje::Error => Color32::from_rgb(244, 138, 110),
        TipoMensaje::Informacion => Color32::from_rgb(158, 182, 170),
    };
    let abajo = contexto.screen_rect().bottom() - 28.0 - 44.0 + (1.0 - easing::cubic_out(p)) * 18.0;
    Area::new(Id::new("aviso_area"))
        .order(Order::Tooltip)
        .fixed_pos(pos2(56.0, abajo))
        .interactable(false)
        .show(contexto, |ui| {
            ui.set_opacity(p);
            Frame::none().fill(PINO).rounding(6.0).inner_margin(Margin::symmetric(16.0, 11.0)).show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (punto, _) = ui.allocate_exact_size(vec2(8.0, 18.0), Sense::hover());
                    ui.painter().circle_filled(punto.center(), 4.0, color);
                    ui.label(RichText::new(&m.texto).color(NIEBLA).size(14.5));
                });
            });
        });
}

/// Hoja lateral que entra por la derecha. Devuelve verdadero si se pulsó fuera de ella.
pub fn hoja(contexto: &egui::Context, nombre: &str, abierta: bool, contenido: impl FnOnce(&mut Ui)) -> bool {
    let id = Id::new(nombre);
    let p = easing::cubic_out(contexto.animate_bool_with_time(id, abierta, duracion(0.24)));
    if p <= 0.0 {
        return false;
    }
    let pantalla = contexto.screen_rect();
    let mut fuera = false;
    Area::new(id.with("velo")).order(Order::PanelResizeLine).fixed_pos(pantalla.min).show(contexto, |ui| {
        let respuesta = ui.allocate_response(pantalla.size(), Sense::click());
        ui.painter().rect_filled(pantalla, 0.0, PINO.gamma_multiply(0.22 * p));
        fuera = respuesta.clicked();
    });
    let x = pantalla.right() - ANCHO_HOJA * p;
    Area::new(id.with("hoja")).order(Order::Middle).constrain(false).fixed_pos(pos2(x, 0.0)).show(contexto, |ui| {
        Frame::none().fill(HOJA).inner_margin(Margin::symmetric(40.0, 40.0)).show(ui, |ui| {
            ui.set_width(ANCHO_HOJA - 80.0);
            ui.set_min_height(pantalla.height() - 80.0);
            ui.set_max_height(pantalla.height() - 80.0);
            egui::ScrollArea::vertical().id_salt(id.with("desplazar")).auto_shrink([false, false]).show(ui, contenido);
        });
    });
    fuera || (abierta && contexto.input(|i| i.key_pressed(Key::Escape)))
}

pub fn titulo_de_hoja(ui: &mut Ui, texto: &str) {
    ui.label(RichText::new(texto).font(liviana(30.0)).color(PINO));
    ui.add_space(20.0);
}

/// Búsqueda con línea inferior. Devuelve verdadero con Enter o con la acción Consultar.
pub fn barra_de_consulta(ui: &mut Ui, pista: &str, valor: &mut String) -> bool {
    let mut pedir = false;
    ui.horizontal(|ui| {
        let ancho = (ui.available_width() - 90.0).clamp(160.0, 380.0);
        let (rect, _) = ui.allocate_exact_size(vec2(ancho, 38.0), Sense::hover());
        let id = ui.id().with("consulta");
        let editor = TextEdit::singleline(valor)
            .id(id)
            .frame(false)
            .hint_text(RichText::new(pista).color(SALVIA))
            .font(FontId::proportional(15.0))
            .text_color(PINO)
            .margin(Margin::ZERO)
            .desired_width(rect.width());
        let respuesta = ui.put(rect.shrink2(vec2(0.0, 8.0)), editor);
        pedir = respuesta.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        let foco = ui.ctx().animate_bool_with_time(id.with("foco"), respuesta.has_focus(), duracion(0.14));
        let y = rect.bottom() - 2.0;
        ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, LINEA));
        if foco > 0.0 {
            let fin = lerp(rect.left()..=rect.right(), foco);
            ui.painter().line_segment([pos2(rect.left(), y), pos2(fin, y)], Stroke::new(1.6_f32, PINO));
        }
        ui.add_space(6.0);
        pedir |= enlace(ui, PINO, "Consultar", true, 15.0);
    });
    ui.add_space(14.0);
    pedir
}

/// Lista desplegable con línea inferior, como los campos.
pub fn selector(ui: &mut Ui, etiqueta: &str, id: &str, texto: String, opciones: impl FnOnce(&mut Ui)) {
    ui.add_space(6.0);
    ui.label(RichText::new(etiqueta).size(12.5).color(SALVIA));
    ui.add_space(-4.0);
    ui.scope(|ui| {
        ui.spacing_mut().button_padding = vec2(0.0, 6.0);
        ui.visuals_mut().widgets.hovered.weak_bg_fill = Color32::TRANSPARENT;
        ui.visuals_mut().widgets.active.weak_bg_fill = Color32::TRANSPARENT;
        ui.visuals_mut().widgets.open.weak_bg_fill = Color32::TRANSPARENT;
        let r = egui::ComboBox::from_id_salt(id)
            .selected_text(RichText::new(texto).size(16.0).color(PINO))
            .truncate()
            .width(ui.available_width())
            .show_ui(ui, opciones);
        let rect = r.response.rect;
        let y = rect.bottom() + 1.0;
        ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, LINEA));
    });
    ui.add_space(12.0);
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum AccionHoja {
    Modificar,
    Inhabilitar,
    Guardar,
    Salir,
    Confirmado,
    Volver,
}

/// Pie de la hoja: Modificar, Inhabilitar y Salir en consulta; Guardar y Salir al escribir.
pub fn acciones(ui: &mut Ui, modo: Modo, inhabilitable: bool, pregunta: Option<&str>) -> Option<AccionHoja> {
    if let Some(texto) = pregunta {
        return confirmar_inhabilitar(ui, texto).map(|si| if si { AccionHoja::Confirmado } else { AccionHoja::Volver });
    }
    let mut accion = None;
    ui.horizontal(|ui| {
        if modo == Modo::Consulta {
            if boton(ui, SOBRE_NIEBLA, "Modificar", true) {
                accion = Some(AccionHoja::Modificar);
            }
            ui.add_space(10.0);
            if inhabilitable && enlace(ui, ARCILLA, "Inhabilitar", true, 15.0) {
                accion = Some(AccionHoja::Inhabilitar);
            }
        } else if boton(ui, SOBRE_NIEBLA, "Guardar", true) {
            accion = Some(AccionHoja::Guardar);
        }
        ui.add_space(10.0);
        if enlace(ui, SALVIA, "Salir", true, 15.0) {
            accion = Some(AccionHoja::Salir);
        }
    });
    accion
}

pub fn linea_de_texto(ui: &Ui, texto: &str, fuente: FontId, color: Color32, ancho: f32) -> Arc<Galley> {
    let mut trabajo = LayoutJob::simple_singleline(texto.to_string(), fuente, color);
    trabajo.wrap = TextWrapping::truncate_at_width(ancho);
    ui.fonts(|f| f.layout_job(trabajo))
}

pub struct Columna<'a> {
    pub titulo: &'a str,
    pub peso: f32,
}

pub const fn col(titulo: &str, peso: f32) -> Columna<'_> {
    Columna { titulo, peso }
}

/// Lista de registros: solo líneas horizontales. Activo no se marca; Inhabilitado sí.
pub fn tabla(
    ui: &mut Ui,
    id: &str,
    columnas: &[Columna],
    filas: &[Vec<String>],
    seleccion: Option<usize>,
    columna_estado: Option<usize>,
    resaltar: &dyn Fn(usize) -> f32,
) -> Option<usize> {
    let ancho = ui.available_width();
    let total: f32 = columnas.iter().map(|c| c.peso).sum();
    let anchos: Vec<f32> = columnas.iter().map(|c| c.peso / total * ancho).collect();

    let (cabecera, _) = ui.allocate_exact_size(vec2(ancho, 30.0), Sense::hover());
    let mut x = cabecera.left();
    for (columna, ancho_col) in columnas.iter().zip(&anchos) {
        let g = linea_de_texto(ui, columna.titulo, FontId::proportional(13.0), SALVIA, ancho_col - 12.0);
        ui.painter().galley(pos2(x, cabecera.center().y - g.size().y / 2.0), g, SALVIA);
        x += ancho_col;
    }
    let y = cabecera.bottom() - 0.5;
    ui.painter().line_segment([pos2(cabecera.left(), y), pos2(cabecera.right(), y)], Stroke::new(1.0_f32, LINEA));

    if filas.is_empty() {
        ui.add_space(22.0);
        ui.label(RichText::new("Sin resultados.").color(SALVIA));
        return None;
    }
    let mut pulsada = None;
    let fondo = ui.spacing().item_spacing.y;
    ui.spacing_mut().item_spacing.y = 0.0;
    ui.add_space(-fondo);
    egui::ScrollArea::vertical().id_salt(id).auto_shrink([false, true]).show_rows(ui, ALTO_FILA, filas.len(), |ui, rango| {
        for i in rango {
            let (rect, respuesta) = ui.allocate_exact_size(vec2(ui.available_width(), ALTO_FILA), Sense::click());
            let id_fila = respuesta.id;
            let encima = ui.ctx().animate_bool_with_time(id_fila.with("h"), respuesta.hovered(), duracion(0.12));
            if seleccion == Some(i) {
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(230, 236, 229));
            } else if encima > 0.0 {
                ui.painter().rect_filled(rect, 0.0, Color32::from_rgb(235, 239, 233).gamma_multiply(encima));
            }
            let destello = resaltar(i);
            if destello > 0.0 {
                ui.painter().rect_filled(rect, 0.0, MAIZ.gamma_multiply(0.28 * destello));
            }
            let y = rect.bottom() - 0.5;
            ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, LINEA.gamma_multiply(0.7)));
            let mut x = rect.left();
            for (j, ancho_col) in anchos.iter().enumerate() {
                let valor = filas[i].get(j).map(String::as_str).unwrap_or("");
                if columna_estado == Some(j) {
                    if valor != "A" && valor != "Activo" {
                        punto_inhabilitado(ui, pos2(x, rect.center().y));
                    }
                } else {
                    let g = linea_de_texto(ui, valor, FontId::proportional(15.0), PINO, ancho_col - 14.0);
                    ui.painter().galley(pos2(x, rect.center().y - g.size().y / 2.0), g, PINO);
                }
                x += ancho_col;
            }
            if respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                pulsada = Some(i);
            }
        }
    });
    ui.spacing_mut().item_spacing.y = fondo;
    pulsada
}

fn punto_inhabilitado(ui: &Ui, inicio: Pos2) {
    ui.painter().circle_filled(inicio + vec2(4.0, 0.0), 3.5, ARCILLA);
    ui.painter().text(inicio + vec2(15.0, 0.0), Align2::LEFT_CENTER, "Inhabilitado", FontId::proportional(14.0), ARCILLA);
}

/// Título de una vista con su acción principal a la derecha.
pub fn cabecera(ui: &mut Ui, titulo: &str, accion: Option<&str>) -> bool {
    let mut pulsado = false;
    ui.horizontal(|ui| {
        ui.label(RichText::new(titulo).font(liviana(34.0)).color(PINO));
        if let Some(texto) = accion {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                pulsado = boton(ui, SOBRE_NIEBLA, texto, true);
            });
        }
    });
    ui.add_space(14.0);
    pulsado
}

/// Dato en modo lectura: etiqueta tenue encima y valor debajo.
pub fn dato(ui: &mut Ui, etiqueta: &str, valor: &str) {
    ui.with_layout(Layout::top_down(Align::LEFT), |ui| {
        ui.label(RichText::new(etiqueta).size(12.5).color(SALVIA));
        let texto = if valor.is_empty() { "Sin dato" } else { valor };
        let color = if valor.is_empty() { SALVIA } else { PINO };
        ui.label(RichText::new(texto).size(16.0).color(color));
    });
    ui.add_space(10.0);
}

/// Confirmación dentro de la hoja. Devuelve Some(true) al confirmar y Some(false) al desistir.
pub fn confirmar_inhabilitar(ui: &mut Ui, texto: &str) -> Option<bool> {
    let mut decision = None;
    ui.label(RichText::new(texto).size(15.0).color(PINO));
    ui.label(RichText::new("El registro se conserva en la base de datos.").size(13.0).color(SALVIA));
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        let tono = Paleta { boton: ARCILLA, boton_texto: Color32::WHITE, ..SOBRE_NIEBLA };
        if boton(ui, tono, "Inhabilitar", true) {
            decision = Some(true);
        }
        ui.add_space(8.0);
        if enlace(ui, PINO, "Volver", true, 15.0) {
            decision = Some(false);
        }
    });
    decision
}

