//! Sección Consultar de la pantalla Base de datos: la base se puede preguntar desde el programa.
//! Las consultas guiadas cubren HU-01 a HU-05 con un clic; el cuadro admite cualquier consulta
//! de lectura en SQL. Las reglas de seguridad están en `db::consulta`.

use super::comun::*;
use super::theme::*;
use super::AppState;
use crate::db::consulta::{consultar, ResultadoConsulta, GUIADAS, MAXIMO_FILAS};
use crate::db::vigia::{Columna, Foto, TablaViva};
use eframe::egui::{self, pos2, vec2, CursorIcon, FontId, Key, Margin, Modifiers, RichText, Sense, Stroke, TextEdit, Ui};

#[derive(Default)]
pub struct FormConsulta {
    pub sql: String,
    /// Consulta guiada que está en el cuadro, mientras no se edite.
    pub guiada: Option<usize>,
    pub resultado: Option<Result<ResultadoConsulta, String>>,
    /// El resultado como tabla para la rejilla; se arma una vez por consulta.
    pub tabla: Option<TablaViva>,
}

fn tabla_de(resultado: &ResultadoConsulta) -> TablaViva {
    let columna = |nombre: &String| Columna { nombre: nombre.clone(), tipo: String::new(), llave_primaria: false, obligatoria: false, foranea: None };
    TablaViva {
        nombre: "consulta".into(),
        columnas: resultado.columnas.iter().map(columna).collect(),
        filas: resultado.filas.clone(),
        claves: Vec::new(),
        activos: 0,
        inhabilitados: 0,
    }
}

impl AppState {
    /// Pone en el cuadro una consulta guiada y la corre.
    pub fn consulta_guiada(&mut self, indice: usize) {
        if let Some(guiada) = GUIADAS.get(indice) {
            self.consulta.sql = guiada.sql.to_string();
            self.consulta.guiada = Some(indice);
            self.consulta_ejecutar();
        }
    }

    pub fn consulta_ejecutar(&mut self) {
        if !self.es_administrador() {
            return self.error("Su perfil no tiene permiso para consultar la base de datos.");
        }
        if self.consulta.guiada.is_some_and(|i| GUIADAS[i].sql != self.consulta.sql) {
            self.consulta.guiada = None;
        }
        let resultado = consultar(&self.db.ruta, &self.consulta.sql);
        self.consulta.tabla = resultado.as_ref().ok().filter(|r| !r.columnas.is_empty()).map(tabla_de);
        self.consulta.resultado = Some(resultado);
    }

    pub fn consulta_limpiar(&mut self) {
        self.consulta = FormConsulta::default();
    }
}

fn plural(n: usize, singular: &str, plural: &str) -> String {
    format!("{n} {}", if n == 1 { singular } else { plural })
}

fn guiadas(ui: &mut Ui, elegida: Option<usize>) -> Option<usize> {
    let mut pedida = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 22.0;
        for (i, guiada) in GUIADAS.iter().enumerate() {
            let marcada = elegida == Some(i);
            let color = if marcada { PINO } else { SALVIA };
            let g = ui.painter().layout_no_wrap(guiada.titulo.to_string(), FontId::proportional(15.0), color);
            let (rect, respuesta) = ui.allocate_exact_size(g.size() + vec2(0.0, 8.0), Sense::click());
            ui.painter().galley(rect.left_top(), g, color);
            if marcada {
                let y = rect.bottom() - 1.0;
                ui.painter().line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.6_f32, MAIZ));
            }
            if respuesta.on_hover_cursor(CursorIcon::PointingHand).clicked() {
                pedida = Some(i);
            }
        }
    });
    pedida
}

fn linea_de_resultado(ui: &mut Ui, resultado: &Result<ResultadoConsulta, String>) {
    match resultado {
        Ok(r) => {
            let mut texto = format!("{} en {} ms", plural(r.filas.len(), "fila", "filas"), r.milisegundos);
            if r.recortada {
                texto.push_str(&format!(". Hay más: se muestran las primeras {MAXIMO_FILAS}"));
            }
            ui.label(RichText::new(texto).size(13.5).color(SALVIA));
        }
        Err(error) => {
            ui.label(RichText::new(error).size(14.0).color(ARCILLA));
        }
    }
}

/// La sección completa: consultas guiadas, cuadro de SQL, acciones y resultado.
pub fn seccion(ui: &mut Ui, estado: &mut AppState, foto: &Foto) {
    ui.label(RichText::new("Consultar").font(semi(20.0)).color(PINO));
    ui.add_space(2.0);
    ui.label(
        RichText::new("Elija una consulta guiada o escriba la suya en SQL. Desde aquí solo se lee: los datos se cambian en los formularios.")
            .size(13.0)
            .color(SALVIA),
    );
    ui.add_space(10.0);
    let pedida = guiadas(ui, estado.consulta.guiada);
    ui.add_space(8.0);

    let id = egui::Id::new("consulta_sql");
    // Ctrl+Enter corre la consulta; se toma antes de que el cuadro escriba el salto de línea.
    let atajo = ui.memory(|m| m.has_focus(id)) && ui.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Enter));
    let cuadro = ui.add(
        TextEdit::multiline(&mut estado.consulta.sql)
            .id(id)
            .code_editor()
            .frame(false)
            .text_color(PINO)
            .hint_text(RichText::new("SELECT nombre, capacidad FROM corral WHERE estado = 'A'").color(SALVIA))
            .desired_rows(5)
            .desired_width(f32::INFINITY)
            .margin(Margin::symmetric(0.0, 8.0)),
    );
    // La misma línea inferior de los demás campos, que se marca al escribir.
    let foco = ui.ctx().animate_bool_with_time(id.with("foco"), cuadro.has_focus(), duracion(0.14));
    let (izquierda, derecha, y) = (cuadro.rect.left(), cuadro.rect.right(), cuadro.rect.bottom() + 2.0);
    ui.painter().line_segment([pos2(izquierda, y), pos2(derecha, y)], Stroke::new(1.0_f32, LINEA));
    if foco > 0.0 {
        let fin = egui::lerp(izquierda..=derecha, foco);
        ui.painter().line_segment([pos2(izquierda, y), pos2(fin, y)], Stroke::new(1.6_f32, PINO));
    }
    ui.add_space(14.0);

    let mut correr = atajo;
    let mut limpiar = false;
    ui.horizontal(|ui| {
        correr |= boton(ui, SOBRE_NIEBLA, "Consultar", true);
        ui.add_space(18.0);
        limpiar = enlace(ui, PINO, "Limpiar", true, 15.0);
        ui.add_space(18.0);
        ui.label(RichText::new("Ctrl + Enter también consulta").size(12.5).color(SALVIA));
    });

    if let Some(i) = pedida {
        estado.consulta_guiada(i);
    } else if correr {
        estado.consulta_ejecutar();
    } else if limpiar {
        estado.consulta_limpiar();
    }

    if let Some(resultado) = &estado.consulta.resultado {
        ui.add_space(10.0);
        linea_de_resultado(ui, resultado);
    }
    if let Some(tabla) = &estado.consulta.tabla {
        ui.add_space(6.0);
        super::mapa::rejilla(ui, tabla, foto);
    }
}
