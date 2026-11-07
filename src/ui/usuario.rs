//! Gestión Usuario (solo Administrador). Cada usuario se crea para una persona ya registrada
//! y comparte su identificación: esa es la relación 1:1 del modelo.

use super::comun::*;
use super::theme::*;
use super::{AccionPendiente, AppState, Confirmacion, Modo};
use crate::auth;
use crate::models::{estado_de, estado_texto, Perfil, Persona, Usuario, UsuarioVista, ACTIVO, INACTIVO};
use crate::validaciones;
use eframe::egui;

#[derive(Default)]
pub struct FormUsuario {
    pub id: Option<i64>,
    pub idpersona: Option<i64>,
    pub nombre_persona: String,
    pub nombreu: String,
    pub contrasena: String,
    pub ver_clave: bool,
    pub idperfil: Option<i64>,
    pub activo: bool,
    pub modo: Modo,
    pub hoja: bool,
    pub buscar: String,
    pub lista: Vec<UsuarioVista>,
    pub perfiles: Vec<Perfil>,
    pub personas_libres: Vec<Persona>,
}

impl AppState {
    pub fn usuario_abrir(&mut self) {
        self.usuario = FormUsuario { activo: true, ..Default::default() };
        self.usuario_recargar();
    }

    pub fn usuario_recargar(&mut self) {
        let datos = (|| {
            let lista = self.db.listar_usuarios()?;
            let perfiles = self.db.listar_perfiles()?;
            let libres = self.db.personas_sin_usuario()?;
            Ok::<_, String>((lista, perfiles, libres))
        })();
        match datos {
            Ok((lista, perfiles, libres)) => {
                self.usuario.lista = lista;
                self.usuario.perfiles = perfiles.into_iter().filter(|p| p.estado == ACTIVO).collect();
                self.usuario.personas_libres = libres;
            }
            Err(e) => self.error(e),
        }
    }

    pub fn usuario_mostrar(&mut self, vista: &UsuarioVista) {
        let u = &vista.usuario;
        self.usuario.id = Some(u.idusuario);
        self.usuario.idpersona = Some(u.idusuario);
        self.usuario.nombre_persona = vista.persona.clone();
        self.usuario.nombreu = u.nombreu.clone();
        self.usuario.contrasena.clear();
        self.usuario.idperfil = Some(u.idperfil);
        self.usuario.activo = u.estado == ACTIVO;
        self.usuario.modo = Modo::Consulta;
        self.usuario.hoja = true;
    }

    /// Salir de la hoja: descarta lo escrito y vuelve a la consulta o a la lista.
    pub fn usuario_salir_de_hoja(&mut self) {
        self.confirmar = None;
        self.usuario.contrasena.clear();
        let guardado = self.usuario.id.filter(|_| self.usuario.modo == Modo::Edicion);
        let vista = guardado.and_then(|id| self.usuario.lista.iter().find(|v| v.usuario.idusuario == id).cloned());
        match vista {
            Some(v) => self.usuario_mostrar(&v),
            None => self.usuario.hoja = false,
        }
    }

    pub fn usuario_crear(&mut self) {
        self.usuario_recargar();
        self.usuario.id = None;
        self.usuario.idpersona = None;
        self.usuario.nombre_persona.clear();
        self.usuario.nombreu.clear();
        self.usuario.contrasena.clear();
        self.usuario.idperfil = None;
        self.usuario.activo = true;
        self.usuario.modo = Modo::Nuevo;
        self.usuario.hoja = true;
        if self.usuario.personas_libres.is_empty() {
            self.informar("Todas las personas activas ya tienen usuario. Registre primero una persona.");
        }
    }

