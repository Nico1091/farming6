//! Gestión Perfil: Crear, Consultar, Modificar, Inhabilitar, Guardar y Salir (solo Administrador).

use super::comun::*;
use super::theme::*;
use super::{AccionPendiente, AppState, Confirmacion, Modo};
use crate::models::{estado_de, estado_texto, Perfil, ACTIVO, INACTIVO};
use crate::validaciones;
use eframe::egui;

#[derive(Default)]
pub struct FormPerfil {
    pub id: Option<i64>,
    pub descripc: String,
    pub activo: bool,
    pub modo: Modo,
    pub hoja: bool,
    pub buscar: String,
    pub lista: Vec<Perfil>,
}

impl AppState {
    pub fn perfil_abrir(&mut self) {
        self.perfil = FormPerfil { activo: true, ..Default::default() };
        self.perfil_recargar();
    }

    pub fn perfil_recargar(&mut self) {
        match self.db.listar_perfiles() {
            Ok(lista) => self.perfil.lista = lista,
            Err(e) => self.error(e),
        }
    }

    pub fn perfil_mostrar(&mut self, p: &Perfil) {
        self.perfil.id = Some(p.idperfil);
        self.perfil.descripc = p.descripc.clone();
        self.perfil.activo = p.estado == ACTIVO;
        self.perfil.modo = Modo::Consulta;
        self.perfil.hoja = true;
    }

    /// Salir de la hoja: descarta lo escrito y vuelve a la consulta o a la lista.
    pub fn perfil_salir_de_hoja(&mut self) {
        self.confirmar = None;
        let guardado = self.perfil.id.filter(|_| self.perfil.modo == Modo::Edicion);
        match guardado.and_then(|id| self.db.perfil_por_id(id).ok().flatten()) {
            Some(p) => self.perfil_mostrar(&p),
            None => self.perfil.hoja = false,
        }
    }

    pub fn perfil_crear(&mut self) {
        self.perfil.id = None;
        self.perfil.descripc.clear();
        self.perfil.activo = true;
        self.perfil.modo = Modo::Nuevo;
        self.perfil.hoja = true;
    }

    pub fn perfil_consultar(&mut self) {
        self.perfil_recargar();
        let dato = self.perfil.buscar.trim().to_string();
        if dato.is_empty() {
            let total = self.perfil.lista.len();
            return self.informar(format!("Perfiles en la base de datos: {total}."));
        }
        let hallado = match dato.parse::<i64>() {
            Ok(id) => self.db.perfil_por_id(id),
            Err(_) => self.db.perfil_por_descripcion(&dato),
        };
        match hallado {
            Ok(Some(p)) => {
                self.perfil_mostrar(&p);
                self.exito(format!("Perfil encontrado: {}.", p.descripc));
            }
            Ok(None) => self.error("No existe un perfil con ese código o descripción."),
            Err(e) => self.error(e),
        }
    }

    pub fn perfil_modificar(&mut self) {
        if self.perfil.id.is_none() {
            return self.error("Primero consulte o seleccione un perfil.");
        }
        self.perfil.modo = Modo::Edicion;
    }

    /// Reglas que protegen el sistema antes de apagar un perfil.
    fn perfil_se_puede_inhabilitar(&self, id: i64) -> Result<(), String> {
        let perfil = self.db.perfil_por_id(id)?.ok_or("Ese perfil no existe.")?;
        if perfil.descripc.eq_ignore_ascii_case("Administrador") {
            return Err("El perfil Administrador no se puede inhabilitar.".into());
        }
        let activos = self.db.usuarios_activos_de_perfil(id)?;
        if activos > 0 {
            return Err(format!("Este perfil tiene {activos} usuario(s) activo(s). Cámbielos de perfil primero."));
        }
        Ok(())
    }

    pub fn perfil_pedir_inhabilitar(&mut self) {
        let Some(id) = self.perfil.id else {
            return self.error("Primero consulte o seleccione un perfil.");
        };
        if let Err(e) = self.perfil_se_puede_inhabilitar(id) {
            return self.error(e);
        }
        self.confirmar = Some(Confirmacion {
            texto: format!("¿Desea inhabilitar el perfil {}?", self.perfil.descripc),
            accion: AccionPendiente::InhabilitarPerfil(id),
        });
    }

