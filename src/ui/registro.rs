//! Gestión de Granjas (HU-02), Corrales (HU-03), Razas (HU-04) y Lotes (HU-05): Crear,
//! Consultar, Modificar, Inhabilitar, Guardar y Salir. Una sola pantalla sirve a los cuatro
//! formularios; lo que cambia es la definición de cada uno en models/registros.rs. Lo propio de
//! cada entidad va al final: la cascada de departamento y municipio, la imagen de la raza, y los
//! corrales y el cierre del lote.

use super::comun::*;
use super::theme::*;
use super::{AccionPendiente, AppState, Confirmacion, Modo};
use crate::imagen::{self, Imagen};
use crate::models::*;
use crate::reloj;
use crate::validaciones;
use eframe::egui::{self, ColorImage, RichText, TextureId, TextureOptions, Vec2};
use std::path::Path;

/// Corral de la granja del lote en el formulario: si está elegido y cuántos cerdos del lote van ahí.
#[derive(Clone, Debug, PartialEq)]
pub struct CorralEnFormulario {
    pub corral: CorralDisponible,
    pub elegido: bool,
    pub cupos: String,
}

#[derive(Default)]
pub struct FormRegistro {
    pub id: Option<i64>,
    /// Un texto por campo, tal como se digita. En las llaves foráneas es el código elegido.
    pub valores: Vec<String>,
    pub activo: bool,
    /// Estado guardado: «A», «I» o, en el lote, «C» (Cerrado).
    pub estado: String,
    pub modo: Modo,
    pub hoja: bool,
    pub buscar: String,
    pub lista: Vec<Registro>,
    /// Opciones de cada llave foránea: código y texto.
    pub opciones: Vec<Vec<(i64, String)>>,
    /// Valor elegido en cada filtro de la lista; vacío es «Todos».
    pub filtros: Vec<String>,
    pub opciones_filtro: Vec<Vec<(String, String)>>,
    pub buscar_en_filtro: String,
    /// Cascada del municipio: departamento elegido y las listas de cada nivel.
    pub departamento: String,
    pub departamentos: Vec<(String, String)>,
    pub municipios: Vec<(String, String)>,
    pub buscar_departamento: String,
    pub buscar_municipio: String,
    /// Raza: imagen elegida o guardada, y su versión para la vista previa.
    pub imagen: Option<Imagen>,
    pub version_imagen: u64,
    /// Lote: corrales de su granja con lo que se asigna, y la granja de la que se leyeron.
    pub corrales: Vec<CorralEnFormulario>,
    pub granja_de_corrales: Option<i64>,
    /// Lote en consulta: sus corrales asignados.
    pub asignados: Vec<Asignacion>,
    /// Lote: fecha final real mientras se pide el cierre.
    pub cerrando: Option<String>,
}

impl FormRegistro {
    fn vacio(e: &Entidad) -> Self {
        Self {
            activo: true,
            estado: ACTIVO.to_string(),
            valores: vec![String::new(); e.campos.len()],
            opciones: vec![Vec::new(); e.campos.len()],
            filtros: vec![String::new(); e.filtros.len()],
            opciones_filtro: vec![Vec::new(); e.filtros.len()],
            ..Default::default()
        }
    }
}

fn como_texto(opciones: &[(i64, String)]) -> Vec<(String, String)> {
    opciones.iter().map(|(codigo, texto)| (codigo.to_string(), texto.clone())).collect()
}

impl AppState {
    /// El Administrador ve todo; el productor, solo lo de sus granjas.
    pub fn persona_del_filtro(&self) -> Option<i64> {
        if self.es_administrador() {
            None
        } else {
            self.id_en_sesion()
        }
    }

    /// HU-04: el productor solo consulta las razas.
    pub fn solo_consulta(&self, cual: Cual) -> bool {
        cual.entidad().solo_consulta_sin_admin && !self.es_administrador()
    }

    pub fn registro(&self, cual: Cual) -> &FormRegistro {
        &self.registros[cual.indice()]
    }

    pub fn registro_mut(&mut self, cual: Cual) -> &mut FormRegistro {
        &mut self.registros[cual.indice()]
    }

    pub fn registro_abrir(&mut self, cual: Cual) {
        *self.registro_mut(cual) = FormRegistro::vacio(cual.entidad());
        self.registro_recargar(cual);
        self.registro_cargar_filtros(cual);
    }

    pub fn registro_recargar(&mut self, cual: Cual) {
        match self.db.listar_registros(cual.entidad(), self.persona_del_filtro()) {
            Ok(lista) => self.registro_mut(cual).lista = lista,
            Err(e) => self.error(e),
        }
    }

    fn filtro_visible(&self, filtro: &Filtro) -> bool {
        !filtro.solo_admin || self.es_administrador()
    }

    /// Opciones de cada filtro visible; el que depende de otro solo carga cuando ese tiene valor.
    pub fn registro_cargar_filtros(&mut self, cual: Cual) {
        let e = cual.entidad();
        let persona = self.persona_del_filtro();
        for (i, filtro) in e.filtros.iter().enumerate() {
            if !self.filtro_visible(filtro) {
                continue;
            }
            let depende = filtro.depende.map(|k| self.registro(cual).filtros[k].parse::<i64>().ok());
            if depende == Some(None) {
                self.registro_mut(cual).opciones_filtro[i].clear();
                continue;
            }
            match self.db.opciones_de_filtro(filtro, persona, depende.flatten()) {
                Ok(opciones) => self.registro_mut(cual).opciones_filtro[i] = opciones,
                Err(error) => self.error(error),
            }
        }
    }

    /// Un filtro cambió: los que dependen de él vuelven a «Todos» y se recargan sus opciones.
    pub fn registro_cambiar_filtro(&mut self, cual: Cual, i: usize) {
        for (k, filtro) in cual.entidad().filtros.iter().enumerate() {
            if filtro.depende == Some(i) {
                self.registro_mut(cual).filtros[k].clear();
            }
        }
        self.registro_cargar_filtros(cual);
    }

