//! Solo con la función `captura`: recorre las pantallas con datos de ejemplo en una base
//! temporal y guarda su propio cuadro. Sirve para revisar el diseño; no va en el ejecutable final.

use crate::auth::correo;
use crate::models::{Cual, DatosPersona};
use crate::ui::{AppState, Pantalla};
use eframe::egui;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const NOMBRES: [&str; 39] = [
    "00_apertura",
    "01_ingreso_base_vacia",
    "02_registro_paso1",
    "03_registro_cuenta",
    "04_ingreso_error",
    "05_recuperar",
    "06_panel",
    "07_personas",
    "08_persona_hoja",
    "09_persona_edicion_error",
    "10_usuario_nuevo",
    "11_perfiles",
    "12_mapa",
    "13_mapa_con_cambio",
    "14_confirmar",
    "15_correo",
    "16_panel_sin_productor",
    "17_mis_datos",
    "18_telon",
    "19_personas_antes",
    "20_hoja_entrando",
    "21_productores",
    "22_productor_nuevo_error",
    "23_granjas_filtradas",
    "24_granja_nueva_cascada",
    "25_corrales",
    "26_corral_ficha",
    "27_razas",
    "28_raza_ficha_imagen",
    "29_lotes",
    "30_lote_nuevo_corrales",
    "31_lote_ficha",
    "32_lote_cierre_rechazado",
    "33_mapa_completo",
    "34_panel_administrador",
    "35_productor_mis_granjas",
    "36_productor_razas",
    "37_consulta_guiada",
    "38_consulta_rechazada",
];

const PNG: &[u8] = include_bytes!("pruebas/raza_prueba.png");
const JPG: &[u8] = include_bytes!("pruebas/raza_prueba.jpg");

/// Espera antes de cada foto. Las cortas toman la animación a mitad de camino.
fn espera(paso: usize) -> u64 {
    match paso {
        0 => 550,
        1 => 2200,
        18 | 20 => 110,
        _ => 1500,
    }
}

