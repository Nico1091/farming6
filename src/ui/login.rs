//! Ingreso, recuperación y registro: pantalla completa, sin cajas, sobre las cordilleras.

use super::comun::*;
use super::persona::{campos_contacto, campos_identidad};
use super::theme::*;
use super::{paisaje, AppState, Pantalla, Sesion};
use crate::auth::{self, correo};
use crate::models::{DatosPersona, Usuario, ACTIVO};
use crate::{reloj, validaciones};
use eframe::egui::emath::easing;
use eframe::egui::{self, pos2, vec2, Key, Order, Rect, RichText, UiBuilder};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread;
use std::time::{Duration, Instant};

const P: Paleta = SOBRE_PINO;

pub struct EnvioEnCurso {
    idusuario: i64,
    hash: String,
    resultado: Receiver<Result<(), String>>,
}

#[derive(Default)]
pub struct FormAcceso {
    pub usuario: String,
    pub clave: String,
    pub ver_clave: bool,
    pub recuperando: bool,
    pub dato: String,
    pub envio: Option<EnvioEnCurso>,
}

#[derive(Default)]
pub struct FormInicial {
    pub datos: DatosPersona,
    pub nombreu: String,
    pub clave: String,
    pub ver_clave: bool,
    pub paso: usize,
}

impl AppState {
    /// Botón Ingresar.
    pub fn ingresar(&mut self) {
        let nombre = self.acceso.usuario.trim().to_string();
        if nombre.is_empty() {
            return self.error("Debe escribir el usuario.");
        }
        if self.acceso.clave.is_empty() {
            return self.error("Debe escribir la contraseña.");
        }
        let usuario = match self.db.usuario_por_nombre(&nombre) {
            Ok(Some(u)) => u,
            Ok(None) => return self.error("El usuario no se encuentra registrado"),
            Err(e) => return self.error(e),
        };
        if !auth::verificar(&self.acceso.clave, &usuario.contrasena) {
            return self.error("La contraseña no coincide con la registrada.");
        }
        let persona = match self.db.persona_por_id(usuario.idusuario) {
            Ok(Some(p)) => p,
            Ok(None) => return self.error("El usuario no tiene una persona asociada."),
            Err(e) => return self.error(e),
        };
        let perfil = match self.db.perfil_por_id(usuario.idperfil) {
            Ok(Some(p)) => p,
            Ok(None) => return self.error("El usuario no tiene un perfil asociado."),
            Err(e) => return self.error(e),
        };
        if usuario.estado != ACTIVO || persona.estado != ACTIVO || perfil.estado != ACTIVO {
            return self.error("Su usuario está inhabilitado. Comuníquese con el administrador.");
        }

        let administrador = perfil.descripc.eq_ignore_ascii_case("Administrador");
        self.sesion = Some(Sesion { usuario, persona, perfil: perfil.descripc, administrador });
        self.acceso = Default::default();
        self.mensaje = None;
        self.pantalla = Pantalla::Panel;
        self.telon = Some(Instant::now());
        self.cambio_de_vista = Instant::now();
    }

    /// Botón Cancelar: limpia los campos del formulario.
    pub fn cancelar_acceso(&mut self) {
        self.acceso = Default::default();
        self.mensaje = None;
    }

    pub fn alternar_recuperar(&mut self) {
        self.acceso.recuperando = !self.acceso.recuperando;
        self.mensaje = None;
        self.cambio_de_formulario = Instant::now();
    }