    /// Lee de la base las opciones de cada llave foránea y la cascada del municipio.
    fn registro_cargar_opciones(&mut self, cual: Cual) {
        let e = cual.entidad();
        let persona = self.persona_del_filtro();
        for (i, campo) in e.campos.iter().enumerate() {
            match &campo.clase {
                Clase::Opcion(fuente) => {
                    let actual = self.registro(cual).valores[i].parse::<i64>().ok();
                    match self.db.opciones(fuente, persona, actual) {
                        Ok(opciones) => self.registro_mut(cual).opciones[i] = opciones,
                        Err(error) => self.error(error),
                    }
                }
                Clase::Municipio => {
                    match self.db.departamentos() {
                        Ok(lista) => self.registro_mut(cual).departamentos = como_texto(&lista),
                        Err(error) => self.error(error),
                    }
                    let municipio = self.registro(cual).valores[i].parse::<i64>().ok();
                    if let (true, Some(m)) = (self.registro(cual).departamento.is_empty(), municipio) {
                        if let Ok(Some(d)) = self.db.departamento_de_municipio(m) {
                            self.registro_mut(cual).departamento = d.to_string();
                        }
                    }
                    self.cargar_municipios(cual);
                }
                _ => {}
            }
        }
    }

    fn cargar_municipios(&mut self, cual: Cual) {
        let Ok(departamento) = self.registro(cual).departamento.parse::<i64>() else {
            self.registro_mut(cual).municipios.clear();
            return;
        };
        match self.db.municipios_de(departamento) {
            Ok(lista) => self.registro_mut(cual).municipios = como_texto(&lista),
            Err(error) => self.error(error),
        }
    }

    /// HU-02: al cambiar el departamento se listan sus municipios y se borra el municipio elegido.
    pub fn granja_cambio_departamento(&mut self, cual: Cual) {
        let e = cual.entidad();
        if let Some(i) = e.campos.iter().position(|c| matches!(c.clase, Clase::Municipio)) {
            self.registro_mut(cual).valores[i].clear();
        }
        self.cargar_municipios(cual);
    }

    fn siguiente_version(&mut self) -> u64 {
        self.versiones += 1;
        self.versiones
    }

    pub fn registro_mostrar(&mut self, cual: Cual, r: &Registro) {
        let asignados = if cual == Cual::Lote { self.db.asignaciones_de_lote(r.id).unwrap_or_default() } else { Vec::new() };
        let imagen = if cual == Cual::Raza {
            self.db.imagen_de_raza(r.id).ok().flatten().and_then(|bytes| imagen::validar(bytes).ok())
        } else {
            None
        };
        let version = self.siguiente_version();
        let form = self.registro_mut(cual);
        form.id = Some(r.id);
        form.valores = r.valores.clone();
        form.activo = r.estado == ACTIVO;
        form.estado = r.estado.clone();
        form.modo = Modo::Consulta;
        form.hoja = true;
        form.asignados = asignados;
        form.imagen = imagen;
        form.version_imagen = version;
        form.cerrando = None;
        form.corrales.clear();
        form.granja_de_corrales = None;
        form.departamento.clear();
    }

    /// Salir de la hoja: descarta lo escrito y vuelve a la consulta o a la lista.
    pub fn registro_salir_de_hoja(&mut self, cual: Cual) {
        self.confirmar = None;
        let persona = self.persona_del_filtro();
        let form = self.registro(cual);
        let guardado = form.id.filter(|_| form.modo == Modo::Edicion || form.cerrando.is_some());
        match guardado.and_then(|id| self.db.registro_por_id(cual.entidad(), id, persona).ok().flatten()) {
            Some(r) => self.registro_mostrar(cual, &r),
            None => self.registro_mut(cual).hoja = false,
        }
    }

    pub fn registro_crear(&mut self, cual: Cual) {
        if self.solo_consulta(cual) {
            return self.error("Su perfil solo puede consultar las razas.");
        }
        let e = cual.entidad();
        let hoy = validaciones::fecha_texto(reloj::ahora().fecha);
        let anterior = std::mem::take(self.registro_mut(cual));
        let mut form = FormRegistro {
            lista: anterior.lista,
            buscar: anterior.buscar,
            filtros: anterior.filtros,
            opciones_filtro: anterior.opciones_filtro,
            modo: Modo::Nuevo,
            hoja: true,
            ..FormRegistro::vacio(e)
        };
        for (i, campo) in e.campos.iter().enumerate() {
            match campo.clase {
                Clase::Fecha { futura: false } => form.valores[i] = hoy.clone(),
                _ if campo.columna == "margen" => form.valores[i] = "15".into(),
                _ => {}
            }
        }
        // Si la lista está filtrada por una granja, el registro nuevo queda en esa granja.
        if let Some(k) = e.filtros.iter().position(|f| f.dato == Dato::Campo("codigo_granja")) {
            if !form.filtros[k].is_empty() {
                form.valores[e.indice("codigo_granja")] = form.filtros[k].clone();
            }
        }
        form.version_imagen = self.siguiente_version();
        *self.registro_mut(cual) = form;
        self.registro_cargar_opciones(cual);
        self.registro_avisar_si_falta_origen(cual);
    }

    /// Si una lista obligatoria está vacía, dice qué hay que registrar primero.
    fn registro_avisar_si_falta_origen(&mut self, cual: Cual) {
        let e = cual.entidad();
        let form = self.registro(cual);
        let falta = e.campos.iter().enumerate().find(|(i, c)| {
            matches!(c.clase, Clase::Opcion(_)) && form.opciones[*i].is_empty() && !(c.solo_admin && !self.es_administrador())
        });
        if let Some((_, campo)) = falta {
            let texto = format!("Todavía no hay registros activos para elegir en «{}». Créelos primero.", campo.etiqueta);
            self.informar(texto);
        }
    }

    pub fn registro_consultar(&mut self, cual: Cual) {
        let e = cual.entidad();
        self.registro_recargar(cual);
        let dato = sin_tildes(self.registro(cual).buscar.trim());
        let admin = self.es_administrador();
        let form = self.registro(cual);
        let hallados: Vec<Registro> =
            form.lista.iter().filter(|r| pasa_filtros(e, form, r, admin) && (dato.is_empty() || coincide(r, &dato))).cloned().collect();
        if dato.is_empty() {
            return self.informar(format!("{} en la lista: {}.", e.plural, hallados.len()));
        }
        match hallados.as_slice() {
            [] => self.error(format!("No hay {} con ese código o texto.", e.plural.to_lowercase())),
            [uno] => {
                let titulo = uno.rotulos.get(e.titulo).cloned().unwrap_or_default();
                self.registro_mostrar(cual, uno);
                self.exito(format!("{} {}: {titulo}.", e.singular, if e.femenino { "encontrada" } else { "encontrado" }));
            }
            varios => self.informar(format!("{} que coinciden: {}.", e.plural, varios.len())),
        }
    }

