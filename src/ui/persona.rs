//! Gestión Persona. El Administrador tiene todas las acciones; el perfil Persona no puede
//! inhabilitar y solo consulta o modifica sus propios datos.

use super::comun::*;
use super::theme::*;
use super::{AccionPendiente, AppState, Confirmacion, Modo, Pantalla};
use crate::models::{estado_de, estado_texto, DatosPersona, Persona, ACTIVO, INACTIVO};
use crate::{reloj, validaciones};
use eframe::egui;

#[derive(Default)]
pub struct FormPersona {
    pub id: Option<i64>,
    pub datos: DatosPersona,
    pub activo: bool,
    pub modo: Modo,
    pub hoja: bool,
    pub buscar: String,
    pub lista: Vec<Persona>,
}

impl AppState {
    pub fn persona_abrir(&mut self) {
        self.persona = FormPersona { activo: true, ..Default::default() };
        self.persona_recargar();
        if !self.es_administrador() {
            if let Some(propia) = self.persona.lista.first().cloned() {
                self.persona_mostrar(&propia);
            }
        }
    }

    pub fn persona_recargar(&mut self) {
        let lista = if self.es_administrador() {
            self.db.listar_personas()
        } else {
            match self.id_en_sesion() {
                Some(id) => self.db.persona_por_id(id).map(|p| p.into_iter().collect()),
                None => Ok(Vec::new()),
            }
        };
        match lista {
            Ok(lista) => self.persona.lista = lista,
            Err(e) => self.error(e),
        }
    }

    pub fn persona_mostrar(&mut self, p: &Persona) {
        self.persona.id = Some(p.idpersona);
        self.persona.datos = DatosPersona::desde(p);
        self.persona.activo = p.estado == ACTIVO;
        self.persona.modo = Modo::Consulta;
        self.persona.hoja = true;
    }

    /// Salir de la hoja: descarta lo escrito. Sin edición en curso cierra la hoja,
    /// y a quien no administra lo devuelve al panel.
    pub fn persona_salir_de_hoja(&mut self) {
        self.confirmar = None;
        let en_edicion = self.persona.modo == Modo::Edicion;
        let guardado = self.persona.id.filter(|_| en_edicion);
        match guardado.and_then(|id| self.db.persona_por_id(id).ok().flatten()) {
            Some(p) => self.persona_mostrar(&p),
            None if self.es_administrador() => self.persona.hoja = false,
            None => self.ir_a(Pantalla::Panel),
        }
    }

    pub fn persona_crear(&mut self) {
        self.persona.id = None;
        self.persona.datos = DatosPersona::default();
        self.persona.activo = true;
        self.persona.modo = Modo::Nuevo;
        self.persona.hoja = true;
    }

    /// Botón Registrarse de la pantalla de acceso: carga el formulario Persona.
    pub fn persona_preparar_registro(&mut self) {
        self.persona = FormPersona { activo: true, modo: Modo::Nuevo, ..Default::default() };
        self.pantalla = Pantalla::Registro;
    }

    pub fn persona_consultar(&mut self) {
        self.persona_recargar();
        let dato = self.persona.buscar.trim().to_lowercase();
        if dato.is_empty() {
            let total = self.persona.lista.len();
            return self.informar(format!("Personas en la base de datos: {total}."));
        }
        let mut hallado = None;
        if self.es_administrador() {
            hallado = match dato.parse::<i64>() {
                Ok(id) => self.db.persona_por_id(id).unwrap_or(None),
                Err(_) => None,
            };
            if hallado.is_none() {
                hallado = self
                    .persona
                    .lista
                    .iter()
                    .find(|p| p.correo == dato || p.nombre_completo().to_lowercase().contains(&dato))
                    .cloned();
            }
        } else if let Some(propia) = self.persona.lista.first() {
            let coincide = propia.idpersona.to_string() == dato || propia.correo == dato;
            hallado = coincide.then(|| propia.clone());
        }
        match hallado {
            Some(p) => {
                self.persona_mostrar(&p);
                self.exito(format!("Persona encontrada: {}.", p.nombre_completo()));
            }
            None => self.error("No se encontró una persona con ese dato."),
        }
    }

    pub fn persona_modificar(&mut self) {
        let Some(id) = self.persona.id else {
            return self.error("Primero consulte o seleccione una persona.");
        };
        if !self.es_administrador() && Some(id) != self.id_en_sesion() {
            return self.error("Solo puede modificar sus propios datos.");
        }
        self.persona.modo = Modo::Edicion;
    }

    /// Reglas que evitan dejar el sistema sin quien lo administre.
    fn persona_se_puede_inhabilitar(&self, id: i64) -> Result<(), String> {
        if !self.es_administrador() {
            return Err("Su perfil no puede inhabilitar personas.".into());
        }
        if Some(id) == self.id_en_sesion() {
            return Err("No puede inhabilitar a la persona con la que inició sesión.".into());
        }
        Ok(())
    }

