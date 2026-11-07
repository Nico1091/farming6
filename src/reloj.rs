//! Fecha y hora de Colombia (UTC-5, sin horario de verano) con la biblioteca estándar.

use std::time::{SystemTime, UNIX_EPOCH};

const DESFASE_COLOMBIA: i64 = -5 * 3600;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Fecha {
    pub anio: i32,
    pub mes: u32,
    pub dia: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct Momento {
    pub fecha: Fecha,
    pub hora: u32,
    pub minuto: u32,
    pub segundo: u32,
    /// 0 = domingo.
    pub dia_semana: usize,
}

impl Momento {
    pub fn hora_texto(&self) -> String {
        format!("{:02}:{:02}:{:02}", self.hora, self.minuto, self.segundo)
    }
}

pub fn ahora() -> Momento {
    let segundos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
        + DESFASE_COLOMBIA;
    let dias = segundos.div_euclid(86_400);
    let resto = segundos.rem_euclid(86_400);
    Momento {
        fecha: fecha_desde_dias(dias),
        hora: (resto / 3600) as u32,
        minuto: (resto % 3600 / 60) as u32,
        segundo: (resto % 60) as u32,
        dia_semana: (dias + 4).rem_euclid(7) as usize,
    }
}

/// Convierte días desde 1970-01-01 a fecha civil (algoritmo de Howard Hinnant).
fn fecha_desde_dias(dias: i64) -> Fecha {
    let z = dias + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let dia = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let mes = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    let anio = yoe + era * 400 + i64::from(mes <= 2);
    Fecha { anio: anio as i32, mes, dia }
}

pub fn es_bisiesto(anio: i32) -> bool {
    (anio % 4 == 0 && anio % 100 != 0) || anio % 400 == 0
}

pub fn dias_del_mes(anio: i32, mes: u32) -> u32 {
    match mes {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if es_bisiesto(anio) => 29,
        2 => 28,
        _ => 0,
    }
}

#[cfg(test)]
mod pruebas {
    use super::*;

    #[test]
    fn convierte_fechas_conocidas() {
        assert_eq!(fecha_desde_dias(0), Fecha { anio: 1970, mes: 1, dia: 1 });
        assert_eq!(fecha_desde_dias(20_454), Fecha { anio: 2026, mes: 1, dia: 1 });
        assert_eq!(fecha_desde_dias(19_782), Fecha { anio: 2024, mes: 2, dia: 29 });
    }

    #[test]
    fn dias_del_mes_respeta_bisiestos() {
        assert_eq!(dias_del_mes(2024, 2), 29);
        assert_eq!(dias_del_mes(2026, 2), 28);
        assert_eq!(dias_del_mes(1900, 2), 28);
        assert_eq!(dias_del_mes(2000, 2), 29);
    }
}