    pub fn registro_modificar(&mut self, cual: Cual) {
        if self.solo_consulta(cual) {
            return self.error("Su perfil solo puede consultar las razas.");
        }
        if self.registro(cual).id.is_none() {
            return self.error("Primero consulte o seleccione un registro.");
        }
        let estado = self.registro(cual).estado.clone();
        if cual == Cual::Lote && estado != EN_CEBA {
            let nombre = cual.entidad().nombre_de_estado(&estado).to_lowercase();
            return self.error(format!("Solo se modifica un lote en ceba; este lote está {nombre}."));
        }
        self.registro_mut(cual).modo = Modo::Edicion;
        self.registro_cargar_opciones(cual);
        if cual == Cual::Lote {
            self.lote_cargar_corrales();
        }
    }

    fn registro_se_puede_inhabilitar(&self, cual: Cual, id: i64) -> Result<(), String> {
        let e = cual.entidad();
        if self.solo_consulta(cual) {
            return Err("Su perfil solo puede consultar las razas.".into());
        }
        if self.db.registro_por_id(e, id, self.persona_del_filtro())?.is_none() {
            return Err("Ese registro no existe o no es suyo.".into());
        }
        match self.db.bloqueo_para_inhabilitar(e, id)? {
            Some(mensaje) => Err(mensaje),
            None => Ok(()),
        }
    }

    pub fn registro_pedir_inhabilitar(&mut self, cual: Cual) {
        let e = cual.entidad();
        let Some(id) = self.registro(cual).id else {
            return self.error("Primero consulte o seleccione un registro.");
        };
        if let Err(error) = self.registro_se_puede_inhabilitar(cual, id) {
            return self.error(error);
        }
        self.confirmar = Some(Confirmacion {
            texto: format!("¿Desea inhabilitar {} {} con código {id}?", e.articulo(), e.singular.to_lowercase()),
            accion: AccionPendiente::InhabilitarRegistro(cual, id),
        });
    }

    pub fn registro_inhabilitar(&mut self, cual: Cual, id: i64) {
        let e = cual.entidad();
        if let Err(error) = self.registro_se_puede_inhabilitar(cual, id) {
            return self.error(error);
        }
        match self.db.cambiar_estado_registro(e, id, INACTIVO) {
            Ok(()) => {
                let form = self.registro_mut(cual);
                form.activo = false;
                form.estado = INACTIVO.to_string();
                form.modo = Modo::Consulta;
                form.hoja = false;
                self.registro_recargar(cual);
                self.exito(e.inhabilitado());
            }
            Err(error) => self.error(error),
        }
    }

    /// HU-05: corrales elegidos con sus cupos. Rechaza lo que no cabe y devuelve la advertencia de
    /// la historia cuando la capacidad de los corrales no alcanza para la cantidad del lote.
    fn lote_corrales_elegidos(&self, cantidad: i64) -> Result<(Vec<(i64, i64)>, Option<String>), String> {
        let mut elegidos = Vec::new();
        let (mut capacidad, mut ubicados) = (0, 0);
        for fila in self.registro(Cual::Lote).corrales.iter().filter(|c| c.elegido) {
            let nombre = &fila.corral.nombre;
            let cupos = fila.cupos.trim();
            let cupos: i64 = match cupos.parse() {
                Ok(n) if cupos.chars().all(|c| c.is_ascii_digit()) => n,
                _ => return Err(format!("Escriba cuántos cerdos del lote van en el corral {nombre}, en números enteros.")),
            };
            if cupos < 1 {
                return Err(format!("En el corral {nombre} debe ir al menos un cerdo, o quítelo de la lista."));
            }
            if cupos > fila.corral.libres() {
                return Err(format!(
                    "El corral {nombre} tiene {} espacio(s) libre(s) y le asignó {cupos} cerdos.",
                    fila.corral.libres()
                ));
            }
            elegidos.push((fila.corral.codigo, cupos));
            capacidad += fila.corral.capacidad;
            ubicados += cupos;
        }
        if ubicados > cantidad {
            return Err(format!("Ubicó {ubicados} cerdos en corrales y el lote tiene {cantidad}."));
        }
        let advertencia = if elegidos.is_empty() {
            Some("El lote todavía no tiene corrales asignados.".to_string())
        } else if capacidad < cantidad {
            Some(format!("Atención: los corrales asignados tienen capacidad para {capacidad} cerdos y el lote tiene {cantidad}."))
        } else if ubicados < cantidad {
            Some(format!("Faltan {} cerdos por ubicar en corrales.", cantidad - ubicados))
        } else {
            None
        };
        Ok((elegidos, advertencia))
    }