    pub fn persona_pedir_inhabilitar(&mut self) {
        let Some(id) = self.persona.id else {
            return self.error("Primero consulte o seleccione una persona.");
        };
        if let Err(e) = self.persona_se_puede_inhabilitar(id) {
            return self.error(e);
        }
        let nombre = format!("{} {}", self.persona.datos.nom1, self.persona.datos.apell1);
        self.confirmar = Some(Confirmacion {
            texto: format!("¿Desea inhabilitar a {nombre}? Tampoco podrá ingresar con su usuario."),
            accion: AccionPendiente::InhabilitarPersona(id),
        });
    }

    pub fn persona_inhabilitar(&mut self, id: i64) {
        if let Err(e) = self.persona_se_puede_inhabilitar(id) {
            return self.error(e);
        }
        match self.db.cambiar_estado_persona(id, INACTIVO) {
            Ok(()) => {
                self.persona.activo = false;
                self.persona.modo = Modo::Consulta;
                self.persona.hoja = false;
                self.persona_recargar();
                self.exito("Persona inhabilitada. El registro sigue en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }

    /// Valida y escribe en la base. Sirve para Gestión Persona y para Registrarse.
    fn persona_escribir(&mut self) -> Result<Persona, String> {
        let hoy = reloj::ahora().fecha;
        let id = validaciones::persona(&self.persona.datos, hoy)?;
        let correo = self.persona.datos.correo.trim().to_lowercase();
        let correo_ajeno = self.db.persona_por_correo(&correo)?.is_some_and(|p| Some(p.idpersona) != self.persona.id);
        if correo_ajeno {
            return Err("Ese correo ya está registrado.".into());
        }
        match (self.persona.modo, self.persona.id) {
            (Modo::Nuevo, _) => {
                if self.db.persona_por_id(id)?.is_some() {
                    return Err("Esa identificación ya está registrada.".into());
                }
                let persona = self.persona.datos.a_persona(id, estado_de(self.persona.activo));
                self.db.crear_persona(&persona)?;
                Ok(persona)
            }
            (Modo::Edicion, Some(original)) => {
                if !self.es_administrador() && Some(original) != self.id_en_sesion() {
                    return Err("Solo puede modificar sus propios datos.".into());
                }
                if !self.persona.activo {
                    self.persona_se_puede_inhabilitar(original)?;
                }
                let persona = self.persona.datos.a_persona(original, estado_de(self.persona.activo));
                self.db.modificar_persona(&persona)?;
                Ok(persona)
            }
            _ => Err("Pulse Crear o Modificar antes de guardar.".into()),
        }
    }

    pub fn persona_guardar(&mut self) {
        match self.persona_escribir() {
            Ok(persona) => {
                self.persona.id = Some(persona.idpersona);
                self.persona.modo = Modo::Consulta;
                self.persona.hoja = false;
                self.persona_recargar();
                if let Some(sesion) = &mut self.sesion {
                    if sesion.usuario.idusuario == persona.idpersona {
                        sesion.persona = persona;
                    }
                }
                self.exito("Persona guardada en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }

    /// Guardar desde Registrarse: vuelve al acceso con el resultado.
    pub fn persona_guardar_registro(&mut self) {
        match self.persona_escribir() {
            Ok(_) => {
                self.persona = Default::default();
                self.pantalla = Pantalla::Acceso;
                self.exito("Registro guardado. El administrador debe crear su usuario para que pueda ingresar.");
            }
            Err(e) => self.error(e),
        }
    }
}

/// Primer paso del registro y primera parte de la hoja: como máximo seis campos.
pub fn campos_identidad(ui: &mut egui::Ui, p: Paleta, d: &mut DatosPersona, identificacion_editable: bool) {
    if identificacion_editable {
        campo(ui, p, "Identificación", &mut d.identificacion, true);
    } else {
        dato(ui, "Identificación", &d.identificacion);
    }
    ui.columns(2, |c| {
        campo(&mut c[0], p, "Primer nombre", &mut d.nom1, true);
        campo(&mut c[1], p, "Segundo nombre", &mut d.nom2, true);
        campo(&mut c[0], p, "Primer apellido", &mut d.apell1, true);
        campo(&mut c[1], p, "Segundo apellido", &mut d.apell2, true);
    });
    campo(ui, p, "Fecha de nacimiento, AAAA-MM-DD", &mut d.fecha_nac, true);
}

pub fn campos_contacto(ui: &mut egui::Ui, p: Paleta, d: &mut DatosPersona) {
    ui.columns(2, |c| {
        campo(&mut c[0], p, "Móvil", &mut d.movil, true);
        campo(&mut c[1], p, "Teléfono", &mut d.tele, true);
    });
    campo(ui, p, "Correo electrónico", &mut d.correo, true);
    campo(ui, p, "Dirección", &mut d.direccion, true);
}

fn datos_en_lectura(ui: &mut egui::Ui, d: &DatosPersona) {
    dato(ui, "Identificación", &d.identificacion);
    ui.columns(2, |c| {
        dato(&mut c[0], "Nombres", format!("{} {}", d.nom1, d.nom2).trim());
        dato(&mut c[1], "Apellidos", format!("{} {}", d.apell1, d.apell2).trim());
        dato(&mut c[0], "Móvil", &d.movil);
        dato(&mut c[1], "Teléfono", &d.tele);
        dato(&mut c[0], "Fecha de nacimiento", &d.fecha_nac);
        dato(&mut c[1], "Dirección", &d.direccion);
    });
    dato(ui, "Correo electrónico", &d.correo);
}

fn ficha(ui: &mut egui::Ui, estado: &mut AppState) {
    let administrador = estado.es_administrador();
    let modo = estado.persona.modo;
    let form = &mut estado.persona;
    let titulo = if modo == Modo::Nuevo {
        "Persona nueva".to_string()
    } else {
        format!("{} {}", form.datos.nom1, form.datos.apell1).trim().to_string()
    };
    titulo_de_hoja(ui, &titulo);
    if modo == Modo::Consulta {
        datos_en_lectura(ui, &form.datos);
        if administrador {
            dato(ui, "Estado", if form.activo { "Activo" } else { "Inhabilitado" });
        }
    } else {
        campos_identidad(ui, SOBRE_NIEBLA, &mut form.datos, modo == Modo::Nuevo);
        campos_contacto(ui, SOBRE_NIEBLA, &mut form.datos);
        if administrador {
            interruptor(ui, SOBRE_NIEBLA, &mut form.activo, true);
        }
    }
    ui.add_space(16.0);
    let inhabilitable = administrador && estado.persona.activo;
    let pregunta = estado.confirmar.as_ref().map(|c| c.texto.clone());
    match acciones(ui, modo, inhabilitable, pregunta.as_deref()) {
        Some(AccionHoja::Modificar) => estado.persona_modificar(),
        Some(AccionHoja::Inhabilitar) => estado.persona_pedir_inhabilitar(),
        Some(AccionHoja::Guardar) => estado.persona_guardar(),
        Some(AccionHoja::Confirmado) => estado.confirmado(),
        Some(AccionHoja::Volver) => estado.confirmar = None,
        Some(AccionHoja::Salir) => estado.persona_salir_de_hoja(),
        None => {}
    }
}

/// Quien no administra no ve una lista: ve sus datos en la página.
fn mis_datos(ui: &mut egui::Ui, estado: &mut AppState) {
    egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.set_max_width(480.0);
        ficha(ui, estado);
    });
}

pub fn pantalla(ui: &mut egui::Ui, estado: &mut AppState) {
    if !estado.es_administrador() {
        return mis_datos(ui, estado);
    }
    if cabecera(ui, "Personas", Some("Crear")) {
        estado.persona_crear();
    }
    if barra_de_consulta(ui, "Consultar por identificación, nombre o correo", &mut estado.persona.buscar) {
        estado.persona_consultar();
    }

    let filtro = estado.persona.buscar.trim().to_lowercase();
    let visibles: Vec<usize> = (0..estado.persona.lista.len())
        .filter(|&i| {
            let p = &estado.persona.lista[i];
            filtro.is_empty()
                || p.idpersona.to_string().contains(&filtro)
                || p.correo.contains(&filtro)
                || p.nombre_completo().to_lowercase().contains(&filtro)
        })
        .collect();
    let filas: Vec<Vec<String>> = visibles
        .iter()
        .map(|&i| {
            let p = &estado.persona.lista[i];
            vec![
                p.idpersona.to_string(),
                p.nombre_completo(),
                p.correo.clone(),
                p.movil.clone(),
                estado_texto(&p.estado).to_string(),
            ]
        })
        .collect();
    let elegida = visibles
        .iter()
        .position(|&i| estado.persona.hoja && Some(estado.persona.lista[i].idpersona) == estado.persona.id);
    let foto = estado.foto_actual();
    let resaltar = |i: usize| foto.resaltado_fila("persona", &filas[i][0]);
    let columnas = [
        col("Identificación", 1.6),
        col("Nombre", 3.0),
        col("Correo", 3.0),
        col("Móvil", 1.6),
        col("Estado", 1.5),
    ];
    if let Some(i) = tabla(ui, "tabla_persona", &columnas, &filas, elegida, Some(4), &resaltar) {
        let persona = estado.persona.lista[visibles[i]].clone();
        estado.persona_mostrar(&persona);
    }
}

pub fn hoja_lateral(contexto: &egui::Context, estado: &mut AppState) {
    if !estado.es_administrador() {
        return;
    }
    let abierta = estado.persona.hoja;
    let cerrar = hoja(contexto, "hoja_persona", abierta, |ui| ficha(ui, estado));
    if abierta && cerrar && estado.persona.modo == Modo::Consulta {
        estado.persona_salir_de_hoja();
    }
}
