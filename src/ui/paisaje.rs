//! Cordilleras en capas con un sol que sale al abrir. Es el único adorno de la aplicación.

use super::theme::*;
use eframe::egui::emath::easing;
use eframe::egui::{pos2, Color32, Mesh, Painter, Rect, Shape};

pub struct Tonos {
    pub cielo: Option<(Color32, Color32)>,
    pub capas: [(Color32, Color32); 4],
}

pub const AMANECER: Tonos = Tonos {
    cielo: Some((PINO, Color32::from_rgb(36, 90, 76))),
    capas: [
        (Color32::from_rgb(58, 112, 96), Color32::from_rgb(34, 82, 70)),
        (Color32::from_rgb(40, 92, 78), Color32::from_rgb(26, 68, 57)),
        (Color32::from_rgb(28, 74, 62), Color32::from_rgb(20, 58, 48)),
        (Color32::from_rgb(20, 58, 48), PINO),
    ],
};

pub const DIA: Tonos = Tonos {
    cielo: None,
    capas: [
        (Color32::from_rgb(232, 237, 230), NIEBLA),
        (Color32::from_rgb(220, 229, 219), Color32::from_rgb(233, 238, 231)),
        (Color32::from_rgb(205, 218, 206), Color32::from_rgb(222, 231, 222)),
        (Color32::from_rgb(186, 204, 190), Color32::from_rgb(208, 221, 209)),
    ],
};

fn cielo(pintor: &Painter, r: Rect, arriba: Color32, abajo: Color32) {
    let mut malla = Mesh::default();
    malla.colored_vertex(r.left_top(), arriba);
    malla.colored_vertex(r.right_top(), arriba);
    malla.colored_vertex(r.left_bottom(), abajo);
    malla.colored_vertex(r.right_bottom(), abajo);
    malla.add_triangle(0, 1, 2);
    malla.add_triangle(1, 2, 3);
    pintor.add(Shape::mesh(malla));
}

/// Una cordillera: sube hacia la derecha para dejar calma la zona del formulario.
fn altura(capa: usize, t: f32, deriva: f32) -> f32 {
    let (base, subida, ondas) = [
        (0.10, 0.62, (6.0, 0.022, 13.0, 0.012)),
        (0.075, 0.44, (5.0, 0.020, 11.0, 0.010)),
        (0.05, 0.28, (4.0, 0.016, 9.0, 0.008)),
        (0.03, 0.15, (3.0, 0.012, 8.0, 0.006)),
    ][capa];
    let fase = capa as f32 * 1.7 + deriva;
    base + subida * t.powf(1.6) + ondas.1 * (t * ondas.0 + fase).sin() + ondas.3 * (t * ondas.2 + fase * 0.6).sin()
}

fn cordillera(pintor: &Painter, r: Rect, capa: usize, tonos: (Color32, Color32), elevacion: f32, deriva: f32) {
    const PASOS: u32 = 120;
    let mut malla = Mesh::default();
    for i in 0..=PASOS {
        let t = i as f32 / PASOS as f32;
        let x = r.left() + t * r.width();
        let cima = r.bottom() - altura(capa, t, deriva) * r.height() * elevacion;
        malla.colored_vertex(pos2(x, cima), tonos.0);
        malla.colored_vertex(pos2(x, r.bottom()), tonos.1);
        if i < PASOS {
            let n = i * 2;
            malla.add_triangle(n, n + 1, n + 2);
            malla.add_triangle(n + 1, n + 2, n + 3);
        }
    }
    pintor.add(Shape::mesh(malla));
}

/// `segundos` es el tiempo desde la apertura; `tiempo` hace derivar muy despacio las capas.
pub fn pintar(pintor: &Painter, r: Rect, tonos: &Tonos, segundos: f32, tiempo: f32, sol: f32) {
    let pintor = pintor.with_clip_rect(r);
    if let Some((arriba, abajo)) = tonos.cielo {
        cielo(&pintor, r, arriba, abajo);
    }
    let avance = |retraso: f32, largo: f32| easing::cubic_out(((segundos - retraso) / largo).clamp(0.0, 1.0));

    if sol > 0.0 {
        let subida = easing::cubic_out(((segundos - 0.3) / 1.6).clamp(0.0, 1.0));
        let x = r.left() + r.width() * 0.76;
        let inicio = r.bottom() - r.height() * 0.42;
        let fin = r.top() + r.height() * 0.40;
        let centro = pos2(x, inicio + (fin - inicio) * subida);
        pintor.circle_filled(centro, sol, MAIZ);
    }
    for capa in 0..4 {
        let elevacion = avance(capa as f32 * 0.12, 1.1);
        cordillera(&pintor, r, capa, tonos.capas[capa], elevacion, tiempo * (0.012 + capa as f32 * 0.006));
    }
}