    /// Reglas que dependen de más de un campo o de otros registros.
    fn registro_reglas(&self, cual: Cual, valores: &[Valor]) -> Result<(), String> {
        let e = cual.entidad();
        let form = self.registro(cual);
        match cual {
            Cual::Lote => {
                let inicial = valores[e.indice("fecha_inicial")].texto();
                let final_estimada = valores[e.indice("fecha_final")].texto();
                if final_estimada <= inicial {
                    return Err(format!("La fecha final estimada debe ser posterior a la fecha inicial ({inicial})."));
                }
            }
            Cual::Corral if form.modo == Modo::Edicion => {
                let capacidad = valores[e.indice("capacidad")].entero().unwrap_or(0);
                let ocupados = form
                    .id
                    .and_then(|id| form.lista.iter().find(|r| r.id == id))
                    .and_then(|r| r.extras.get(1))
                    .and_then(|x| x.parse::<i64>().ok())
                    .unwrap_or(0);
                if capacidad < ocupados {
                    return Err(format!("No se puede reducir la capacidad a {capacidad}: el corral aloja {ocupados} cerdos."));
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn registro_guardar(&mut self, cual: Cual) {
        let e = cual.entidad();
        let hoy = reloj::ahora().fecha;
        let administrador = self.es_administrador();
        let en_sesion = self.id_en_sesion();
        let (modo, id, activo) = {
            let form = self.registro(cual);
            (form.modo, form.id, form.activo)
        };
        if modo == Modo::Consulta {
            return self.error("Pulse Crear o Modificar antes de guardar.");
        }
        if self.solo_consulta(cual) {
            return self.error("Su perfil solo puede consultar las razas.");
        }
        // Al modificar, los campos que no cambian se leen de la base, no del formulario; de paso se
        // confirma que el registro existe y que es de quien está en sesión.
        let en_base = match (modo, id) {
            (Modo::Edicion, Some(id)) => match self.db.registro_por_id(e, id, self.persona_del_filtro()) {
                Ok(Some(r)) => Some(r),
                Ok(None) => return self.error("Ese registro no existe o no es suyo."),
                Err(error) => return self.error(error),
            },
            _ => None,
        };

        let mut valores = Vec::with_capacity(e.campos.len());
        for (i, campo) in e.campos.iter().enumerate() {
            let form = self.registro(cual);
            let propio = campo.solo_admin && !administrador;
            let fijo = modo == Modo::Edicion && campo.cambia == Cambia::AlCrear;
            let valor = if fijo {
                let guardado = en_base.as_ref().map(|r| r.valores[i].as_str()).unwrap_or("");
                validaciones::valor_guardado(campo, guardado)
            } else if let Clase::Imagen = campo.clase {
                form.imagen.as_ref().map(|img| Valor::Bytes(img.bytes.clone())).unwrap_or(Valor::Nulo)
            } else {
                let texto = match (propio, en_sesion) {
                    (true, Some(persona)) => persona.to_string(),
                    _ => form.valores[i].clone(),
                };
                match validaciones::valor_de(campo, &texto, hoy) {
                    Ok(valor) => valor,
                    Err(error) => return self.error(error),
                }
            };
            // La llave foránea debe ser una de las opciones leídas de la base para este usuario.
            if !fijo && !propio {
                let elegido = valor.entero();
                let valida = match (&campo.clase, elegido) {
                    (Clase::Opcion(_), Some(codigo)) => form.opciones[i].iter().any(|(c, _)| *c == codigo),
                    (Clase::Municipio, Some(codigo)) => form.municipios.iter().any(|(c, _)| *c == codigo.to_string()),
                    _ => true,
                };
                if !valida {
                    return self.error(format!("Elija «{}» de la lista.", campo.etiqueta));
                }
            }
            valores.push(valor);
        }
        if let Err(error) = self.registro_reglas(cual, &valores) {
            return self.error(error);
        }

        let mut advertencia = None;
        let mut corrales = None;
        if cual == Cual::Lote && activo {
            // Sin la lista de corrales cargada, guardar dejaría al lote sin sus asignaciones.
            self.lote_cargar_corrales();
            let cantidad = valores[e.indice("cantidad")].entero().unwrap_or(0);
            match self.lote_corrales_elegidos(cantidad) {
                Ok((elegidos, aviso)) => {
                    corrales = Some(elegidos);
                    advertencia = aviso;
                }
                Err(error) => return self.error(error),
            }
        }

        let estado = estado_de(activo);
        if modo == Modo::Edicion && estado == INACTIVO {
            if let Some(id) = id {
                if let Err(error) = self.registro_se_puede_inhabilitar(cual, id) {
                    return self.error(error);
                }
            }
        }
        let hoy_texto = validaciones::fecha_texto(hoy);
        let resultado = match (modo, id) {
            (Modo::Nuevo, _) => self.db.crear_registro(e, &valores, corrales.as_deref(), &hoy_texto),
            (Modo::Edicion, Some(id)) => self.db.modificar_registro(e, id, &valores, estado, corrales.as_deref(), &hoy_texto).map(|()| id),
            _ => return self.error("Pulse Crear o Modificar antes de guardar."),
        };
        match resultado {
            Ok(guardado) => {
                self.miniaturas.retain(|(codigo, _), _| *codigo != guardado);
                let form = self.registro_mut(cual);
                form.id = Some(guardado);
                form.modo = Modo::Consulta;
                form.hoja = false;
                self.registro_recargar(cual);
                match advertencia {
                    Some(aviso) => self.advertir(format!("{} {aviso}", e.guardado())),
                    None => self.exito(e.guardado()),
                }
            }
            Err(error) => self.error(error),
        }
    }

    // Lote (HU-05)

    /// Corrales activos de la granja elegida, con lo que ya tiene asignado el lote. Solo vuelve a
    /// leer la base cuando cambia la granja.
    pub fn lote_cargar_corrales(&mut self) {
        let e = Cual::Lote.entidad();
        let form = self.registro(Cual::Lote);
        let granja = form.valores[e.indice("codigo_granja")].parse::<i64>().ok();
        if granja == form.granja_de_corrales {
            return;
        }
        let lote = form.id.filter(|_| form.modo == Modo::Edicion);
        let Some(granja) = granja else {
            let form = self.registro_mut(Cual::Lote);
            form.corrales.clear();
            form.granja_de_corrales = None;
            return;
        };
        let disponibles = match self.db.corrales_disponibles(granja, lote) {
            Ok(lista) => lista,
            Err(error) => return self.error(error),
        };
        let asignados = lote.map(|l| self.db.asignaciones_de_lote(l).unwrap_or_default()).unwrap_or_default();
        let corrales = disponibles
            .into_iter()
            .map(|corral| {
                let asignado = asignados.iter().find(|a| a.codigo_corral == corral.codigo);
                CorralEnFormulario { elegido: asignado.is_some(), cupos: asignado.map(|a| a.cupos.to_string()).unwrap_or_default(), corral }
            })
            .collect();
        let form = self.registro_mut(Cual::Lote);
        form.corrales = corrales;
        form.granja_de_corrales = Some(granja);
    }

    /// Al elegir un corral, propone tantos cerdos como quepan de los que faltan por ubicar.
    pub fn lote_proponer_cupos(&mut self, k: usize) {
        let e = Cual::Lote.entidad();
        let form = self.registro_mut(Cual::Lote);
        let cantidad = form.valores[e.indice("cantidad")].trim().parse::<i64>().unwrap_or(0);
        let otros: i64 = form
            .corrales
            .iter()
            .enumerate()
            .filter(|(j, c)| *j != k && c.elegido)
            .filter_map(|(_, c)| c.cupos.trim().parse::<i64>().ok())
            .sum();
        let propuesta = (cantidad - otros).min(form.corrales[k].corral.libres());
        form.corrales[k].cupos = if propuesta > 0 { propuesta.to_string() } else { String::new() };
    }

    pub fn lote_pedir_cierre(&mut self) {
        let hoy = validaciones::fecha_texto(reloj::ahora().fecha);
        self.registro_mut(Cual::Lote).cerrando = Some(hoy);
    }

    /// HU-05: cuando todos los cerdos del lote están vendidos o muertos, el lote se cierra con su
    /// fecha final real y pasa a Cerrado.
    pub fn lote_cerrar(&mut self) {
        let e = Cual::Lote.entidad();
        let (Some(id), Some(fecha)) = (self.registro(Cual::Lote).id, self.registro(Cual::Lote).cerrando.clone()) else {
            return;
        };
        let registro = match self.db.registro_por_id(e, id, self.persona_del_filtro()) {
            Ok(Some(r)) => r,
            Ok(None) => return self.error("Ese lote no existe o no es suyo."),
            Err(error) => return self.error(error),
        };
        let vivos: i64 = registro.extras.get(LOTE_VIVOS).and_then(|x| x.parse().ok()).unwrap_or(0);
        if vivos > 0 {
            return self.error(format!(
                "No se puede cerrar: el lote tiene {vivos} cerdos vivos. Se cierra cuando todos estén vendidos o muertos."
            ));
        }
        let fecha_real = match validaciones::leer_fecha(&fecha) {
            Ok(f) => validaciones::fecha_texto(f),
            Err(error) => return self.error(error),
        };
        let inicial = &registro.valores[e.indice("fecha_inicial")];
        if fecha_real.as_str() < inicial.as_str() {
            return self.error(format!("La fecha final real no puede ser anterior a la fecha inicial ({inicial})."));
        }
        if fecha_real > validaciones::fecha_texto(reloj::ahora().fecha) {
            return self.error("La fecha final real no puede ser futura.");
        }
        match self.db.cerrar_lote(id, &fecha_real) {
            Ok(()) => {
                self.registro_recargar(Cual::Lote);
                if let Ok(Some(r)) = self.db.registro_por_id(e, id, self.persona_del_filtro()) {
                    self.registro_mostrar(Cual::Lote, &r);
                }
                self.exito(format!("Lote cerrado con fecha final real {fecha_real}."));
            }
            Err(error) => self.error(error),
        }
    }

    // Raza (HU-04)

    pub fn raza_elegir_imagen(&mut self) {
        if let Some(ruta) = imagen::elegir_archivo() {
            self.raza_cargar_archivo(&ruta);
        }
    }

    pub fn raza_cargar_archivo(&mut self, ruta: &Path) {
        match imagen::leer_archivo(ruta) {
            Ok(imagen) => {
                let texto = format!("Imagen lista: {}. Pulse Guardar para dejarla en la base de datos.", imagen.descripcion());
                let version = self.siguiente_version();
                let form = self.registro_mut(Cual::Raza);
                form.imagen = Some(imagen);
                form.version_imagen = version;
                self.informar(texto);
            }
            Err(error) => self.error(error),
        }
    }

    pub fn raza_quitar_imagen(&mut self) {
        let version = self.siguiente_version();
        let form = self.registro_mut(Cual::Raza);
        form.imagen = None;
        form.version_imagen = version;
    }

    /// Textura de la imagen del formulario de raza; se crea una vez por versión.
    fn vista_previa(&mut self, contexto: &egui::Context) -> Option<(TextureId, Vec2)> {
        let version = self.registro(Cual::Raza).version_imagen;
        if let Some((guardada, textura)) = &self.vista_previa {
            if *guardada == version {
                return Some((textura.id(), textura.size_vec2()));
            }
        }
        let bytes = self.registro(Cual::Raza).imagen.as_ref()?.bytes.clone();
        let pixeles = imagen::miniatura(&bytes, 760).ok()?;
        let color = ColorImage::from_rgba_unmultiplied([pixeles.ancho, pixeles.alto], &pixeles.rgba);
        let textura = contexto.load_texture("vista_previa_de_raza", color, TextureOptions::LINEAR);
        let resultado = (textura.id(), textura.size_vec2());
        self.vista_previa = Some((version, textura));
        Some(resultado)
    }

    /// Miniatura de la imagen guardada de una raza, para la lista.
    fn miniatura_de_raza(&mut self, contexto: &egui::Context, codigor: i64, descripcion: &str) -> Option<(TextureId, Vec2)> {
        if descripcion.is_empty() {
            return None;
        }
        let llave = (codigor, descripcion.to_string());
        if !self.miniaturas.contains_key(&llave) {
            let bytes = self.db.imagen_de_raza(codigor).ok().flatten()?;
            let pixeles = imagen::miniatura(&bytes, 96).ok()?;
            let color = ColorImage::from_rgba_unmultiplied([pixeles.ancho, pixeles.alto], &pixeles.rgba);
            let textura = contexto.load_texture(format!("miniatura_de_raza_{codigor}"), color, TextureOptions::LINEAR);
            self.miniaturas.insert(llave.clone(), textura);
        }
        self.miniaturas.get(&llave).map(|t| (t.id(), t.size_vec2()))
    }
}

/// Coincide si el texto es el código exacto o está en alguno de los datos que se muestran.
fn coincide(r: &Registro, dato: &str) -> bool {
    r.id.to_string() == dato || r.rotulos.iter().chain(r.extras.iter()).any(|t| sin_tildes(t).contains(dato))
}

/// La fila cumple todos los filtros elegidos que este usuario ve.
fn pasa_filtros(e: &Entidad, form: &FormRegistro, r: &Registro, administrador: bool) -> bool {
    e.filtros.iter().enumerate().all(|(i, filtro)| {
        let elegido = form.filtros.get(i).map(String::as_str).unwrap_or("");
        elegido.is_empty() || (filtro.solo_admin && !administrador) || r.dato(e, filtro.dato, false) == elegido
    })
}

/// Campos que este usuario ve: el productor no elige «Productor», siempre es él mismo.
fn visibles(e: &Entidad, administrador: bool) -> Vec<usize> {
    (0..e.campos.len()).filter(|&i| administrador || !e.campos[i].solo_admin).collect()
}

/// Texto que se muestra cuando una columna calculada no tiene dato.
fn vacio_de(titulo: &str) -> &'static str {
    match titulo {
        "Lotes asignados" => "Ninguno",
        "Corrales" => "Sin corrales",
        "Imagen" => "Sin imagen",
        _ => "",
    }
}

fn texto_de_columna(e: &Entidad, r: &Registro, columna: &crate::models::Columna) -> String {
    let texto = r.dato(e, columna.dato, true);
    if texto.is_empty() {
        vacio_de(columna.titulo).to_string()
    } else {
        texto
    }
}

pub fn pantalla(ui: &mut egui::Ui, estado: &mut AppState, cual: Cual) {
    let e = cual.entidad();
    let administrador = estado.es_administrador();
    let titulo = if administrador { e.plural } else { e.plural_propio };
    let accion = (!estado.solo_consulta(cual)).then_some("Crear");
    if cabecera(ui, titulo, accion) {
        estado.registro_crear(cual);
    }
    filtros(ui, estado, cual);
    if barra_de_consulta(ui, e.pista, &mut estado.registro_mut(cual).buscar) {
        estado.registro_consultar(cual);
    }

    let columnas: Vec<&crate::models::Columna> = e.lista.iter().filter(|c| administrador || !c.solo_admin).collect();
    let (mostradas, filas, elegida) = {
        let form = estado.registro(cual);
        let texto = sin_tildes(form.buscar.trim());
        let mostradas: Vec<usize> = (0..form.lista.len())
            .filter(|&i| pasa_filtros(e, form, &form.lista[i], administrador) && (texto.is_empty() || coincide(&form.lista[i], &texto)))
            .collect();
        let filas: Vec<Vec<String>> =
            mostradas.iter().map(|&i| columnas.iter().map(|c| texto_de_columna(e, &form.lista[i], c)).collect()).collect();
        let elegida = mostradas.iter().position(|&i| form.hoja && Some(form.lista[i].id) == form.id);
        (mostradas, filas, elegida)
    };
    let titulos: Vec<super::comun::Columna> = columnas.iter().map(|c| col(c.titulo, c.peso)).collect();
    let columna_estado = columnas.iter().position(|c| c.dato == Dato::Estado);
    let columna_imagen = columnas.iter().position(|c| c.dato == Dato::Campo("imagen"));

    let mut texturas: Vec<Option<(TextureId, Vec2)>> = Vec::new();
    if let Some(i_imagen) = columna_imagen.map(|_| e.indice("imagen")) {
        let filas_con_imagen: Vec<(i64, String)> =
            mostradas.iter().map(|&i| (estado.registro(cual).lista[i].id, estado.registro(cual).lista[i].valores[i_imagen].clone())).collect();
        for (codigo, descripcion) in filas_con_imagen {
            texturas.push(estado.miniatura_de_raza(ui.ctx(), codigo, &descripcion));
        }
    }
    let textura_de = |i: usize| texturas.get(i).copied().flatten();
    let miniaturas = columna_imagen.map(|columna| (columna, &textura_de as &dyn Fn(usize) -> Option<(TextureId, Vec2)>));

    let foto = estado.foto_actual();
    let claves: Vec<String> = mostradas.iter().map(|&i| estado.registro(cual).lista[i].id.to_string()).collect();
    let resaltar = |i: usize| foto.resaltado_fila(e.tabla, &claves[i]);
    let pulsada = tabla_con_miniaturas(ui, e.tabla, &titulos, &filas, elegida, columna_estado, &resaltar, miniaturas);
    if let Some(i) = pulsada {
        let registro = estado.registro(cual).lista[mostradas[i]].clone();
        estado.registro_mostrar(cual, &registro);
    }
}

/// Filtros de la lista, en una fila: «Todos» deja de filtrar.
fn filtros(ui: &mut egui::Ui, estado: &mut AppState, cual: Cual) {
    let e = cual.entidad();
    let administrador = estado.es_administrador();
    let visibles: Vec<usize> = (0..e.filtros.len()).filter(|&i| administrador || !e.filtros[i].solo_admin).collect();
    if visibles.is_empty() {
        return;
    }
    let mut cambiado = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 26.0;
        for &i in &visibles {
            let filtro = &e.filtros[i];
            let form = estado.registro_mut(cual);
            let sin_padre = filtro.depende.is_some_and(|k| form.filtros[k].is_empty());
            let vacio = if sin_padre { "Elija primero el departamento" } else { "Todos" };
            let opciones = form.opciones_filtro[i].clone();
            let FormRegistro { filtros, buscar_en_filtro, .. } = form;
            let buscar = (opciones.len() > 12).then_some(buscar_en_filtro);
            let id = format!("filtro_{}_{i}", e.tabla);
            if desplegable(ui, filtro.etiqueta, &id, 220.0, &mut filtros[i], &opciones, vacio, true, buscar) {
                cambiado = Some(i);
            }
        }
    });
    if let Some(i) = cambiado {
        estado.registro_cambiar_filtro(cual, i);
    }
    ui.add_space(6.0);
}

fn etiqueta_de(campo: &Campo) -> String {
    let pista = match (&campo.clase, campo.unidad) {
        (Clase::Fecha { .. }, _) => " (AAAA-MM-DD)".to_string(),
        (Clase::Telefono, _) => " (10 dígitos)".to_string(),
        (Clase::Codigo, _) => " (número entero)".to_string(),
        (_, "") => String::new(),
        (_, unidad) => format!(" ({unidad})"),
    };
    let opcional = if campo.obligatorio { "" } else { ", opcional" };
    format!("{}{pista}{opcional}", campo.etiqueta)
}

/// Valor guardado con su unidad, para la ficha: «20 cerdos», «110 kg», «15 %».
fn con_unidad(campo: &Campo, rotulo: &str) -> String {
    match (&campo.clase, campo.unidad) {
        (Clase::Entero { .. }, unidad) if !unidad.is_empty() && !rotulo.is_empty() => format!("{rotulo} {unidad}"),
        _ => rotulo.to_string(),
    }
}

fn subtitulo(ui: &mut egui::Ui, texto: &str) {
    ui.add_space(10.0);
    ui.label(RichText::new(texto).size(13.0).color(SALVIA).strong());
    ui.add_space(4.0);
}

fn nota(ui: &mut egui::Ui, texto: &str, color: egui::Color32) {
    ui.label(RichText::new(texto).size(13.5).color(color));
    ui.add_space(6.0);
}

fn ficha(ui: &mut egui::Ui, estado: &mut AppState, cual: Cual) {
    let e = cual.entidad();
    let administrador = estado.es_administrador();
    let solo_consulta = estado.solo_consulta(cual);
    let pregunta = estado.confirmar.as_ref().map(|c| c.texto.clone());
    let modo = estado.registro(cual).modo;
    let actual: Option<Registro> = {
        let form = estado.registro(cual);
        form.id.filter(|_| modo != Modo::Nuevo).and_then(|id| form.lista.iter().find(|r| r.id == id).cloned())
    };
    let rotulos = match &actual {
        Some(r) => r.rotulos.clone(),
        None => estado.registro(cual).valores.clone(),
    };

    let titulo = match modo {
        Modo::Nuevo => format!("{} {}", e.singular, if e.femenino { "nueva" } else { "nuevo" }),
        _ if e.titulo == 0 => format!("{} {}", e.singular, rotulos.first().cloned().unwrap_or_default()),
        _ => rotulos.get(e.titulo).cloned().unwrap_or_default(),
    };
    titulo_de_hoja(ui, &titulo);

    for i in visibles(e, administrador) {
        let campo = &e.campos[i];
        let escribe = match modo {
            Modo::Nuevo => true,
            Modo::Edicion => campo.cambia == Cambia::Siempre,
            Modo::Consulta => false,
        };
        if !escribe {
            match campo.clase {
                Clase::Imagen => mostrar_imagen(ui, estado),
                Clase::Municipio => {
                    let departamento = actual.as_ref().and_then(|r| r.extras.get(1)).cloned().unwrap_or_default();
                    ui.columns(2, |c| {
                        dato(&mut c[0], "Departamento", &departamento);
                        dato(&mut c[1], "Municipio", &rotulos[i]);
                    });
                }
                _ => dato(ui, campo.etiqueta, &con_unidad(campo, &rotulos[i])),
            }
            continue;
        }
        match &campo.clase {
            Clase::Opcion(_) => {
                let ancho = ui.available_width();
                let form = estado.registro_mut(cual);
                let opciones = como_texto(&form.opciones[i]);
                desplegable(ui, campo.etiqueta, campo.columna, ancho, &mut form.valores[i], &opciones, "Elija una opción", false, None);
            }
            Clase::Municipio => cascada(ui, estado, cual, i),
            Clase::Imagen => editor_de_imagen(ui, estado),
            _ => {
                campo_de_texto(ui, &etiqueta_de(campo), &mut estado.registro_mut(cual).valores[i]);
            }
        }
    }

    if modo == Modo::Consulta {
        if let Some(r) = &actual {
            datos_calculados(ui, estado, cual, r);
        }
    } else if cual == Cual::Lote {
        editor_de_corrales(ui, estado);
    }

    let form = estado.registro_mut(cual);
    match modo {
        Modo::Consulta => dato(ui, "Estado", e.nombre_de_estado(&form.estado)),
        Modo::Edicion => interruptor_con(ui, SOBRE_NIEBLA, &mut form.activo, true, e.nombre_de_estado(ACTIVO)),
        Modo::Nuevo => {}
    }

    ui.add_space(18.0);
    if solo_consulta {
        if enlace(ui, SALVIA, "Salir", true, 15.0) {
            estado.registro_salir_de_hoja(cual);
        }
        return;
    }
    if cual == Cual::Lote && modo == Modo::Consulta {
        let guardado = estado.registro(cual).estado.clone();
        if guardado == CERRADO {
            nota(ui, "El lote está cerrado: ya no admite cambios.", SALVIA);
            if enlace(ui, SALVIA, "Salir", true, 15.0) {
                estado.registro_salir_de_hoja(cual);
            }
            return;
        }
        if estado.registro(cual).cerrando.is_some() {
            return cierre_de_lote(ui, estado);
        }
    }
    let activo = estado.registro(cual).activo;
    match acciones(ui, modo, activo, pregunta.as_deref()) {
        Some(AccionHoja::Modificar) => estado.registro_modificar(cual),
        Some(AccionHoja::Inhabilitar) => estado.registro_pedir_inhabilitar(cual),
        Some(AccionHoja::Guardar) => estado.registro_guardar(cual),
        Some(AccionHoja::Confirmado) => estado.confirmado(),
        Some(AccionHoja::Volver) => estado.confirmar = None,
        Some(AccionHoja::Salir) => estado.registro_salir_de_hoja(cual),
        None => {}
    }
    let en_ceba = estado.registro(cual).estado == EN_CEBA;
    if cual == Cual::Lote && modo == Modo::Consulta && en_ceba && pregunta.is_none() {
        ui.add_space(14.0);
        if enlace(ui, PINO, "Cerrar lote", true, 15.0) {
            estado.lote_pedir_cierre();
        }
    }
}

/// Lo que se calcula al consultar: nunca se guarda.
fn datos_calculados(ui: &mut egui::Ui, estado: &AppState, cual: Cual, r: &Registro) {
    let extra = |i: usize| r.extras.get(i).cloned().unwrap_or_default();
    match cual {
        Cual::Granja => {
            ui.columns(2, |c| {
                dato(&mut c[0], "Corrales activos", &extra(2));
                dato(&mut c[1], "Lotes en ceba", &extra(3));
            });
        }
        Cual::Corral => {
            let capacidad: i64 = r.valores[Cual::Corral.entidad().indice("capacidad")].parse().unwrap_or(0);
            let ocupados: i64 = extra(1).parse().unwrap_or(0);
            let lotes = extra(0);
            dato(ui, "Lotes asignados", if lotes.is_empty() { "Ninguno" } else { &lotes });
            ui.columns(2, |c| {
                dato(&mut c[0], "Cerdos que aloja", &ocupados.to_string());
                dato(&mut c[1], "Espacios libres", &(capacidad - ocupados).max(0).to_string());
            });
        }
        Cual::Lote => {
            ui.columns(3, |c| {
                dato(&mut c[0], "Cerdos vivos", &extra(LOTE_VIVOS));
                dato(&mut c[1], "Vendidos", &extra(LOTE_VENDIDOS));
                dato(&mut c[2], "Muertos", &extra(LOTE_MUERTOS));
            });
            dato(ui, "Costo por kilo actual", &extra(LOTE_COSTO));
            if !extra(LOTE_CIERRE).is_empty() {
                dato(ui, "Fecha final real", &extra(LOTE_CIERRE));
            }
            subtitulo(ui, "Corrales asignados");
            let asignados = &estado.registro(cual).asignados;
            if asignados.is_empty() {
                nota(ui, "Sin corrales asignados.", PINO);
            }
            for a in asignados {
                nota(ui, &format!("{} · {} cerdos de {} · desde {}", a.nombre, a.cupos, a.capacidad, a.fecha), PINO);
            }
        }
        Cual::Raza => {}
    }
}

/// HU-02: primero el departamento y luego un municipio de ese departamento.
fn cascada(ui: &mut egui::Ui, estado: &mut AppState, cual: Cual, i: usize) {
    let ancho = ui.available_width();
    let cambio = {
        let form = estado.registro_mut(cual);
        let departamentos = form.departamentos.clone();
        let FormRegistro { departamento, buscar_departamento, .. } = form;
        desplegable(ui, "Departamento", "departamento_formulario", ancho, departamento, &departamentos, "Elija el departamento", false, Some(buscar_departamento))
    };
    if cambio {
        estado.granja_cambio_departamento(cual);
    }
    let form = estado.registro_mut(cual);
    let municipios = form.municipios.clone();
    let vacio = if form.departamento.is_empty() { "Elija primero el departamento" } else { "Elija el municipio" };
    let FormRegistro { valores, buscar_municipio, .. } = form;
    desplegable(ui, "Municipio", "municipio_formulario", ancho, &mut valores[i], &municipios, vacio, false, Some(buscar_municipio));
}

/// Imagen guardada o elegida, del ancho de la hoja.
fn mostrar_imagen(ui: &mut egui::Ui, estado: &mut AppState) {
    let imagen = estado.registro(Cual::Raza).imagen.clone();
    let vista = estado.vista_previa(ui.ctx());
    match (imagen, vista) {
        (Some(imagen), Some((textura, tamano))) => {
            ui.label(RichText::new("Imagen").size(12.5).color(SALVIA));
            ui.add_space(4.0);
            let escala = (ui.available_width() / tamano.x).min(230.0 / tamano.y).min(1.0);
            ui.add(egui::Image::new((textura, tamano * escala)).rounding(6.0));
            ui.label(RichText::new(imagen.descripcion()).size(12.5).color(SALVIA));
            ui.add_space(10.0);
        }
        (Some(_), None) => dato(ui, "Imagen", "No se pudo mostrar la imagen guardada."),
        (None, _) => dato(ui, "Imagen", "Sin imagen"),
    }
}

/// HU-04: la imagen se elige con el diálogo de Windows o se arrastra a la ventana.
fn editor_de_imagen(ui: &mut egui::Ui, estado: &mut AppState) {
    let soltado = ui.ctx().input(|i| i.raw.dropped_files.iter().find_map(|f| f.path.clone()));
    if let Some(ruta) = soltado {
        estado.raza_cargar_archivo(&ruta);
    }
    let encima = ui.ctx().input(|i| !i.raw.hovered_files.is_empty());
    mostrar_imagen(ui, estado);
    let tiene_imagen = estado.registro(Cual::Raza).imagen.is_some();
    let (mut elegir, mut quitar) = (false, false);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 22.0;
        elegir = enlace(ui, PINO, if tiene_imagen { "Cambiar imagen" } else { "Elegir imagen" }, true, 15.0);
        if tiene_imagen {
            quitar = enlace(ui, ARCILLA, "Quitar imagen", true, 15.0);
        }
    });
    let pista = if encima { "Suelte la imagen para cargarla." } else { "JPG o PNG de máximo 2 MB. También puede arrastrar el archivo a esta ventana." };
    nota(ui, pista, if encima { PINO } else { SALVIA });
    if elegir {
        estado.raza_elegir_imagen();
    }
    if quitar {
        estado.raza_quitar_imagen();
    }
}

