//! Gestión de Productores (HU-01). El productor es una persona: se usa el formulario de persona
//! con las adaptaciones de la historia (teléfono de 10 dígitos, dirección y fecha de nacimiento
//! obligatorias, mayor de 18 años y correo opcional), y la tabla productor registra desde cuándo
//! cría cerdos. Solo el Administrador entra a esta pantalla; el productor modifica sus propios
//! datos en «Mis datos», sin el documento ni el estado.

use super::comun::*;
use super::theme::*;
use super::{AccionPendiente, AppState, Confirmacion, Modo};
use crate::models::{estado_de, estado_texto, sin_tildes, DatosPersona, ProductorVista, ACTIVO, INACTIVO};
use crate::{reloj, validaciones};
use eframe::egui;

#[derive(Default)]
pub struct FormProductor {
    pub id: Option<i64>,
    pub datos: DatosPersona,
    pub activo: bool,
    pub modo: Modo,
    pub hoja: bool,
    pub buscar: String,
    pub lista: Vec<ProductorVista>,
}

impl AppState {
    pub fn productor_abrir(&mut self) {
        self.productor = FormProductor { activo: true, ..Default::default() };
        self.productor_recargar();
    }

    pub fn productor_recargar(&mut self) {
        match self.db.listar_productores() {
            Ok(lista) => self.productor.lista = lista,
            Err(e) => self.error(e),
        }
    }

    pub fn productor_mostrar(&mut self, v: &ProductorVista) {
        self.productor.id = Some(v.persona.idpersona);
        self.productor.datos = DatosPersona::desde(&v.persona);
        self.productor.activo = v.persona.estado == ACTIVO;
        self.productor.modo = Modo::Consulta;
        self.productor.hoja = true;
    }

    /// Salir de la hoja: descarta lo escrito y vuelve a la consulta o a la lista.
    pub fn productor_salir_de_hoja(&mut self) {
        self.confirmar = None;
        let guardado = self.productor.id.filter(|_| self.productor.modo == Modo::Edicion);
        match guardado.and_then(|id| self.db.productor_por_id(id).ok().flatten()) {
            Some(v) => self.productor_mostrar(&v),
            None => self.productor.hoja = false,
        }
    }

    pub fn productor_crear(&mut self) {
        self.productor.id = None;
        self.productor.datos = DatosPersona::default();
        self.productor.activo = true;
        self.productor.modo = Modo::Nuevo;
        self.productor.hoja = true;
    }

    /// HU-01: busca por documento, nombre o apellido.
    pub fn productor_consultar(&mut self) {
        self.productor_recargar();
        let dato = sin_tildes(self.productor.buscar.trim());
        if dato.is_empty() {
            let total = self.productor.lista.len();
            return self.informar(format!("Productores en la base de datos: {total}."));
        }
        let hallados: Vec<ProductorVista> = self.productor.lista.iter().filter(|v| coincide(v, &dato)).cloned().collect();
        match hallados.as_slice() {
            [] => self.error("No se encontró un productor con ese documento, nombre o apellido."),
            [uno] => {
                self.productor_mostrar(uno);
                self.exito(format!("Productor encontrado: {}.", uno.persona.nombre_completo()));
            }
            varios => self.informar(format!("Productores que coinciden: {}.", varios.len())),
        }
    }

    pub fn productor_modificar(&mut self) {
        if self.productor.id.is_none() {
            return self.error("Primero consulte o seleccione un productor.");
        }
        self.productor.modo = Modo::Edicion;
    }

    fn productor_se_puede_inhabilitar(&self, id: i64) -> Result<(), String> {
        if !self.es_administrador() {
            return Err("Su perfil no puede inhabilitar productores.".into());
        }
        if Some(id) == self.id_en_sesion() {
            return Err("No puede inhabilitar a la persona con la que inició sesión.".into());
        }
        Ok(())
    }

    pub fn productor_pedir_inhabilitar(&mut self) {
        let Some(id) = self.productor.id else {
            return self.error("Primero consulte o seleccione un productor.");
        };
        if let Err(e) = self.productor_se_puede_inhabilitar(id) {
            return self.error(e);
        }
        let nombre = format!("{} {}", self.productor.datos.nom1, self.productor.datos.apell1);
        self.confirmar = Some(Confirmacion {
            texto: format!("¿Desea inhabilitar al productor {nombre}? Tampoco podrá iniciar sesión."),
            accion: AccionPendiente::InhabilitarProductor(id),
        });
    }