    /// Envía el usuario y una contraseña temporal al correo registrado. La contraseña solo
    /// se cambia en la base cuando el servidor confirma que recibió el mensaje.
    pub fn recuperar(&mut self) {
        if self.acceso.envio.is_some() {
            return;
        }
        let dato = self.acceso.dato.trim().to_string();
        if dato.is_empty() {
            return self.error("Digite su identificación o su correo electrónico.");
        }
        let persona = if dato.chars().all(|c| c.is_ascii_digit()) {
            match dato.parse() {
                Ok(id) => self.db.persona_por_id(id),
                Err(_) => Ok(None),
            }
        } else if let Err(e) = validaciones::correo(&dato) {
            return self.error(e);
        } else {
            self.db.persona_por_correo(&dato.to_lowercase())
        };
        let persona = match persona {
            Ok(Some(p)) => p,
            Ok(None) => return self.error("El usuario no se encuentra registrado"),
            Err(e) => return self.error(e),
        };
        let usuario = match self.db.usuario_por_id(persona.idpersona) {
            Ok(Some(u)) => u,
            Ok(None) => return self.error("El usuario no se encuentra registrado"),
            Err(e) => return self.error(e),
        };
        if usuario.estado != ACTIVO || persona.estado != ACTIVO {
            return self.error("Su usuario está inhabilitado. Comuníquese con el administrador.");
        }
        let Some(cuenta) = correo::cargar(&self.carpeta) else {
            return self.error("El correo del sistema no está configurado. Debe hacerlo el administrador.");
        };

        let temporal = auth::clave_temporal();
        let hash = match auth::cifrar(&temporal) {
            Ok(h) => h,
            Err(e) => return self.error(e),
        };
        let mensaje = correo::mensaje_recuperacion(
            &cuenta.correo,
            &persona.correo,
            &persona.nombre_completo(),
            &usuario.nombreu,
            &temporal,
        );
        let servidor = self.servidor_correo.clone();
        let (emisor, receptor) = mpsc::channel();
        thread::spawn(move || {
            let resultado = correo::enviar(&servidor, Some(&cuenta), &cuenta.correo, &persona.correo, &mensaje);
            let _ = emisor.send(resultado);
        });
        self.acceso.envio = Some(EnvioEnCurso { idusuario: usuario.idusuario, hash, resultado: receptor });
    }

    /// Revisa si terminó el envío en curso. Se llama en cada cuadro.
    pub fn revisar_envio(&mut self) {
        let Some(envio) = &self.acceso.envio else {
            return;
        };
        let resultado = match envio.resultado.try_recv() {
            Ok(r) => r,
            Err(TryRecvError::Empty) => return,
            Err(TryRecvError::Disconnected) => Err("El envío se interrumpió.".to_string()),
        };
        let envio = self.acceso.envio.take().expect("el envío existe");
        let resultado = resultado.and_then(|()| self.db.cambiar_contrasena(envio.idusuario, &envio.hash));
        match resultado {
            Ok(()) => {
                self.acceso.dato.clear();
                self.acceso.recuperando = false;
                self.cambio_de_formulario = Instant::now();
                self.exito("Su usuario y contraseña fue enviado al correo registrado");
            }
            Err(e) => self.error(e),
        }
    }

    /// Sin usuarios en la base, quien se registra primero queda como administrador.
    pub fn es_primer_registro(&self) -> bool {
        self.db.total_usuarios().map(|n| n == 0).unwrap_or(false)
    }

    /// Crea la primera persona y su usuario Administrador cuando la base está vacía.
    pub fn crear_administrador_inicial(&mut self) {
        let hoy = reloj::ahora().fecha;
        let id = match validaciones::persona(&self.inicial.datos, hoy) {
            Ok(id) => id,
            Err(e) => return self.error(e),
        };
        if let Err(e) = validaciones::nombre_usuario(&self.inicial.nombreu) {
            return self.error(e);
        }
        if let Err(e) = validaciones::contrasena(&self.inicial.clave) {
            return self.error(e);
        }
        let perfil = match self.db.perfil_por_descripcion("Administrador") {
            Ok(Some(p)) => p,
            Ok(None) => return self.error("No existe el perfil Administrador."),
            Err(e) => return self.error(e),
        };
        let hash = match auth::cifrar(&self.inicial.clave) {
            Ok(h) => h,
            Err(e) => return self.error(e),
        };
        let persona = self.inicial.datos.a_persona(id, ACTIVO);
        let usuario = Usuario {
            idusuario: id,
            nombreu: self.inicial.nombreu.trim().to_string(),
            contrasena: hash,
            idperfil: perfil.idperfil,
            estado: ACTIVO.to_string(),
        };
        match self.db.crear_persona_con_usuario(&persona, &usuario) {
            Ok(()) => {
                self.inicial = Default::default();
                self.pantalla = Pantalla::Acceso;
                self.cambio_de_formulario = Instant::now();
                self.exito("Cuenta creada. Ya puede ingresar.");
            }
            Err(e) => self.error(e),
        }
    }

    pub fn abrir_registro(&mut self) {
        self.mensaje = None;
        self.inicial = Default::default();
        self.persona_preparar_registro();
        self.cambio_de_formulario = Instant::now();
    }

    pub fn registro_atras(&mut self) {
        self.mensaje = None;
        self.cambio_de_formulario = Instant::now();
        if self.inicial.paso == 0 {
            self.pantalla = Pantalla::Acceso;
        } else {
            self.inicial.paso -= 1;
        }
    }