/// HU-05: corrales activos de la granja del lote, con los cerdos que van en cada uno.
fn editor_de_corrales(ui: &mut egui::Ui, estado: &mut AppState) {
    estado.lote_cargar_corrales();
    let e = Cual::Lote.entidad();
    subtitulo(ui, "Corrales del lote");
    let form = estado.registro(Cual::Lote);
    if form.valores[e.indice("codigo_granja")].is_empty() {
        return nota(ui, "Elija la granja para ver sus corrales.", SALVIA);
    }
    if form.corrales.is_empty() {
        return nota(ui, "La granja no tiene corrales activos. Créelos en Corrales.", ARCILLA);
    }
    let mut recien = Vec::new();
    let cuantos = form.corrales.len();
    for k in 0..cuantos {
        let form = estado.registro_mut(Cual::Lote);
        let fila = &mut form.corrales[k];
        let antes = fila.elegido;
        ui.horizontal(|ui| {
            riel(ui, SOBRE_NIEBLA, &mut fila.elegido, true, LINEA);
            ui.add_space(6.0);
            ui.vertical(|ui| {
                ui.label(RichText::new(format!("{} · código {}", fila.corral.nombre, fila.corral.codigo)).size(15.0).color(PINO));
                let detalle = format!("capacidad {} · libres {}", fila.corral.capacidad, fila.corral.libres());
                ui.label(RichText::new(detalle).size(12.5).color(SALVIA));
            });
            if fila.elegido {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(RichText::new("cerdos").size(13.0).color(SALVIA));
                    campo_corto(ui, &mut fila.cupos, 54.0, &format!("cupos_{}", fila.corral.codigo));
                });
            }
        });
        ui.add_space(6.0);
        if fila.elegido && !antes {
            recien.push(k);
        }
    }
    for k in recien {
        estado.lote_proponer_cupos(k);
    }

    let form = estado.registro(Cual::Lote);
    let cantidad = form.valores[e.indice("cantidad")].trim().parse::<i64>().ok();
    let elegidos: Vec<&CorralEnFormulario> = form.corrales.iter().filter(|c| c.elegido).collect();
    let capacidad: i64 = elegidos.iter().map(|c| c.corral.capacidad).sum();
    let ubicados: i64 = elegidos.iter().filter_map(|c| c.cupos.trim().parse::<i64>().ok()).sum();
    let de = cantidad.map(|n| format!(" de {n}")).unwrap_or_default();
    nota(ui, &format!("Capacidad de los corrales elegidos: {capacidad} · Cerdos ubicados: {ubicados}{de}"), SALVIA);
    if let Some(n) = cantidad.filter(|_| !elegidos.is_empty()) {
        if capacidad < n {
            nota(ui, &format!("Los corrales elegidos tienen capacidad para {capacidad} cerdos y el lote tiene {n}."), ARCILLA);
        } else if ubicados < n {
            nota(ui, &format!("Faltan {} cerdos por ubicar.", n - ubicados), ARCILLA);
        }
    }
}

