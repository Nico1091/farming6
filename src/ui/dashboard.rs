//! Panel: un saludo, las cifras reales de la base y la franja de cordilleras.

use super::comun::*;
use super::theme::*;
use super::{paisaje, AppState, Pantalla};
use eframe::egui::{lerp, pos2, vec2, Align2, CursorIcon, FontId, Rect, RichText, Sense, Stroke, Ui};

fn cifra(ui: &mut Ui, valor: usize, etiqueta: &str, inhabilitados: usize) -> bool {
    let (rect, respuesta) = ui.allocate_exact_size(vec2(176.0, 136.0), Sense::click());
    let pintor = ui.painter();
    pintor.text(rect.left_top(), Align2::LEFT_TOP, valor.to_string(), liviana(64.0), PINO);
    let y = rect.top() + 92.0;
    pintor.text(pos2(rect.left(), y), Align2::LEFT_TOP, etiqueta, FontId::proportional(15.0), SALVIA);
    if inhabilitados > 0 {
        let texto = if inhabilitados == 1 { "1 inhabilitado".to_string() } else { format!("{inhabilitados} inhabilitados") };
        pintor.text(pos2(rect.left(), y + 22.0), Align2::LEFT_TOP, texto, FontId::proportional(13.0), ARCILLA);
    }
    let encima = ui.ctx().animate_bool_with_time(respuesta.id.with("h"), respuesta.hovered(), duracion(0.14));
    if encima > 0.0 {
        let fin = lerp(rect.left()..=(rect.left() + 40.0), encima);
        let linea = rect.top() + 88.0;
        ui.painter().line_segment([pos2(rect.left(), linea), pos2(fin, linea)], Stroke::new(1.6_f32, MAIZ));
    }
    respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked()
}

pub fn pantalla(ui: &mut Ui, estado: &mut AppState) {
    let desde = estado.cambio_de_vista;
    let nombre = estado.sesion.as_ref().map(|s| s.persona.nom1.clone()).unwrap_or_default();
    let perfil = estado.sesion.as_ref().map(|s| s.perfil.clone()).unwrap_or_default();

    let pantalla = ui.ctx().screen_rect();
    let banda = Rect::from_min_max(pos2(pantalla.left(), pantalla.bottom() - 260.0), pantalla.right_bottom());
    let segundos = if animaciones() { desde.elapsed().as_secs_f32() } else { 10.0 };
    paisaje::pintar(ui.painter(), banda, &paisaje::DIA, segundos, 0.0, 0.0);
    if segundos < 1.7 {
        ui.ctx().request_repaint();
    }

    ui.add_space(26.0);
    entrada(ui, desde, 0, |ui| {
        ui.label(RichText::new(format!("Hola, {nombre}")).font(liviana(64.0)).color(PINO));
        ui.label(RichText::new(perfil).color(SALVIA));
    });
    ui.add_space(26.0);

    if estado.es_administrador() {
        let foto = estado.foto_actual();
        let dato_de = |nombre: &str| foto.tabla(nombre).map(|t| (t.activos, t.inhabilitados)).unwrap_or((0, 0));
        let (perfiles, personas, usuarios) = (dato_de("perfil"), dato_de("persona"), dato_de("usuario"));
        let mut destino = None;
        entrada(ui, desde, 1, |ui| {
            ui.horizontal(|ui| {
                if cifra(ui, personas.0, "Personas", personas.1) {
                    destino = Some(Pantalla::Persona);
                }
                if cifra(ui, usuarios.0, "Usuarios", usuarios.1) {
                    destino = Some(Pantalla::Usuario);
                }
                if cifra(ui, perfiles.0, "Perfiles", perfiles.1) {
                    destino = Some(Pantalla::Perfil);
                }
            });
        });
        if let Some(pantalla) = destino {
            estado.mensaje = None;
            estado.ir_a(pantalla);
        }
    }

    entrada(ui, desde, 2, |ui| {
        ui.label(
            RichText::new("El precio mínimo de venta es el costo por kilo más el margen del productor.")
                .size(15.0)
                .color(SALVIA),
        );
    });

    if !estado.es_administrador() {
        ui.add_space(10.0);
        if enlace(ui, PINO, "Ver mis datos", true, 15.0) {
            estado.ir_a(Pantalla::Persona);
        }
    }
}