    /// Valida el paso actual y avanza, o guarda en el último.
    pub fn registro_siguiente(&mut self) {
        let hoy = reloj::ahora().fecha;
        let primero = self.es_primer_registro();
        match self.inicial.paso {
            0 => match validaciones::identidad(&self.persona.datos, hoy) {
                Ok(id) if matches!(self.db.persona_por_id(id), Ok(Some(_))) => {
                    return self.error("Esa identificación ya está registrada.");
                }
                Ok(_) => {}
                Err(e) => return self.error(e),
            },
            1 => {
                if let Err(e) = validaciones::contacto(&self.persona.datos) {
                    return self.error(e);
                }
            }
            _ => {}
        }
        self.mensaje = None;
        let ultimo = if primero { 2 } else { 1 };
        if self.inicial.paso < ultimo {
            self.inicial.paso += 1;
            self.cambio_de_formulario = Instant::now();
        } else if primero {
            self.inicial.datos = self.persona.datos.clone();
            self.crear_administrador_inicial();
            if self.pantalla == Pantalla::Acceso {
                self.persona = Default::default();
            }
        } else {
            self.persona_guardar_registro();
            self.cambio_de_formulario = Instant::now();
        }
    }
}

fn fondo(ui: &egui::Ui, estado: &AppState) {
    let segundos = estado.apertura.elapsed().as_secs_f32();
    let tiempo = ui.input(|i| i.time) as f32;
    let tiempo = if animaciones() { tiempo } else { 0.0 };
    let segundos = if animaciones() { segundos } else { 10.0 };
    paisaje::pintar(ui.painter(), ui.max_rect(), &paisaje::AMANECER, segundos, tiempo, 54.0);
    if animaciones() {
        let espera = if segundos < 2.5 { 0 } else { 33 };
        ui.ctx().request_repaint_after(Duration::from_millis(espera));
    }
}

fn columna(contexto: &egui::Context, estado: &mut AppState, contenido: impl FnOnce(&mut egui::Ui, &mut AppState)) {
    egui::CentralPanel::default().frame(egui::Frame::none().fill(PINO)).show(contexto, |ui| {
        fondo(ui, estado);
        let zona = ui.max_rect();
        let izquierda = zona.left() + (zona.width() * 0.09).max(56.0);
        let rect = Rect::from_min_max(pos2(izquierda, zona.top()), pos2(izquierda + 372.0, zona.bottom()));
        ui.allocate_new_ui(UiBuilder::new().max_rect(rect), |ui| {
            ui.visuals_mut().override_text_color = Some(P.texto);
            ui.add_space(((zona.height() - 620.0) / 2.0).max(32.0));
            contenido(ui, estado);
        });
    });
}

fn marca(ui: &mut egui::Ui, desde: Instant) {
    entrada(ui, desde, 0, |ui| {
        ui.label(RichText::new("Farming 6").font(liviana(58.0)).color(P.texto));
        ui.add_space(26.0);
    });
}

pub fn acceso(contexto: &egui::Context, estado: &mut AppState) {
    if estado.acceso.recuperando {
        return recuperar(contexto, estado);
    }
    columna(contexto, estado, |ui, estado| {
        let d = estado.cambio_de_formulario;
        marca(ui, d);
        let usuario = entrada(ui, d, 1, |ui| campo(ui, P, "Usuario", &mut estado.acceso.usuario, true));
        let sin_foco = ui.ctx().memory(|m| m.focused().is_none());
        if sin_foco && d.elapsed() < Duration::from_millis(200) {
            usuario.request_focus();
        }
        let form = &mut estado.acceso;
        let clave = entrada(ui, d, 2, |ui| campo_clave(ui, P, "Contraseña", &mut form.clave, &mut form.ver_clave, true));
        let enter = (usuario.lost_focus() || clave.lost_focus()) && ui.input(|i| i.key_pressed(Key::Enter));

        mensaje_en_linea(ui, P, estado);
        ui.add_space(6.0);
        let (mut ingresar, mut cancelar) = (enter, false);
        entrada(ui, d, 3, |ui| {
            ui.horizontal(|ui| {
                ingresar |= boton(ui, P, "Ingresar", true);
                ui.add_space(14.0);
                cancelar = enlace(ui, P.suave, "Cancelar", true, 15.0);
            });
        });
        ui.add_space(30.0);
        let (mut recuperar_pulsado, mut registrar) = (false, false);
        entrada(ui, d, 4, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 26.0;
                registrar = enlace(ui, P.suave, "Registro", true, 14.0);
                recuperar_pulsado = enlace(ui, P.suave, "Recuperar contraseña", true, 14.0);
            });
            if estado.es_primer_registro() {
                ui.label(RichText::new("Aún no hay cuentas. Cree la primera en Registro.").size(13.0).color(P.suave));
            }
        });

        if ingresar {
            estado.ingresar();
        }
        if cancelar {
            estado.cancelar_acceso();
        }
        if recuperar_pulsado {
            estado.alternar_recuperar();
        }
        if registrar {
            estado.abrir_registro();
        }
    });
}