/// HU-05: cierre del lote con su fecha final real.
fn cierre_de_lote(ui: &mut egui::Ui, estado: &mut AppState) {
    subtitulo(ui, "Cerrar el lote");
    nota(ui, "Un lote se cierra cuando todos sus cerdos están vendidos o muertos; después no admite gastos ni ventas.", SALVIA);
    if let Some(fecha) = estado.registro_mut(Cual::Lote).cerrando.as_mut() {
        campo_de_texto(ui, "Fecha final real (AAAA-MM-DD)", fecha);
    }
    ui.add_space(8.0);
    let (mut cerrar, mut volver) = (false, false);
    ui.horizontal(|ui| {
        cerrar = boton(ui, SOBRE_NIEBLA, "Cerrar lote", true);
        ui.add_space(10.0);
        volver = enlace(ui, SALVIA, "Volver", true, 15.0);
    });
    if cerrar {
        estado.lote_cerrar();
    }
    if volver {
        estado.registro_mut(Cual::Lote).cerrando = None;
    }
}

fn campo_de_texto(ui: &mut egui::Ui, etiqueta: &str, valor: &mut String) {
    campo(ui, SOBRE_NIEBLA, etiqueta, valor, true);
}

pub fn hoja_lateral(contexto: &egui::Context, estado: &mut AppState, cual: Cual) {
    let abierta = estado.registro(cual).hoja;
    let nombre = format!("hoja_{}", cual.entidad().tabla);
    let cerrar = hoja(contexto, &nombre, abierta, |ui| ficha(ui, estado, cual));
    if abierta && cerrar && estado.registro(cual).modo == Modo::Consulta && estado.registro(cual).cerrando.is_none() {
        estado.registro_salir_de_hoja(cual);
    }
}