enum Accion {
    Clic(f32, f32),
    Texto(&'static str),
    Tecla(egui::Key),
    Comprobar(&'static str, fn(&AppState) -> bool),
}

/// Prueba con eventos reales de ratón y teclado, en puntos de una ventana de 1366 por 800.
fn guion_de_eventos() -> Vec<Accion> {
    use egui::Key;
    use Accion::*;
    vec![
        Comprobar("abre en el ingreso", |e| e.pantalla == Pantalla::Acceso && e.sesion.is_none()),
        Clic(300.0, 250.0),
        Texto("admin.farming"),
        Comprobar("escribe en Usuario", |e| e.acceso.usuario == "admin.farming"),
        Tecla(Key::Tab),
        Texto("Potrero2026"),
        Comprobar("el tabulador pasa a Contraseña", |e| e.acceso.clave == "Potrero2026"),
        Tecla(Key::Enter),
        Comprobar("Enter ingresa", |e| e.pantalla == Pantalla::Panel),
        Clic(395.0, 43.0),
        Comprobar("la barra lleva a Usuarios", |e| e.pantalla == Pantalla::Usuario),
        Clic(240.0, 278.0),
        Comprobar("la fila abre la hoja", |e| e.usuario.hoja && e.usuario.id.is_some()),
        Clic(200.0, 600.0),
        Comprobar("pulsar fuera cierra la hoja", |e| !e.usuario.hoja),
        Clic(240.0, 278.0),
        Tecla(Key::Escape),
        Comprobar("Escape cierra la hoja", |e| !e.usuario.hoja),
        Clic(1246.0, 119.0),
        Comprobar("Crear abre la hoja vacía", |e| e.usuario.hoja && e.usuario.id.is_none()),
        Clic(1111.0, 470.0),
        Comprobar("Salir cierra la hoja", |e| !e.usuario.hoja),
        Clic(1298.0, 43.0),
        Comprobar("Salir de la barra cierra la sesión", |e| e.pantalla == Pantalla::Acceso && e.sesion.is_none()),
        Clic(148.0, 539.0),
        Comprobar("Registro abre el registro", |e| e.pantalla == Pantalla::Registro),
    ]
}

pub struct Guion {
    paso: usize,
    desde: Instant,
    pidio: bool,
    salida: PathBuf,
    eventos: std::collections::VecDeque<Accion>,
    fase_del_clic: u8,
    ultimo_evento: Instant,
    informe: String,
    verificar: bool,
}

fn persona(id: &str, nom1: &str, apell1: &str, correo: &str, movil: &str) -> DatosPersona {
    DatosPersona {
        identificacion: id.into(),
        nom1: nom1.into(),
        apell1: apell1.into(),
        direccion: "Corregimiento Banco de Arena".into(),
        movil: movil.into(),
        correo: correo.into(),
        fecha_nac: "1988-03-14".into(),
        ..Default::default()
    }
}

fn nueva_persona(e: &mut AppState, d: DatosPersona) {
    e.ir_a(Pantalla::Persona);
    e.persona_crear();
    e.persona.datos = d;
    e.persona_guardar();
}

fn nuevo_usuario(e: &mut AppState, id: i64, nombre: &str, perfil: i64) {
    e.ir_a(Pantalla::Usuario);
    e.usuario_crear();
    e.usuario.idpersona = Some(id);
    e.usuario.nombreu = nombre.into();
    e.usuario.contrasena = "Potrero2026".into();
    e.usuario.idperfil = Some(perfil);
    e.usuario_guardar();
}

fn nuevo_productor(e: &mut AppState, d: DatosPersona) {
    e.ir_a(Pantalla::Productor);
    e.productor_crear();
    e.productor.datos = d;
    e.productor_guardar();
}

fn llenar(e: &mut AppState, cual: Cual, datos: &[(&str, &str)]) {
    for (columna, valor) in datos {
        let i = cual.entidad().indice(columna);
        e.registro_mut(cual).valores[i] = (*valor).to_string();
    }
}

fn nuevo_registro(e: &mut AppState, cual: Cual, datos: &[(&str, &str)]) {
    e.ir_a(Pantalla::Gestion(cual));
    e.registro_crear(cual);
    llenar(e, cual, datos);
    e.registro_guardar(cual);
}

fn elegir_municipio(e: &mut AppState, departamento: &str, municipio: &str) {
    e.registro_mut(Cual::Granja).departamento = departamento.into();
    e.granja_cambio_departamento(Cual::Granja);
    let i = Cual::Granja.entidad().indice("idmunicipio");
    e.registro_mut(Cual::Granja).valores[i] = municipio.into();
}

fn nueva_granja(e: &mut AppState, datos: &[(&str, &str)], departamento: &str, municipio: &str) {
    e.ir_a(Pantalla::Gestion(Cual::Granja));
    e.registro_crear(Cual::Granja);
    llenar(e, Cual::Granja, datos);
    elegir_municipio(e, departamento, municipio);
    e.registro_guardar(Cual::Granja);
}

fn nuevo_lote(e: &mut AppState, datos: &[(&str, &str)], corrales: &[(i64, &str)]) {
    e.ir_a(Pantalla::Gestion(Cual::Lote));
    e.registro_crear(Cual::Lote);
    llenar(e, Cual::Lote, datos);
    e.lote_cargar_corrales();
    for (corral, cupos) in corrales {
        if let Some(fila) = e.registro_mut(Cual::Lote).corrales.iter_mut().find(|c| c.corral.codigo == *corral) {
            fila.elegido = true;
            fila.cupos = (*cupos).to_string();
        }
    }
    e.registro_guardar(Cual::Lote);
}

fn nueva_raza(e: &mut AppState, codigo: &str, descripcion: &str, imagen: Option<(&str, &[u8])>) {
    e.ir_a(Pantalla::Gestion(Cual::Raza));
    e.registro_crear(Cual::Raza);
    llenar(e, Cual::Raza, &[("codigor", codigo), ("descripcion", descripcion)]);
    if let Some((nombre, bytes)) = imagen {
        let ruta = e.carpeta.join(nombre);
        if fs::write(&ruta, bytes).is_ok() {
            e.raza_cargar_archivo(&ruta);
        }
    }
    e.registro_guardar(Cual::Raza);
}

/// Datos de ejemplo del Parcial II: tres productores, tres granjas, cinco corrales, cuatro razas y
/// tres lotes con sus corrales.
fn sembrar_porcinos(e: &mut AppState) {
    nuevo_productor(e, DatosPersona { direccion: "Vereda Agua Linda".into(), ..persona("1090000002", "Pedro", "Contreras", "pedro@ejemplo.com", "3102223344") });
    nuevo_productor(e, DatosPersona { direccion: "Vereda Puerto León".into(), ..persona("1090000003", "Luz Marina", "Duarte", "luz@ejemplo.com", "3203334455") });
    nuevo_productor(e, persona("88123456", "Ramiro", "Peñaranda", "", "6075712345"));
    nueva_granja(e, &[("codigo", "100"), ("idproductor", "1090000002"), ("nombre", "La Esperanza"), ("vereda", "El Pórtico"), ("telefono", "3102223344")], "54", "54001");
    nueva_granja(e, &[("codigo", "200"), ("idproductor", "1090000002"), ("nombre", "El Recreo"), ("vereda", "La Y")], "54", "54261");
    nueva_granja(e, &[("codigo", "300"), ("idproductor", "1090000003"), ("nombre", "Villa Luz"), ("vereda", "Puerto León"), ("correo", "villaluz@ejemplo.com")], "54", "54405");
    for (codigo, granja, nombre, capacidad) in
        [("11", "100", "Corral 1", "20"), ("12", "100", "Corral 2", "15"), ("13", "100", "Maternidad", "8"), ("21", "200", "Corral 1", "30"), ("31", "300", "Corral A", "25")]
    {
        nuevo_registro(e, Cual::Corral, &[("codigo", codigo), ("codigo_granja", granja), ("nombre", nombre), ("capacidad", capacidad)]);
    }
    nueva_raza(e, "1", "Landrace", Some(("landrace.png", PNG)));
    nueva_raza(e, "2", "Duroc", Some(("duroc.jpg", JPG)));
    nueva_raza(e, "3", "Pietrain", None);
    nueva_raza(e, "4", "Large White", Some(("large.png", PNG)));
    let lote = |codigo, granja, inicial, final_, cantidad| {
        [("codigol", codigo), ("codigo_granja", granja), ("fecha_inicial", inicial), ("fecha_final", final_), ("cantidad", cantidad), ("peso_objetivo", "110")]
    };
    nuevo_lote(e, &lote("1001", "100", "2026-09-01", "2027-01-10", "30"), &[(11, "20"), (12, "10")]);
    nuevo_lote(e, &lote("1002", "200", "2026-10-15", "2027-02-20", "24"), &[(21, "24")]);
    nuevo_lote(e, &lote("1003", "300", "2026-08-20", "2026-12-28", "28"), &[(31, "25")]);
    e.mensaje = None;
}

fn abrir_registro_de(e: &mut AppState, cual: Cual, id: i64) {
    e.ir_a(Pantalla::Gestion(cual));
    if let Some(r) = e.registro(cual).lista.iter().find(|r| r.id == id).cloned() {
        e.registro_mostrar(cual, &r);
    }
}

fn preparar(paso: usize, e: &mut AppState) {
    match paso {
        2 => {
            e.abrir_registro();
            e.persona.datos = persona("1090000001", "Carolina", "Mendoza", "carolina@ejemplo.com", "3001112233");
        }
        3 => {
            e.registro_siguiente();
            e.registro_siguiente();
            e.inicial.nombreu = "admin.farming".into();
            e.inicial.clave = "Potrero2026".into();
        }
        4 => {
            e.registro_siguiente();
            e.acceso.usuario = "pedro".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
        }
        5 => {
            e.acceso = Default::default();
            e.mensaje = None;
            e.alternar_recuperar();
            e.acceso.dato = "pedro@ejemplo.com".into();
        }
        6 => {
            e.acceso = Default::default();
            e.acceso.usuario = "admin.farming".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
            nueva_persona(e, persona("1090000004", "Jorge", "Ibáñez", "jorge@ejemplo.com", "3102223345"));
            nueva_persona(e, persona("1090000005", "Marta", "Suárez", "marta@ejemplo.com", "3203334456"));
            nuevo_usuario(e, 1_090_000_004, "jorge.ibanez", 2);
            e.ir_a(Pantalla::Perfil);
            e.perfil_crear();
            e.perfil.descripc = "Técnico agrícola".into();
            e.perfil_guardar();
            e.mensaje = None;
            e.ir_a(Pantalla::Panel);
        }
        7 => e.ir_a(Pantalla::Persona),
        8 => {
            e.persona.buscar = "1090000005".into();
            e.persona_consultar();
            e.mensaje = None;
        }
        9 => {
            e.persona_modificar();
            e.persona.datos.correo = "correo-malo".into();
            e.persona_guardar();
        }
        10 => {
            e.mensaje = None;
            e.ir_a(Pantalla::Usuario);
            e.usuario_crear();
            e.usuario.idpersona = Some(1_090_000_005);
            e.usuario.nombreu = "marta.suarez".into();
            e.usuario.idperfil = Some(2);
        }
        11 => {
            e.ir_a(Pantalla::Perfil);
            e.mensaje = None;
        }
        12 => e.ir_a(Pantalla::Mapa),
        13 => {
            e.ir_a(Pantalla::Persona);
            e.persona.buscar = "1090000004".into();
            e.persona_consultar();
            e.persona_modificar();
            e.persona.datos.direccion = "Vereda La Floresta".into();
            e.persona_guardar();
            e.ir_a(Pantalla::Mapa);
        }
        14 => {
            e.ir_a(Pantalla::Persona);
            e.persona.buscar = "1090000005".into();
            e.persona_consultar();
            e.mensaje = None;
            e.persona_pedir_inhabilitar();
        }
        15 => {
            e.confirmar = None;
            e.ir_a(Pantalla::Correo);
            e.correo.correo = "farming6@ejemplo.com".into();
            e.correo.configurada = correo::cargar(&e.carpeta).is_some();
        }
        16 => {
            e.cerrar_sesion();
            e.acceso.usuario = "jorge.ibanez".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
        }
        17 => e.ir_a(Pantalla::Persona),
        18 => {
            e.cerrar_sesion();
            e.acceso.usuario = "admin.farming".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
        }
        19 => e.ir_a(Pantalla::Persona),
        20 => {
            let primera = e.persona.lista[0].clone();
            e.persona_mostrar(&primera);
        }
        21 => {
            sembrar_porcinos(e);
            e.ir_a(Pantalla::Productor);
        }
        22 => {
            e.productor_crear();
            e.productor.datos = DatosPersona { fecha_nac: "2012-05-01".into(), ..persona("77123456", "Lucía", "Vera", "", "3157778899") };
            e.productor_guardar();
        }
        23 => {
            e.mensaje = None;
            e.ir_a(Pantalla::Gestion(Cual::Granja));
            e.registro_mut(Cual::Granja).filtros[0] = "54".into();
            e.registro_cambiar_filtro(Cual::Granja, 0);
        }
        24 => {
            e.registro_crear(Cual::Granja);
            llenar(e, Cual::Granja, &[("codigo", "400"), ("idproductor", "88123456"), ("nombre", "Los Naranjos")]);
            elegir_municipio(e, "54", "54405");
        }
        25 => {
            e.ir_a(Pantalla::Gestion(Cual::Corral));
            e.registro_mut(Cual::Corral).filtros[0] = "100".into();
        }
        26 => abrir_registro_de(e, Cual::Corral, 11),
        27 => e.ir_a(Pantalla::Gestion(Cual::Raza)),
        28 => abrir_registro_de(e, Cual::Raza, 1),
        29 => e.ir_a(Pantalla::Gestion(Cual::Lote)),
        30 => {
            e.registro_crear(Cual::Lote);
            llenar(e, Cual::Lote, &[("codigol", "1004"), ("codigo_granja", "100"), ("fecha_final", "2027-03-15"), ("cantidad", "18"), ("peso_objetivo", "110")]);
            e.lote_cargar_corrales();
            if let Some(fila) = e.registro_mut(Cual::Lote).corrales.iter_mut().find(|c| c.corral.codigo == 13) {
                fila.elegido = true;
                fila.cupos = "8".into();
            }
        }
        31 => abrir_registro_de(e, Cual::Lote, 1001),
        32 => {
            e.lote_pedir_cierre();
            e.lote_cerrar();
        }
        33 => {
            e.mensaje = None;
            e.ir_a(Pantalla::Mapa);
            e.tabla_del_mapa = 10;
        }
        34 => e.ir_a(Pantalla::Panel),
        35 => {
            nuevo_usuario(e, 1_090_000_002, "pedro.contreras", 2);
            e.cerrar_sesion();
            e.acceso.usuario = "pedro.contreras".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
            e.telon = None;
            e.ir_a(Pantalla::Gestion(Cual::Granja));
        }
        36 => e.ir_a(Pantalla::Gestion(Cual::Raza)),
        37 => {
            e.cerrar_sesion();
            e.acceso.usuario = "admin.farming".into();
            e.acceso.clave = "Potrero2026".into();
            e.ingresar();
            e.telon = None;
            e.mensaje = None;
            e.ir_a(Pantalla::Mapa);
            e.consulta_guiada(2);
        }
        38 => {
            e.consulta.sql = "DELETE FROM corral WHERE codigo = 11".into();
            e.consulta_ejecutar();
        }
        _ => {}
    }
}

impl Guion {
    pub fn nuevo() -> Self {
        let salida = std::env::var_os("FARMING6_FOTOS").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        Self {
            paso: 0,
            desde: Instant::now(),
            pidio: false,
            salida,
            eventos: guion_de_eventos().into(),
            fase_del_clic: 0,
            ultimo_evento: Instant::now(),
            informe: String::new(),
            verificar: false,
        }
    }

    /// Entrega a egui los eventos de la prueba como si vinieran de Windows.
    pub fn inyectar(&mut self, entrada: &mut egui::RawInput) {
        entrada.focused = true;
        if !self.verificar || self.ultimo_evento.elapsed() < Duration::from_millis(320) {
            return;
        }
        let modificadores = egui::Modifiers::NONE;
        match self.eventos.front() {
            Some(Accion::Clic(x, y)) => {
                let pos = egui::pos2(*x, *y);
                let boton = |presionado| egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: presionado,
                    modifiers: modificadores,
                };
                match self.fase_del_clic {
                    0 => entrada.events.push(egui::Event::PointerMoved(pos)),
                    1 => entrada.events.push(boton(true)),
                    _ => {
                        entrada.events.push(boton(false));
                        self.eventos.pop_front();
                    }
                }
                self.fase_del_clic = (self.fase_del_clic + 1) % 3;
            }
            Some(Accion::Texto(texto)) => {
                entrada.events.push(egui::Event::Text((*texto).to_string()));
                self.eventos.pop_front();
            }
            Some(Accion::Tecla(tecla)) => {
                for presionada in [true, false] {
                    entrada.events.push(egui::Event::Key {
                        key: *tecla,
                        physical_key: None,
                        pressed: presionada,
                        repeat: false,
                        modifiers: modificadores,
                    });
                }
                self.eventos.pop_front();
            }
            _ => return,
        }
        self.ultimo_evento = Instant::now();
    }

    fn comprobar(&mut self, contexto: &egui::Context, estado: &AppState) {
        if self.ultimo_evento.elapsed() < Duration::from_millis(320) {
            return;
        }
        while let Some(Accion::Comprobar(nombre, prueba)) = self.eventos.front() {
            let bien = prueba(estado);
            self.informe.push_str(&format!("{} {nombre}\n", if bien { "BIEN" } else { "FALLA" }));
            self.eventos.pop_front();
        }
        if self.eventos.is_empty() {
            let _ = fs::write(self.salida.join("eventos.txt"), &self.informe);
            contexto.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    pub fn avanzar(&mut self, contexto: &egui::Context, estado: &mut AppState) {
        contexto.request_repaint_after(Duration::from_millis(30));
        if self.verificar {
            return self.comprobar(contexto, estado);
        }
        let imagen = contexto.input(|i| {
            i.events.iter().find_map(|evento| match evento {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(imagen) = imagen {
            let mut bytes = format!("{} {}\n", imagen.size[0], imagen.size[1]).into_bytes();
            bytes.extend_from_slice(imagen.as_raw());
            let _ = fs::write(self.salida.join(format!("{}.rgba", NOMBRES[self.paso])), bytes);
            self.paso += 1;
            self.pidio = false;
            self.desde = Instant::now();
            if self.paso >= NOMBRES.len() {
                estado.cerrar_sesion();
                self.verificar = true;
                self.ultimo_evento = Instant::now();
            } else {
                preparar(self.paso, estado);
            }
            return;
        }
        let lista = Duration::from_millis(espera(self.paso));
        if !self.pidio && self.paso < NOMBRES.len() && self.desde.elapsed() > lista {
            self.pidio = true;
            contexto.send_viewport_cmd(egui::ViewportCommand::Screenshot);
        }
    }
}