fn recuperar(contexto: &egui::Context, estado: &mut AppState) {
    columna(contexto, estado, |ui, estado| {
        let d = estado.cambio_de_formulario;
        marca(ui, d);
        entrada(ui, d, 1, |ui| {
            ui.label(RichText::new("Le enviaremos su usuario y una contraseña temporal.").color(P.suave));
            ui.add_space(14.0);
        });
        let enviando = estado.acceso.envio.is_some();
        let etiqueta = "Identificación o correo electrónico";
        let dato = entrada(ui, d, 2, |ui| campo(ui, P, etiqueta, &mut estado.acceso.dato, !enviando));
        let enter = dato.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter));
        mensaje_en_linea(ui, P, estado);
        ui.add_space(6.0);
        let mut volver = false;
        entrada(ui, d, 3, |ui| {
            ui.horizontal(|ui| {
                let texto = if enviando { "Enviando" } else { "Enviar" };
                if boton(ui, P, texto, !enviando) || enter {
                    estado.recuperar();
                }
                ui.add_space(14.0);
                volver = enlace(ui, P.suave, "Volver", !enviando, 15.0);
            });
        });
        if volver {
            estado.alternar_recuperar();
        }
    });
}

pub fn registro(contexto: &egui::Context, estado: &mut AppState) {
    columna(contexto, estado, |ui, estado| {
        let d = estado.cambio_de_formulario;
        let primero = estado.es_primer_registro();
        let paso = estado.inicial.paso;
        let total = if primero { 3 } else { 2 };
        entrada(ui, d, 0, |ui| {
            let titulo = ["Quién es usted", "Cómo le contactamos", "Su acceso al sistema"][paso.min(2)];
            ui.label(RichText::new(titulo).font(liviana(40.0)).color(P.texto));
            ui.label(RichText::new(format!("Paso {} de {total}", paso + 1)).size(13.0).color(P.suave));
            ui.add_space(14.0);
        });
        entrada(ui, d, 1, |ui| match paso {
            0 => campos_identidad(ui, P, &mut estado.persona.datos, true),
            1 => campos_contacto(ui, P, &mut estado.persona.datos),
            _ => {
                ui.label(RichText::new("Es la primera cuenta: quedará como administrador.").color(P.suave));
                ui.add_space(8.0);
                let form = &mut estado.inicial;
                campo(ui, P, "Nombre de usuario", &mut form.nombreu, true);
                campo_clave(ui, P, "Contraseña", &mut form.clave, &mut form.ver_clave, true);
            }
        });
        mensaje_en_linea(ui, P, estado);
        ui.add_space(6.0);
        let (mut seguir, mut atras) = (false, false);
        entrada(ui, d, 2, |ui| {
            ui.horizontal(|ui| {
                let ultimo = paso + 1 == total;
                seguir = boton(ui, P, if ultimo { "Guardar" } else { "Siguiente" }, true);
                ui.add_space(14.0);
                atras = enlace(ui, P.suave, if paso == 0 { "Salir" } else { "Volver" }, true, 15.0);
            });
        });
        if seguir {
            estado.registro_siguiente();
        }
        if atras {
            estado.registro_atras();
        }
    });
}

/// Al ingresar, el pino sube como un telón y descubre la aplicación.
pub fn telon(contexto: &egui::Context, estado: &mut AppState) {
    let Some(desde) = estado.telon else {
        return;
    };
    let p = desde.elapsed().as_secs_f32() / duracion(0.42);
    if p >= 1.0 {
        estado.telon = None;
        return;
    }
    let pantalla = contexto.screen_rect();
    let subida = easing::cubic_in_out(p) * pantalla.height();
    let rect = pantalla.translate(vec2(0.0, -subida));
    let pintor = contexto.layer_painter(egui::LayerId::new(Order::Tooltip, egui::Id::new("telon")));
    let tiempo = contexto.input(|i| i.time) as f32;
    paisaje::pintar(&pintor, rect, &paisaje::AMANECER, 10.0, tiempo, 54.0);
    contexto.request_repaint();
}
