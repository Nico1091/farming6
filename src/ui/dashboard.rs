//! Panel: un saludo, las cifras reales de la base y la franja de cordilleras.

use super::comun::*;
use super::theme::*;
use super::{paisaje, AppState, Pantalla};
use crate::models::Cual;
use eframe::egui::{lerp, pos2, vec2, Align2, CursorIcon, FontId, Rect, RichText, Sense, Stroke, Ui};

fn cifra(ui: &mut Ui, valor: usize, etiqueta: &str, inhabilitados: usize) -> bool {
    let (rect, respuesta) = ui.allocate_exact_size(vec2(132.0, 136.0), Sense::click());
    let pintor = ui.painter();
    pintor.text(rect.left_top(), Align2::LEFT_TOP, valor.to_string(), liviana(60.0), PINO);
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
    ui.add_space(22.0);

    if estado.es_administrador() {
        let foto = estado.foto_actual();
        let dato_de = |nombre: &str| foto.tabla(nombre).map(|t| (t.activos, t.inhabilitados)).unwrap_or((0, 0));
        let cifras: [(&str, &str, Pantalla); 8] = [
            ("persona", "Personas", Pantalla::Persona),
            ("usuario", "Usuarios", Pantalla::Usuario),
            ("perfil", "Perfiles", Pantalla::Perfil),
            ("productor", "Productores", Pantalla::Productor),
            ("granja", "Granjas", Pantalla::Gestion(Cual::Granja)),
            ("corral", "Corrales", Pantalla::Gestion(Cual::Corral)),
            ("raza", "Razas", Pantalla::Gestion(Cual::Raza)),
            ("lote", "Lotes", Pantalla::Gestion(Cual::Lote)),
        ];
        let mut destino = None;
        entrada(ui, desde, 1, |ui| {
            ui.horizontal_wrapped(|ui| {
                for (tabla, etiqueta, pantalla) in cifras {
                    let (activos, inhabilitados) = dato_de(tabla);
                    if cifra(ui, activos, etiqueta, inhabilitados) {
                        destino = Some(pantalla);
                    }
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
            RichText::new("Farming 6 lleva la producción porcina: granjas, corrales, razas y lotes en ceba, para fijar el precio de venta del cerdo en pie.")
                .size(15.0)
                .color(SALVIA),
        );
    });

    if !estado.es_administrador() {
        ui.add_space(10.0);
        let mut destino = None;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 26.0;
            if enlace(ui, PINO, "Ver mis datos", true, 15.0) {
                destino = Some(Pantalla::Persona);
            }
            if estado.es_productor() {
                for (texto, cual) in [("Mis granjas", Cual::Granja), ("Corrales", Cual::Corral), ("Lotes", Cual::Lote), ("Razas", Cual::Raza)] {
                    if enlace(ui, PINO, texto, true, 15.0) {
                        destino = Some(Pantalla::Gestion(cual));
                    }
                }
            }
        });
        if let Some(pantalla) = destino {
            estado.ir_a(pantalla);
        }
    }
}