    pub fn perfil_inhabilitar(&mut self, id: i64) {
        if let Err(e) = self.perfil_se_puede_inhabilitar(id) {
            return self.error(e);
        }
        match self.db.cambiar_estado_perfil(id, INACTIVO) {
            Ok(()) => {
                self.perfil.activo = false;
                self.perfil.modo = Modo::Consulta;
                self.perfil.hoja = false;
                self.perfil_recargar();
                self.exito("Perfil inhabilitado. El registro sigue en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }

    pub fn perfil_guardar(&mut self) {
        let descripc = self.perfil.descripc.trim().to_string();
        if let Err(e) = validaciones::descripcion_perfil(&descripc) {
            return self.error(e);
        }
        // Se compara aquí porque NOCASE de SQLite no distingue tildes: «TÉCNICO» y «técnico».
        let perfiles = match self.db.listar_perfiles() {
            Ok(lista) => lista,
            Err(e) => return self.error(e),
        };
        let buscado = descripc.to_lowercase();
        if perfiles.iter().any(|p| p.descripc.to_lowercase() == buscado && Some(p.idperfil) != self.perfil.id) {
            return self.error("Ya existe un perfil con esa descripción.");
        }
        let estado = estado_de(self.perfil.activo);
        let resultado = match (self.perfil.modo, self.perfil.id) {
            (Modo::Nuevo, _) => match self.db.crear_perfil(&descripc, estado) {
                Ok(id) => {
                    self.perfil.id = Some(id);
                    Ok(())
                }
                Err(e) => Err(e),
            },
            (Modo::Edicion, Some(id)) => {
                if estado == INACTIVO {
                    if let Err(e) = self.perfil_se_puede_inhabilitar(id) {
                        return self.error(e);
                    }
                }
                let es_administrador = self
                    .db
                    .perfil_por_id(id)
                    .ok()
                    .flatten()
                    .is_some_and(|p| p.descripc.eq_ignore_ascii_case("Administrador"));
                if es_administrador && !descripc.eq_ignore_ascii_case("Administrador") {
                    return self.error("El perfil Administrador no se puede renombrar: de él dependen los permisos.");
                }
                self.db.modificar_perfil(id, &descripc, estado)
            }
            _ => return self.error("Pulse Crear o Modificar antes de guardar."),
        };
        match resultado {
            Ok(()) => {
                self.perfil.modo = Modo::Consulta;
                self.perfil.hoja = false;
                self.perfil_recargar();
                self.exito("Perfil guardado en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }
}

pub fn pantalla(ui: &mut egui::Ui, estado: &mut AppState) {
    if cabecera(ui, "Perfiles", Some("Crear")) {
        estado.perfil_crear();
    }
    if barra_de_consulta(ui, "Consultar por código o descripción", &mut estado.perfil.buscar) {
        estado.perfil_consultar();
    }

    let filtro = estado.perfil.buscar.trim().to_lowercase();
    let visibles: Vec<usize> = (0..estado.perfil.lista.len())
        .filter(|&i| {
            let p = &estado.perfil.lista[i];
            filtro.is_empty() || p.idperfil.to_string() == filtro || p.descripc.to_lowercase().contains(&filtro)
        })
        .collect();
    let filas: Vec<Vec<String>> = visibles
        .iter()
        .map(|&i| {
            let p = &estado.perfil.lista[i];
            vec![p.idperfil.to_string(), p.descripc.clone(), estado_texto(&p.estado).to_string()]
        })
        .collect();
    let elegida = visibles
        .iter()
        .position(|&i| estado.perfil.hoja && Some(estado.perfil.lista[i].idperfil) == estado.perfil.id);
    let foto = estado.foto_actual();
    let resaltar = |i: usize| foto.resaltado_fila("perfil", &filas[i][0]);
    let columnas = [col("Código", 1.0), col("Descripción", 4.0), col("Estado", 1.5)];
    if let Some(i) = tabla(ui, "tabla_perfil", &columnas, &filas, elegida, Some(2), &resaltar) {
        let perfil = estado.perfil.lista[visibles[i]].clone();
        estado.perfil_mostrar(&perfil);
    }
}

fn ficha(ui: &mut egui::Ui, estado: &mut AppState) {
    let modo = estado.perfil.modo;
    let form = &mut estado.perfil;
    let titulo = if modo == Modo::Nuevo { "Perfil nuevo".to_string() } else { form.descripc.clone() };
    titulo_de_hoja(ui, &titulo);
    if let Some(id) = form.id.filter(|_| modo != Modo::Nuevo) {
        dato(ui, "Código", &id.to_string());
    }
    if modo == Modo::Consulta {
        dato(ui, "Estado", if form.activo { "Activo" } else { "Inhabilitado" });
    } else {
        campo(ui, SOBRE_NIEBLA, "Descripción", &mut form.descripc, true);
        interruptor(ui, SOBRE_NIEBLA, &mut form.activo, true);
    }
    ui.add_space(18.0);
    let pregunta = estado.confirmar.as_ref().map(|c| c.texto.clone());
    match acciones(ui, modo, estado.perfil.activo, pregunta.as_deref()) {
        Some(AccionHoja::Modificar) => estado.perfil_modificar(),
        Some(AccionHoja::Inhabilitar) => estado.perfil_pedir_inhabilitar(),
        Some(AccionHoja::Guardar) => estado.perfil_guardar(),
        Some(AccionHoja::Confirmado) => estado.confirmado(),
        Some(AccionHoja::Volver) => estado.confirmar = None,
        Some(AccionHoja::Salir) => estado.perfil_salir_de_hoja(),
        None => {}
    }
}

pub fn hoja_lateral(contexto: &egui::Context, estado: &mut AppState) {
    let abierta = estado.perfil.hoja;
    let cerrar = hoja(contexto, "hoja_perfil", abierta, |ui| ficha(ui, estado));
    if abierta && cerrar && estado.perfil.modo == Modo::Consulta {
        estado.perfil_salir_de_hoja();
    }
}