    pub fn usuario_consultar(&mut self) {
        self.usuario_recargar();
        let dato = self.usuario.buscar.trim().to_lowercase();
        if dato.is_empty() {
            let total = self.usuario.lista.len();
            return self.informar(format!("Usuarios en la base de datos: {total}."));
        }
        let hallado = self
            .usuario
            .lista
            .iter()
            .find(|v| v.usuario.idusuario.to_string() == dato || v.usuario.nombreu.to_lowercase() == dato)
            .cloned();
        match hallado {
            Some(vista) => {
                self.usuario_mostrar(&vista);
                self.exito(format!("Usuario encontrado: {}.", vista.usuario.nombreu));
            }
            None => self.error("No existe un usuario con ese dato."),
        }
    }

    pub fn usuario_modificar(&mut self) {
        if self.usuario.id.is_none() {
            return self.error("Primero consulte o seleccione un usuario.");
        }
        self.usuario.modo = Modo::Edicion;
    }

    /// Mantiene al menos un administrador que pueda ingresar.
    fn usuario_se_puede_retirar(&self, id: i64) -> Result<(), String> {
        if Some(id) == self.id_en_sesion() {
            return Err("No puede inhabilitar el usuario con el que inició sesión.".into());
        }
        let usuario = self.db.usuario_por_id(id)?.ok_or("Ese usuario no existe.")?;
        let perfil = self.db.perfil_por_id(usuario.idperfil)?.ok_or("El usuario no tiene perfil.")?;
        let es_administrador = perfil.descripc.eq_ignore_ascii_case("Administrador");
        if es_administrador && usuario.estado == ACTIVO && self.db.administradores_activos()? <= 1 {
            return Err("Debe quedar al menos un administrador activo.".into());
        }
        Ok(())
    }

    pub fn usuario_pedir_inhabilitar(&mut self) {
        let Some(id) = self.usuario.id else {
            return self.error("Primero consulte o seleccione un usuario.");
        };
        if let Err(e) = self.usuario_se_puede_retirar(id) {
            return self.error(e);
        }
        self.confirmar = Some(Confirmacion {
            texto: format!("¿Desea inhabilitar el usuario {}?", self.usuario.nombreu),
            accion: AccionPendiente::InhabilitarUsuario(id),
        });
    }