    /// Borrado lógico: el estado de la persona pasa a inhabilitado, así que ya no puede ingresar.
    /// Sus granjas y lotes se conservan.
    pub fn productor_inhabilitar(&mut self, id: i64) {
        if let Err(e) = self.productor_se_puede_inhabilitar(id) {
            return self.error(e);
        }
        match self.db.cambiar_estado_persona(id, INACTIVO) {
            Ok(()) => {
                self.productor.activo = false;
                self.productor.modo = Modo::Consulta;
                self.productor.hoja = false;
                self.productor_recargar();
                self.exito("Productor inhabilitado: ya no puede iniciar sesión. Sus granjas y lotes siguen en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }

    pub fn productor_guardar(&mut self) {
        let hoy = reloj::ahora().fecha;
        // El documento es la llave primaria: al modificar no cambia.
        if let (Modo::Edicion, Some(original)) = (self.productor.modo, self.productor.id) {
            self.productor.datos.identificacion = original.to_string();
        }
        let id = match validaciones::productor(&self.productor.datos, hoy) {
            Ok(id) => id,
            Err(e) => return self.error(e),
        };
        let correo = self.productor.datos.correo.trim().to_lowercase();
        if !correo.is_empty() {
            match self.db.persona_por_correo(&correo) {
                Ok(Some(p)) if p.idpersona != id => return self.error("Ese correo ya está registrado."),
                Err(e) => return self.error(e),
                _ => {}
            }
        }
        let estado = estado_de(self.productor.activo);
        let resultado = match (self.productor.modo, self.productor.id) {
            (Modo::Nuevo, _) => {
                match self.db.es_productor(id) {
                    Ok(true) => return self.error("Ya existe un productor con ese documento."),
                    Err(e) => return self.error(e),
                    Ok(false) => {}
                }
                let persona = self.productor.datos.a_persona(id, ACTIVO);
                self.db.crear_productor(&persona, &validaciones::fecha_texto(hoy)).map(|existia| {
                    if existia {
                        "Productor guardado. La persona ya estaba registrada y sus datos se actualizaron con los del formulario."
                    } else {
                        "Productor guardado en la base de datos."
                    }
                })
            }
            (Modo::Edicion, Some(original)) => {
                if estado == INACTIVO {
                    if let Err(e) = self.productor_se_puede_inhabilitar(original) {
                        return self.error(e);
                    }
                }
                let persona = self.productor.datos.a_persona(original, estado);
                self.db.modificar_persona(&persona).map(|()| "Productor guardado en la base de datos.")
            }
            _ => return self.error("Pulse Crear o Modificar antes de guardar."),
        };
        match resultado {
            Ok(texto) => {
                self.productor.id = Some(id);
                self.productor.modo = Modo::Consulta;
                self.productor.hoja = false;
                self.productor_recargar();
                if let Some(sesion) = &mut self.sesion {
                    if sesion.usuario.idusuario == id {
                        if let Ok(Some(p)) = self.db.persona_por_id(id) {
                            sesion.persona = p;
                        }
                    }
                }
                self.exito(texto);
            }
            Err(e) => self.error(e),
        }
    }
}

fn coincide(v: &ProductorVista, dato: &str) -> bool {
    let p = &v.persona;
    p.idpersona.to_string().contains(dato) || [&p.nom1, &p.nom2, &p.apell1, &p.apell2].iter().any(|parte| sin_tildes(parte).contains(dato))
        || sin_tildes(&p.nombre_completo()).contains(dato)
}

/// El formulario de persona con las adaptaciones de HU-01, en el orden de la historia.
pub fn campos_productor(ui: &mut egui::Ui, p: Paleta, d: &mut DatosPersona, documento_editable: bool) {
    if documento_editable {
        campo(ui, p, "Documento", &mut d.identificacion, true);
    } else {
        dato(ui, "Documento", &d.identificacion);
    }
    ui.columns(2, |c| {
        campo(&mut c[0], p, "Primer nombre", &mut d.nom1, true);
        campo(&mut c[1], p, "Segundo nombre, opcional", &mut d.nom2, true);
        campo(&mut c[0], p, "Primer apellido", &mut d.apell1, true);
        campo(&mut c[1], p, "Segundo apellido, opcional", &mut d.apell2, true);
    });
    campo(ui, p, "Teléfono (10 dígitos)", &mut d.movil, true);
    campo(ui, p, "Dirección", &mut d.direccion, true);
    campo(ui, p, "Fecha de nacimiento (AAAA-MM-DD)", &mut d.fecha_nac, true);
    campo(ui, p, "Correo electrónico, opcional", &mut d.correo, true);
}

pub fn datos_productor(ui: &mut egui::Ui, d: &DatosPersona) {
    dato(ui, "Documento", &d.identificacion);
    ui.columns(2, |c| {
        dato(&mut c[0], "Nombres", format!("{} {}", d.nom1, d.nom2).trim());
        dato(&mut c[1], "Apellidos", format!("{} {}", d.apell1, d.apell2).trim());
        dato(&mut c[0], "Teléfono", &d.movil);
        dato(&mut c[1], "Fecha de nacimiento", &d.fecha_nac);
    });
    dato(ui, "Dirección", &d.direccion);
    dato(ui, "Correo electrónico", &d.correo);
}

fn ficha(ui: &mut egui::Ui, estado: &mut AppState) {
    let modo = estado.productor.modo;
    let form = &mut estado.productor;
    let titulo = if modo == Modo::Nuevo {
        "Productor nuevo".to_string()
    } else {
        format!("{} {}", form.datos.nom1, form.datos.apell1).trim().to_string()
    };
    titulo_de_hoja(ui, &titulo);
    if modo == Modo::Consulta {
        datos_productor(ui, &form.datos);
        if let Some(v) = form.id.and_then(|id| form.lista.iter().find(|v| v.persona.idpersona == id)) {
            ui.columns(2, |c| {
                dato(&mut c[0], "Granjas activas", &v.granjas.to_string());
                dato(&mut c[1], "Productor desde", &v.fecha_registro);
            });
        }
        dato(ui, "Estado", if form.activo { "Activo" } else { "Inhabilitado" });
    } else {
        campos_productor(ui, SOBRE_NIEBLA, &mut form.datos, modo == Modo::Nuevo);
        if modo == Modo::Edicion {
            interruptor(ui, SOBRE_NIEBLA, &mut form.activo, true);
        }
    }
    ui.add_space(16.0);
    let inhabilitable = estado.productor.activo;
    let pregunta = estado.confirmar.as_ref().map(|c| c.texto.clone());
    match acciones(ui, modo, inhabilitable, pregunta.as_deref()) {
        Some(AccionHoja::Modificar) => estado.productor_modificar(),
        Some(AccionHoja::Inhabilitar) => estado.productor_pedir_inhabilitar(),
        Some(AccionHoja::Guardar) => estado.productor_guardar(),
        Some(AccionHoja::Confirmado) => estado.confirmado(),
        Some(AccionHoja::Volver) => estado.confirmar = None,
        Some(AccionHoja::Salir) => estado.productor_salir_de_hoja(),
        None => {}
    }
}

pub fn pantalla(ui: &mut egui::Ui, estado: &mut AppState) {
    if cabecera(ui, "Productores", Some("Crear")) {
        estado.productor_crear();
    }
    if barra_de_consulta(ui, "Consultar por documento, nombre o apellido", &mut estado.productor.buscar) {
        estado.productor_consultar();
    }
    let filtro = sin_tildes(estado.productor.buscar.trim());
    let visibles: Vec<usize> =
        (0..estado.productor.lista.len()).filter(|&i| filtro.is_empty() || coincide(&estado.productor.lista[i], &filtro)).collect();
    let filas: Vec<Vec<String>> = visibles
        .iter()
        .map(|&i| {
            let v = &estado.productor.lista[i];
            vec![
                v.persona.idpersona.to_string(),
                v.persona.nombre_completo(),
                v.persona.movil.clone(),
                v.persona.correo.clone(),
                v.granjas.to_string(),
                estado_texto(&v.persona.estado).to_string(),
            ]
        })
        .collect();
    let elegida = visibles
        .iter()
        .position(|&i| estado.productor.hoja && Some(estado.productor.lista[i].persona.idpersona) == estado.productor.id);
    let foto = estado.foto_actual();
    let resaltar = |i: usize| foto.resaltado_fila("persona", &filas[i][0]).max(foto.resaltado_fila("productor", &filas[i][0]));
    let columnas = [
        col("Documento", 1.4),
        col("Nombre", 2.6),
        col("Teléfono", 1.4),
        col("Correo", 2.4),
        col("Granjas", 1.0),
        col("Estado", 1.3),
    ];
    if let Some(i) = tabla(ui, "tabla_productor", &columnas, &filas, elegida, Some(5), &resaltar) {
        let v = estado.productor.lista[visibles[i]].clone();
        estado.productor_mostrar(&v);
    }
}

pub fn hoja_lateral(contexto: &egui::Context, estado: &mut AppState) {
    let abierta = estado.productor.hoja;
    let cerrar = hoja(contexto, "hoja_productor", abierta, |ui| ficha(ui, estado));
    if abierta && cerrar && estado.productor.modo == Modo::Consulta {
        estado.productor_salir_de_hoja();
    }
}