    pub fn usuario_inhabilitar(&mut self, id: i64) {
        if let Err(e) = self.usuario_se_puede_retirar(id) {
            return self.error(e);
        }
        match self.db.cambiar_estado_usuario(id, INACTIVO) {
            Ok(()) => {
                self.usuario.activo = false;
                self.usuario.modo = Modo::Consulta;
                self.usuario.hoja = false;
                self.usuario_recargar();
                self.exito("Usuario inhabilitado. El registro sigue en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }

    fn usuario_escribir(&mut self) -> Result<(), String> {
        let nombreu = self.usuario.nombreu.trim().to_string();
        validaciones::nombre_usuario(&nombreu)?;
        let idperfil = self.usuario.idperfil.ok_or("Debe escoger un perfil.")?;
        let perfil = self.db.perfil_por_id(idperfil)?.ok_or("Ese perfil no existe.")?;
        if perfil.estado != ACTIVO {
            return Err("Ese perfil está inhabilitado.".into());
        }
        let ajeno = self.db.usuario_por_nombre(&nombreu)?.is_some_and(|u| Some(u.idusuario) != self.usuario.id);
        if ajeno {
            return Err("Ese nombre de usuario ya existe.".into());
        }
        let estado = estado_de(self.usuario.activo);

        match (self.usuario.modo, self.usuario.id) {
            (Modo::Nuevo, _) => {
                let id = self.usuario.idpersona.ok_or("Debe escoger la persona del usuario.")?;
                let persona = self.db.persona_por_id(id)?.ok_or("Esa persona no está registrada.")?;
                if persona.estado != ACTIVO {
                    return Err("Esa persona está inhabilitada.".into());
                }
                if self.db.usuario_por_id(id)?.is_some() {
                    return Err("Esa persona ya tiene un usuario.".into());
                }
                validaciones::contrasena(&self.usuario.contrasena)?;
                let hash = auth::cifrar(&self.usuario.contrasena)?;
                self.db.crear_usuario(&Usuario {
                    idusuario: id,
                    nombreu,
                    contrasena: hash,
                    idperfil,
                    estado: estado.to_string(),
                })?;
                self.usuario.id = Some(id);
                Ok(())
            }
            (Modo::Edicion, Some(id)) => {
                let actual = self.db.usuario_por_id(id)?.ok_or("Ese usuario no existe.")?;
                if !self.usuario.activo || idperfil != actual.idperfil {
                    self.usuario_se_puede_retirar(id).or_else(|e| {
                        if Some(id) == self.id_en_sesion() && self.usuario.activo {
                            Err("No puede cambiar el perfil del usuario con el que inició sesión.".to_string())
                        } else {
                            Err(e)
                        }
                    })?;
                }
                if !self.usuario.contrasena.is_empty() {
                    validaciones::contrasena(&self.usuario.contrasena)?;
                    let hash = auth::cifrar(&self.usuario.contrasena)?;
                    self.db.cambiar_contrasena(id, &hash)?;
                }
                self.db.modificar_usuario(id, &nombreu, idperfil, estado)
            }
            _ => Err("Pulse Crear o Modificar antes de guardar.".into()),
        }
    }

    /// Si el administrador editó su propio usuario, el menú debe mostrar el dato nuevo.
    fn usuario_actualizar_sesion(&mut self) {
        let Some(id) = self.id_en_sesion() else {
            return;
        };
        if let Ok(Some(usuario)) = self.db.usuario_por_id(id) {
            if let Some(sesion) = &mut self.sesion {
                sesion.usuario = usuario;
            }
        }
    }

    pub fn usuario_guardar(&mut self) {
        match self.usuario_escribir() {
            Ok(()) => {
                self.usuario.contrasena.clear();
                self.usuario.modo = Modo::Consulta;
                self.usuario.hoja = false;
                self.usuario_recargar();
                self.usuario_actualizar_sesion();
                self.exito("Usuario guardado en la base de datos.");
            }
            Err(e) => self.error(e),
        }
    }
}

fn selector_de_persona(ui: &mut egui::Ui, form: &mut FormUsuario) {
    if form.personas_libres.is_empty() {
        dato(ui, "Persona registrada", "No hay personas sin usuario");
        return;
    }
    let texto = form
        .personas_libres
        .iter()
        .find(|p| Some(p.idpersona) == form.idpersona)
        .map(Persona::nombre_completo)
        .unwrap_or_else(|| "Escoja una persona".to_string());
    selector(ui, "Persona registrada", "persona_usuario", texto, |ui| {
        for p in &form.personas_libres {
            ui.selectable_value(&mut form.idpersona, Some(p.idpersona), p.nombre_completo());
        }
    });
}

fn selector_de_perfil(ui: &mut egui::Ui, form: &mut FormUsuario) {
    let texto = form
        .perfiles
        .iter()
        .find(|p| Some(p.idperfil) == form.idperfil)
        .map(|p| p.descripc.clone())
        .unwrap_or_else(|| "Escoja un perfil".to_string());
    selector(ui, "Perfil", "perfil_usuario", texto, |ui| {
        for p in &form.perfiles {
            ui.selectable_value(&mut form.idperfil, Some(p.idperfil), &p.descripc);
        }
    });
}

fn ficha(ui: &mut egui::Ui, estado: &mut AppState) {
    let modo = estado.usuario.modo;
    let form = &mut estado.usuario;
    let titulo = if modo == Modo::Nuevo { "Usuario nuevo".to_string() } else { form.nombreu.clone() };
    titulo_de_hoja(ui, &titulo);
    match modo {
        Modo::Consulta => {
            let id = form.id.map(|i| i.to_string()).unwrap_or_default();
            let perfil = form.lista.iter().find(|v| Some(v.usuario.idusuario) == form.id).map(|v| v.perfil.clone());
            dato(ui, "Identificación", &id);
            dato(ui, "Persona", &form.nombre_persona);
            dato(ui, "Perfil", &perfil.unwrap_or_default());
            dato(ui, "Estado", if form.activo { "Activo" } else { "Inhabilitado" });
        }
        Modo::Nuevo | Modo::Edicion => {
            if modo == Modo::Nuevo {
                selector_de_persona(ui, form);
            } else {
                dato(ui, "Persona", &form.nombre_persona);
            }
            campo(ui, SOBRE_NIEBLA, "Nombre de usuario", &mut form.nombreu, true);
            selector_de_perfil(ui, form);
            let etiqueta = if modo == Modo::Nuevo { "Contraseña" } else { "Contraseña nueva, vacía conserva la actual" };
            campo_clave(ui, SOBRE_NIEBLA, etiqueta, &mut form.contrasena, &mut form.ver_clave, true);
            interruptor(ui, SOBRE_NIEBLA, &mut form.activo, true);
        }
    }
    ui.add_space(18.0);
    let pregunta = estado.confirmar.as_ref().map(|c| c.texto.clone());
    match acciones(ui, modo, estado.usuario.activo, pregunta.as_deref()) {
        Some(AccionHoja::Modificar) => estado.usuario_modificar(),
        Some(AccionHoja::Inhabilitar) => estado.usuario_pedir_inhabilitar(),
        Some(AccionHoja::Guardar) => estado.usuario_guardar(),
        Some(AccionHoja::Confirmado) => estado.confirmado(),
        Some(AccionHoja::Volver) => estado.confirmar = None,
        Some(AccionHoja::Salir) => estado.usuario_salir_de_hoja(),
        None => {}
    }
}

pub fn pantalla(ui: &mut egui::Ui, estado: &mut AppState) {
    if cabecera(ui, "Usuarios", Some("Crear")) {
        estado.usuario_crear();
    }
    if barra_de_consulta(ui, "Consultar por identificación o usuario", &mut estado.usuario.buscar) {
        estado.usuario_consultar();
    }

    let filtro = estado.usuario.buscar.trim().to_lowercase();
    let visibles: Vec<usize> = (0..estado.usuario.lista.len())
        .filter(|&i| {
            let v = &estado.usuario.lista[i];
            filtro.is_empty()
                || v.usuario.idusuario.to_string().contains(&filtro)
                || v.usuario.nombreu.to_lowercase().contains(&filtro)
                || v.persona.to_lowercase().contains(&filtro)
        })
        .collect();
    let filas: Vec<Vec<String>> = visibles
        .iter()
        .map(|&i| {
            let v = &estado.usuario.lista[i];
            vec![
                v.usuario.idusuario.to_string(),
                v.usuario.nombreu.clone(),
                v.persona.clone(),
                v.perfil.clone(),
                estado_texto(&v.usuario.estado).to_string(),
            ]
        })
        .collect();
    let elegida = visibles
        .iter()
        .position(|&i| estado.usuario.hoja && Some(estado.usuario.lista[i].usuario.idusuario) == estado.usuario.id);
    let foto = estado.foto_actual();
    let resaltar = |i: usize| foto.resaltado_fila("usuario", &filas[i][0]);
    let columnas = [
        col("Identificación", 1.6),
        col("Usuario", 2.0),
        col("Persona", 3.0),
        col("Perfil", 1.8),
        col("Estado", 1.5),
    ];
    if let Some(i) = tabla(ui, "tabla_usuario", &columnas, &filas, elegida, Some(4), &resaltar) {
        let vista = estado.usuario.lista[visibles[i]].clone();
        estado.usuario_mostrar(&vista);
    }
}

pub fn hoja_lateral(contexto: &egui::Context, estado: &mut AppState) {
    let abierta = estado.usuario.hoja;
    let cerrar = hoja(contexto, "hoja_usuario", abierta, |ui| ficha(ui, estado));
    if abierta && cerrar && estado.usuario.modo == Modo::Consulta {
        estado.usuario_salir_de_hoja();
    }
}
