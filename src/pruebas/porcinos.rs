//! Pruebas del Parcial II con los requisitos aprobados (HU-01 a HU-05). Usan las mismas funciones
//! de los botones contra una base SQLite real; cada prueba nombra la historia que ejercita.

use super::*;
use crate::imagen;
use crate::validaciones;

const PNG: &[u8] = include_bytes!("raza_prueba.png");
const JPG: &[u8] = include_bytes!("raza_prueba.jpg");

fn hoy() -> String {
    validaciones::fecha_texto(crate::reloj::ahora().fecha)
}

fn productor_de_prueba(documento: &str, nombre: &str, correo: &str) -> DatosPersona {
    DatosPersona {
        identificacion: documento.into(),
        nom1: nombre.into(),
        apell1: "Peñaranda".into(),
        movil: "3157778899".into(),
        direccion: "Vereda Agua Linda".into(),
        fecha_nac: "1980-06-15".into(),
        correo: correo.into(),
        ..Default::default()
    }
}

/// Pulsa Crear en Productores, llena el formulario y pulsa Guardar.
fn crear_productor(e: &mut AppState, d: DatosPersona) -> bool {
    e.ir_a(Pantalla::Productor);
    e.productor_crear();
    e.productor.datos = d;
    e.productor_guardar();
    tipo(e) == Some(TipoMensaje::Exito)
}

fn llenar(e: &mut AppState, cual: Cual, datos: &[(&str, &str)]) {
    for (columna, valor) in datos {
        let i = cual.entidad().indice(columna);
        e.registro_mut(cual).valores[i] = (*valor).to_string();
    }
}

fn guardado(e: &AppState) -> bool {
    matches!(tipo(e), Some(TipoMensaje::Exito) | Some(TipoMensaje::Advertencia))
}

/// Pulsa Crear, llena la ficha y pulsa Guardar. Devuelve el código si la base lo aceptó.
fn crear(e: &mut AppState, cual: Cual, datos: &[(&str, &str)]) -> Option<i64> {
    e.ir_a(Pantalla::Gestion(cual));
    e.registro_crear(cual);
    llenar(e, cual, datos);
    e.registro_guardar(cual);
    guardado(e).then(|| e.registro(cual).id).flatten()
}

/// Elige el departamento y luego el municipio, como en la pantalla.
fn elegir_municipio(e: &mut AppState, departamento: &str, municipio: &str) {
    e.registro_mut(Cual::Granja).departamento = departamento.into();
    e.granja_cambio_departamento(Cual::Granja);
    let i = Cual::Granja.entidad().indice("idmunicipio");
    e.registro_mut(Cual::Granja).valores[i] = municipio.into();
}

fn crear_granja(e: &mut AppState, codigo: &str, productor: &str, nombre: &str, departamento: &str, municipio: &str) -> Option<i64> {
    e.ir_a(Pantalla::Gestion(Cual::Granja));
    e.registro_crear(Cual::Granja);
    llenar(e, Cual::Granja, &[("codigo", codigo), ("idproductor", productor), ("nombre", nombre), ("vereda", "La Floresta")]);
    elegir_municipio(e, departamento, municipio);
    e.registro_guardar(Cual::Granja);
    guardado(e).then(|| e.registro(Cual::Granja).id).flatten()
}

fn crear_corral(e: &mut AppState, codigo: &str, granja: &str, nombre: &str, capacidad: &str) -> Option<i64> {
    crear(e, Cual::Corral, &[("codigo", codigo), ("codigo_granja", granja), ("nombre", nombre), ("capacidad", capacidad)])
}

/// Abre un lote nuevo con sus datos y carga los corrales de su granja, sin guardarlo.
fn preparar_lote(e: &mut AppState, codigo: &str, granja: &str, cantidad: &str) {
    e.ir_a(Pantalla::Gestion(Cual::Lote));
    e.registro_crear(Cual::Lote);
    llenar(
        e,
        Cual::Lote,
        &[
            ("codigol", codigo),
            ("codigo_granja", granja),
            ("fecha_inicial", "2026-09-01"),
            ("fecha_final", "2027-01-15"),
            ("cantidad", cantidad),
            ("peso_objetivo", "110"),
        ],
    );
    e.lote_cargar_corrales();
}

fn elegir_corral(e: &mut AppState, corral: i64, cupos: &str) {
    let fila = e.registro_mut(Cual::Lote).corrales.iter_mut().find(|c| c.corral.codigo == corral).expect("el corral está en la lista");
    fila.elegido = true;
    fila.cupos = cupos.into();
}

fn contar(e: &AppState, sql: &str) -> i64 {
    e.db.conexion.query_row(sql, [], |f| f.get(0)).unwrap()
}

fn rechaza(e: &AppState, sql: &str) -> bool {
    e.db.conexion.execute(sql, []).is_err()
}

fn abrir(e: &mut AppState, cual: Cual, id: i64) {
    e.ir_a(Pantalla::Gestion(cual));
    let registro = e.db.registro_por_id(cual.entidad(), id, e.persona_del_filtro()).unwrap().expect("el registro existe");
    e.registro_mostrar(cual, &registro);
}

/// Un productor (88123456) con la granja 100 en Cúcuta y sus corrales 11 (capacidad 20) y 12 (15).
fn escenario(e: &mut AppState) {
    assert!(crear_productor(e, productor_de_prueba("88123456", "Ramiro", "")), "{}", texto(e));
    assert_eq!(crear_granja(e, "100", "88123456", "La Esperanza", "54", "54001"), Some(100), "{}", texto(e));
    assert_eq!(crear_corral(e, "11", "100", "Corral 1", "20"), Some(11), "{}", texto(e));
    assert_eq!(crear_corral(e, "12", "100", "Corral 2", "15"), Some(12), "{}", texto(e));
}

#[test]
fn la_migracion_crea_el_modelo_y_la_divipola_sin_duplicar() {
    let t = con_admin();
    let e = &t.e;
    let tablas: Vec<String> = e
        .db
        .conexion
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |f| f.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tablas, ["corral", "departamento", "granja", "lote", "lote_corral", "municipio", "perfil", "persona", "productor", "raza", "usuario"]);
    assert_eq!(contar(e, "SELECT COUNT(*) FROM departamento"), 33);
    assert_eq!(contar(e, "SELECT COUNT(*) FROM municipio"), 1122);
    assert_eq!(contar(e, "SELECT COUNT(*) FROM municipio WHERE iddepartamento = 54"), 40, "Norte de Santander");
    let cucuta: String = e.db.conexion.query_row("SELECT nombre FROM municipio WHERE idmunicipio = 54001", [], |f| f.get(0)).unwrap();
    assert_eq!(cucuta, "San José de Cúcuta");
    assert_eq!(contar(e, "SELECT \"notnull\" FROM pragma_table_info('persona') WHERE name = 'correo'"), 0, "HU-01: el correo es opcional");
    assert!(contar(e, "SELECT COUNT(*) FROM sqlite_master WHERE type = 'trigger'") >= 14);
    let perfil: String = e.db.conexion.query_row("SELECT descripc FROM perfil WHERE idperfil = 2", [], |f| f.get(0)).unwrap();
    assert_eq!(perfil, "Productor");

    let otra = Database::abrir(&e.db.ruta).unwrap();
    let municipios: i64 = otra.conexion.query_row("SELECT COUNT(*) FROM municipio", [], |f| f.get(0)).unwrap();
    assert_eq!(municipios, 1122, "abrir otra vez la base no duplica el catálogo");
}

#[test]
fn la_base_del_parcial_i_pasa_al_correo_opcional_sin_perder_filas() {
    let carpeta = std::env::temp_dir().join(format!("farming6-migracion-{}-{}", std::process::id(), CONTADOR.fetch_add(1, Ordering::SeqCst)));
    std::fs::create_dir_all(&carpeta).unwrap();
    let _limpieza = Limpieza(carpeta.clone());
    let ruta = carpeta.join("parcial1.db");
    {
        let vieja = rusqlite::Connection::open(&ruta).unwrap();
        vieja.execute_batch(include_str!("../../migrations/001_init.sql")).unwrap();
        vieja
            .execute_batch(
                "INSERT INTO persona (idpersona, nom1, apell1, movil, correo) VALUES (101010, 'Campesino', 'Campesino', '3234441122', 'campesino@ejemplo.com');
                 INSERT INTO usuario VALUES (101010, 'Campesino', 'hash', 2, 'A');",
            )
            .unwrap();
        let exige: i64 = vieja.query_row("SELECT \"notnull\" FROM pragma_table_info('persona') WHERE name = 'correo'", [], |f| f.get(0)).unwrap();
        assert_eq!(exige, 1, "la base del Parcial I exigía el correo");
    }
    let base = Database::abrir(&ruta).unwrap();
    let persona = base.persona_por_id(101_010).unwrap().expect("la persona sigue");
    assert_eq!(persona.correo, "campesino@ejemplo.com");
    assert!(base.usuario_por_id(101_010).unwrap().is_some());
    let huerfanas: i64 = base.conexion.query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |f| f.get(0)).unwrap();
    assert_eq!(huerfanas, 0, "usuario sigue apuntando a su persona");
    let repetido = "INSERT INTO persona (idpersona, nom1, apell1, movil, correo) VALUES (7, 'Otra', 'Persona', '3000000000', 'CAMPESINO@ejemplo.com')";
    assert!(base.conexion.execute(repetido, []).is_err(), "el correo sigue siendo único");
    base.conexion.execute("INSERT INTO persona (idpersona, nom1, apell1, movil) VALUES (8, 'Sin', 'Correo', '3000000001')", []).unwrap();
    base.conexion.execute("INSERT INTO persona (idpersona, nom1, apell1, movil) VALUES (9, 'Otro', 'Sin', '3000000002')", []).unwrap();
}

#[test]
fn el_menu_del_administrador_tiene_todas_las_opciones() {
    let t = con_admin();
    let menu: Vec<&str> = crate::ui::opciones_del_menu(&t.e).iter().map(|(texto, _)| *texto).collect();
    assert_eq!(menu, ["Panel", "Personas", "Usuarios", "Perfiles", "Productores", "Granjas", "Corrales", "Razas", "Lotes", "Base de datos"]);
}

#[test]
fn hu01_crud_de_productor_con_sus_validaciones() {
    let mut t = con_admin();
    let e = &mut t.e;
    assert!(!crear_productor(e, DatosPersona::default()));
    assert!(texto(e).contains("documento es obligatorio"), "{}", texto(e));

    let anio_menor = crate::reloj::ahora().fecha.anio - 17;
    let menor = DatosPersona { fecha_nac: format!("{anio_menor}-01-01"), ..productor_de_prueba("88123456", "Ramiro", "") };
    assert!(!crear_productor(e, menor));
    assert!(texto(e).contains("mayor de 18"), "{}", texto(e));

    let telefono_corto = DatosPersona { movil: "315777889".into(), ..productor_de_prueba("88123456", "Ramiro", "") };
    assert!(!crear_productor(e, telefono_corto));
    assert!(texto(e).contains("10 dígitos"), "{}", texto(e));

    assert!(crear_productor(e, productor_de_prueba("88123456", "Ramiro", "")), "{}", texto(e));
    let guardada = e.db.persona_por_id(88_123_456).unwrap().unwrap();
    assert_eq!((guardada.correo.as_str(), guardada.estado.as_str()), ("", ACTIVO), "queda activo y sin correo");
    assert_eq!(contar(e, "SELECT COUNT(*) FROM persona WHERE idpersona = 88123456 AND correo IS NULL"), 1, "el correo vacío se guarda nulo");
    assert_eq!(e.db.productor_por_id(88_123_456).unwrap().unwrap().fecha_registro, hoy());

    assert!(!crear_productor(e, productor_de_prueba("88123456", "Otro", "")));
    assert!(texto(e).contains("Ya existe un productor con ese documento"), "{}", texto(e));
    assert!(!crear_productor(e, productor_de_prueba("77123456", "Lucía", "admin@ejemplo.com")));
    assert!(texto(e).contains("correo ya está registrado"), "{}", texto(e));

    // Consultar por documento, nombre o apellido, sin importar las tildes.
    e.ir_a(Pantalla::Productor);
    e.productor.buscar = "penaranda".into();
    e.productor_consultar();
    assert_eq!(e.productor.id, Some(88_123_456), "{}", texto(e));
    assert_eq!(e.productor.lista[0].granjas, 0);
    e.productor.buscar = "88123".into();
    e.productor_consultar();
    assert_eq!(e.productor.id, Some(88_123_456), "{}", texto(e));

    // Modificar: todo menos el documento, que es la llave primaria.
    e.productor_modificar();
    e.productor.datos.direccion = "Vereda El Rodeo".into();
    e.productor.datos.identificacion = "99999999".into();
    e.productor_guardar();
    assert_eq!(tipo(e), Some(TipoMensaje::Exito), "{}", texto(e));
    assert_eq!(e.db.persona_por_id(88_123_456).unwrap().unwrap().direccion, "Vereda El Rodeo");
    assert!(e.db.persona_por_id(99_999_999).unwrap().is_none(), "el documento no cambia");

    // Inhabilitar es un borrado lógico y le impide iniciar sesión.
    crear_usuario_para(e, 88_123_456, "ramiro", "Campo2026", 2);
    e.ir_a(Pantalla::Productor);
    let vista = e.productor.lista.iter().find(|v| v.persona.idpersona == 88_123_456).unwrap().clone();
    e.productor_mostrar(&vista);
    e.productor_pedir_inhabilitar();
    assert!(e.confirmar.is_some(), "pide confirmación");
    e.confirmado();
    assert_eq!(e.db.persona_por_id(88_123_456).unwrap().unwrap().estado, INACTIVO);
    assert!(e.db.es_productor(88_123_456).unwrap(), "el registro se conserva");
    e.cerrar_sesion();
    ingresar(e, "ramiro", "Campo2026");
    assert!(texto(e).contains("inhabilitado"), "{}", texto(e));
    assert!(e.sesion.is_none(), "un productor inactivo no puede iniciar sesión");
}

#[test]
fn hu01_una_persona_ya_registrada_pasa_a_ser_productor() {
    let mut t = con_admin();
    let e = &mut t.e;
    guardar_persona(e, datos("70111222", "Pedro", "pedro@ejemplo.com"));
    assert!(!e.db.es_productor(70_111_222).unwrap());
    let d = DatosPersona {
        identificacion: "70111222".into(),
        nom1: "Pedro".into(),
        apell1: "Rojas".into(),
        movil: "6075712345".into(),
        direccion: "Vereda La Garita".into(),
        fecha_nac: "1990-05-20".into(),
        correo: "pedro@ejemplo.com".into(),
        ..Default::default()
    };
    assert!(crear_productor(e, d), "{}", texto(e));
    assert!(texto(e).contains("ya estaba registrada"), "{}", texto(e));
    assert!(e.db.es_productor(70_111_222).unwrap());
    let p = e.db.persona_por_id(70_111_222).unwrap().unwrap();
    assert_eq!((p.movil.as_str(), p.direccion.as_str()), ("6075712345", "Vereda La Garita"), "un fijo de 10 dígitos también vale");
}

#[test]
fn hu01_el_productor_modifica_sus_datos_sin_documento_ni_estado() {
    let mut t = con_admin();
    let e = &mut t.e;
    assert!(crear_productor(e, productor_de_prueba("88123456", "Ramiro", "ramiro@ejemplo.com")), "{}", texto(e));
    crear_usuario_para(e, 88_123_456, "ramiro", "Campo2026", 2);
    e.cerrar_sesion();
    ingresar(e, "ramiro", "Campo2026");
    assert!(e.es_productor() && !e.es_administrador(), "{}", texto(e));
    let menu: Vec<&str> = crate::ui::opciones_del_menu(e).iter().map(|(texto, _)| *texto).collect();
    assert_eq!(menu, ["Panel", "Mis datos", "Mis granjas", "Corrales", "Lotes", "Razas"]);

    e.ir_a(Pantalla::Persona);
    assert!(e.persona.es_productor, "«Mis datos» usa las reglas del productor");
    e.persona_modificar();
    e.persona.datos.correo.clear();
    e.persona.datos.direccion = "Finca Los Naranjos".into();
    e.persona.activo = false;
    e.persona_guardar();
    assert!(texto(e).contains("no puede inhabilitar"), "el productor no cambia su estado: {}", texto(e));
    e.persona.activo = true;
    e.persona_guardar();
    assert_eq!(tipo(e), Some(TipoMensaje::Exito), "{}", texto(e));
    let p = e.db.persona_por_id(88_123_456).unwrap().unwrap();
    assert_eq!((p.correo.as_str(), p.direccion.as_str(), p.estado.as_str()), ("", "Finca Los Naranjos", ACTIVO));

    e.ir_a(Pantalla::Productor);
    assert_ne!(e.pantalla, Pantalla::Productor, "solo el Administrador gestiona productores");
}

#[test]
fn hu02_crud_de_granja_con_departamento_municipio_y_validaciones() {
    let mut t = con_admin();
    let e = &mut t.e;
    assert!(crear_productor(e, productor_de_prueba("88123456", "Ramiro", "")));
    let ent = Cual::Granja.entidad();

    assert_eq!(crear(e, Cual::Granja, &[]), None);
    assert!(texto(e).contains("Código"), "{}", texto(e));
    assert_eq!(crear_granja(e, "0", "88123456", "La Esperanza", "54", "54001"), None);
    assert!(texto(e).contains("número entero"), "{}", texto(e));

    e.ir_a(Pantalla::Gestion(Cual::Granja));
    e.registro_crear(Cual::Granja);
    llenar(e, Cual::Granja, &[("codigo", "100"), ("idproductor", "88123456"), ("nombre", "La Esperanza"), ("vereda", "El Pórtico")]);
    e.registro_guardar(Cual::Granja);
    assert!(texto(e).contains("Municipio"), "sin municipio: {}", texto(e));
    elegir_municipio(e, "54", "5001");
    e.registro_guardar(Cual::Granja);
    assert!(texto(e).contains("Elija «Municipio» de la lista"), "Medellín no es de Norte de Santander: {}", texto(e));
    elegir_municipio(e, "54", "54001");
    llenar(e, Cual::Granja, &[("telefono", "30012")]);
    e.registro_guardar(Cual::Granja);
    assert!(texto(e).contains("10 dígitos"), "{}", texto(e));
    llenar(e, Cual::Granja, &[("telefono", "3001234567"), ("correo", "granja@")]);
    e.registro_guardar(Cual::Granja);
    assert!(texto(e).contains("correo"), "{}", texto(e));
    llenar(e, Cual::Granja, &[("correo", "")]);
    e.registro_guardar(Cual::Granja);
    assert!(guardado(e), "{}", texto(e));

    let fila = e.db.registro_por_id(ent, 100, None).unwrap().unwrap();
    assert_eq!(fila.estado, ACTIVO, "queda activa");
    assert_eq!(fila.rotulos[ent.indice("idmunicipio")], "San José de Cúcuta");
    assert_eq!(fila.extras[1], "Norte de Santander", "el departamento sale del municipio");
    assert_eq!(fila.rotulos[ent.indice("idproductor")], "Ramiro Peñaranda", "asociada al documento del productor");
    assert_eq!((fila.extras[2].as_str(), fila.extras[3].as_str()), ("0", "0"), "corrales y lotes en ceba");

    assert_eq!(crear_granja(e, "100", "88123456", "Otra", "54", "54405"), None);
    assert!(texto(e).contains("Ya existe una granja con ese código"), "{}", texto(e));

    // Modificar: todos los datos menos el código.
    abrir(e, Cual::Granja, 100);
    e.registro_modificar(Cual::Granja);
    assert_eq!(e.registro(Cual::Granja).departamento, "54", "al modificar vuelve el departamento de su municipio");
    elegir_municipio(e, "54", "54405");
    llenar(e, Cual::Granja, &[("codigo", "555"), ("vereda", "La Garita")]);
    e.registro_guardar(Cual::Granja);
    assert!(guardado(e), "{}", texto(e));
    let fila = e.db.registro_por_id(ent, 100, None).unwrap().unwrap();
    assert_eq!((fila.rotulos[ent.indice("idmunicipio")].as_str(), fila.valores[ent.indice("vereda")].as_str()), ("Los Patios", "La Garita"));
    assert!(e.db.registro_por_id(ent, 555, None).unwrap().is_none(), "el código no cambia");
}

#[test]
fn hu02_el_productor_ve_solo_sus_granjas_y_el_administrador_filtra() {
    let mut t = con_admin();
    let e = &mut t.e;
    assert!(crear_productor(e, productor_de_prueba("88123456", "Ramiro", "")));
    assert!(crear_productor(e, productor_de_prueba("77123456", "Lucía", "")));
    assert_eq!(crear_granja(e, "100", "88123456", "La Esperanza", "54", "54001"), Some(100), "{}", texto(e));
    assert_eq!(crear_granja(e, "200", "77123456", "Villa Luz", "5", "5001"), Some(200), "{}", texto(e));
    crear_usuario_para(e, 88_123_456, "ramiro", "Campo2026", 2);

    e.ir_a(Pantalla::Gestion(Cual::Granja));
    assert_eq!(e.registro(Cual::Granja).opciones_filtro[0].len(), 33, "departamentos");
    assert!(e.registro(Cual::Granja).opciones_filtro[1].is_empty(), "sin departamento no hay municipios");
    e.registro_mut(Cual::Granja).filtros[0] = "54".into();
    e.registro_cambiar_filtro(Cual::Granja, 0);
    assert_eq!(e.registro(Cual::Granja).opciones_filtro[1].len(), 40, "municipios de Norte de Santander");
    e.registro_consultar(Cual::Granja);
    assert!(texto(e).contains("Granjas en la lista: 1"), "{}", texto(e));
    e.registro_mut(Cual::Granja).filtros[1] = "54405".into();
    e.registro_consultar(Cual::Granja);
    assert!(texto(e).contains("Granjas en la lista: 0"), "{}", texto(e));
    e.registro_mut(Cual::Granja).filtros = vec![String::new(), String::new(), "77123456".into()];
    e.registro_consultar(Cual::Granja);
    assert!(texto(e).contains("Granjas en la lista: 1"), "{}", texto(e));

    e.cerrar_sesion();
    ingresar(e, "ramiro", "Campo2026");
    e.ir_a(Pantalla::Gestion(Cual::Granja));
    assert_eq!(e.registro(Cual::Granja).lista.len(), 1, "«Mis granjas»: solo las suyas");
    assert_eq!(crear_granja(e, "300", "77123456", "El Recreo", "54", "54261"), Some(300), "{}", texto(e));
    let ent = Cual::Granja.entidad();
    let fila = e.db.registro_por_id(ent, 300, None).unwrap().unwrap();
    assert_eq!(fila.valores[ent.indice("idproductor")], "88123456", "queda asociada a su documento");
    assert!(e.db.registro_por_id(ent, 200, e.persona_del_filtro()).unwrap().is_none());
    e.registro_inhabilitar(Cual::Granja, 200);
    assert!(texto(e).contains("no es suyo"), "{}", texto(e));
}

#[test]
fn hu03_crud_de_corral_con_capacidad_y_nombre_unico_en_la_granja() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    assert_eq!(crear_granja(e, "200", "88123456", "Villa Luz", "54", "54405"), Some(200), "{}", texto(e));
    assert_eq!(crear_corral(e, "13", "100", "Corral 3", "0"), None);
    assert!(texto(e).contains("entre 1 y 10000"), "{}", texto(e));
    assert_eq!(crear_corral(e, "13", "100", "Corral 3", "doce"), None);
    assert!(texto(e).contains("números enteros"), "{}", texto(e));
    assert_eq!(crear_corral(e, "13", "100", "corral 1", "10"), None);
    assert!(texto(e).contains("ya tiene un corral con ese nombre"), "{}", texto(e));
    assert_eq!(crear_corral(e, "21", "200", "Corral 1", "10"), Some(21), "en otra granja el nombre sí se puede: {}", texto(e));
    assert_eq!(crear_corral(e, "11", "200", "Corral 9", "10"), None);
    assert!(texto(e).contains("Ya existe un corral con ese código"), "{}", texto(e));

    e.ir_a(Pantalla::Gestion(Cual::Corral));
    e.registro_mut(Cual::Corral).filtros[0] = "100".into();
    e.registro_consultar(Cual::Corral);
    assert!(texto(e).contains("Corrales en la lista: 2"), "los corrales de una granja: {}", texto(e));
    let fila = e.db.registro_por_id(Cual::Corral.entidad(), 11, None).unwrap().unwrap();
    assert_eq!((fila.extras[0].as_str(), fila.extras[1].as_str()), ("", "0"), "sin lotes ni cerdos");
    assert_eq!(fila.rotulos[1], "La Esperanza");
    assert_eq!(fila.estado, ACTIVO);
}

#[test]
fn hu03_los_cerdos_del_corral_limitan_su_capacidad_y_su_inhabilitacion() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    preparar_lote(e, "1", "100", "30");
    elegir_corral(e, 11, "20");
    elegir_corral(e, 12, "10");
    e.registro_guardar(Cual::Lote);
    assert_eq!(tipo(e), Some(TipoMensaje::Exito), "{}", texto(e));
    let corral = Cual::Corral.entidad();
    let fila = e.db.registro_por_id(corral, 11, None).unwrap().unwrap();
    assert_eq!((fila.extras[0].as_str(), fila.extras[1].as_str()), ("1", "20"), "lotes asignados y cerdos que aloja");

    abrir(e, Cual::Corral, 11);
    e.registro_modificar(Cual::Corral);
    llenar(e, Cual::Corral, &[("capacidad", "15")]);
    e.registro_guardar(Cual::Corral);
    assert!(texto(e).contains("aloja 20 cerdos"), "{}", texto(e));
    llenar(e, Cual::Corral, &[("capacidad", "25")]);
    e.registro_guardar(Cual::Corral);
    assert!(guardado(e), "{}", texto(e));

    abrir(e, Cual::Corral, 11);
    e.registro_pedir_inhabilitar(Cual::Corral);
    assert!(texto(e).contains("aloja 20 cerdo(s)"), "{}", texto(e));
    assert!(e.confirmar.is_none());
    e.ir_a(Pantalla::Gestion(Cual::Granja));
    e.registro_inhabilitar(Cual::Granja, 100);
    assert!(texto(e).contains("lote(s) en ceba"), "HU-02: {}", texto(e));

    // Inhabilitar el lote libera sus corrales; entonces sí se inhabilitan corral y granja.
    e.ir_a(Pantalla::Gestion(Cual::Lote));
    e.registro_inhabilitar(Cual::Lote, 1);
    assert_eq!(contar(e, "SELECT COUNT(*) FROM lote_corral WHERE codigol = 1 AND estado = 'A'"), 0);
    e.ir_a(Pantalla::Gestion(Cual::Corral));
    e.registro_inhabilitar(Cual::Corral, 11);
    assert!(guardado(e), "{}", texto(e));
    e.ir_a(Pantalla::Gestion(Cual::Granja));
    e.registro_inhabilitar(Cual::Granja, 100);
    assert!(guardado(e), "{}", texto(e));
    assert_eq!(contar(e, "SELECT COUNT(*) FROM granja WHERE codigo = 100"), 1, "el borrado es lógico");
}

#[test]
fn hu04_crud_de_raza_con_imagen_jpg_o_png_de_maximo_2_mb() {
    let mut t = con_admin();
    let e = &mut t.e;
    let carpeta = e.carpeta.clone();
    let escribir = |nombre: &str, bytes: &[u8]| {
        let ruta = carpeta.join(nombre);
        std::fs::write(&ruta, bytes).unwrap();
        ruta
    };
    let png = escribir("landrace.png", PNG);
    let jpg = escribir("duroc.jpg", JPG);
    let gif = escribir("raza.gif", b"GIF89a esto no sirve");
    let falso = escribir("falso.png", b"\x89PNG\r\n\x1a\nesto no es una imagen");
    let mut grande = PNG.to_vec();
    grande.resize(imagen::MAXIMO_BYTES + 1, 0);
    let pesado = escribir("pesado.png", &grande);
    let ent = Cual::Raza.entidad();

    e.ir_a(Pantalla::Gestion(Cual::Raza));
    e.registro_crear(Cual::Raza);
    llenar(e, Cual::Raza, &[("codigor", "1"), ("descripcion", "Landrace")]);
    e.raza_cargar_archivo(&gif);
    assert!(texto(e).contains(".jpg, .jpeg o .png"), "{}", texto(e));
    e.raza_cargar_archivo(&pesado);
    assert!(texto(e).contains("máximo es 2 MB"), "{}", texto(e));
    e.raza_cargar_archivo(&falso);
    assert!(texto(e).contains("dañado"), "{}", texto(e));
    assert!(e.registro(Cual::Raza).imagen.is_none());
    e.raza_cargar_archivo(&png);
    assert_eq!(e.registro(Cual::Raza).imagen.as_ref().map(|i| i.formato), Some(imagen::Formato::Png), "{}", texto(e));
    e.registro_guardar(Cual::Raza);
    assert!(guardado(e), "{}", texto(e));
    assert_eq!(e.db.imagen_de_raza(1).unwrap().as_deref(), Some(PNG), "la imagen queda en la base");
    let fila = e.db.registro_por_id(ent, 1, None).unwrap().unwrap();
    assert_eq!(fila.rotulos[ent.indice("imagen")], "PNG · 1 KB");
    assert_eq!(fila.estado, ACTIVO);

    assert_eq!(crear(e, Cual::Raza, &[("codigor", "2"), ("descripcion", "Criollo")]), Some(2), "sin imagen también: {}", texto(e));
    assert_eq!(crear(e, Cual::Raza, &[("codigor", "3"), ("descripcion", "landrace")]), None);
    assert!(texto(e).contains("esa descripción"), "{}", texto(e));
    assert_eq!(crear(e, Cual::Raza, &[("codigor", "1"), ("descripcion", "Duroc")]), None);
    assert!(texto(e).contains("Ya existe una raza con ese código"), "{}", texto(e));

    // Modificar la descripción y la imagen; el código no cambia.
    abrir(e, Cual::Raza, 2);
    e.registro_modificar(Cual::Raza);
    llenar(e, Cual::Raza, &[("codigor", "9"), ("descripcion", "Duroc")]);
    e.raza_cargar_archivo(&jpg);
    e.registro_guardar(Cual::Raza);
    assert!(guardado(e), "{}", texto(e));
    let fila = e.db.registro_por_id(ent, 2, None).unwrap().unwrap();
    assert_eq!(fila.valores[ent.indice("descripcion")], "Duroc");
    assert!(fila.rotulos[ent.indice("imagen")].starts_with("JPG"), "{}", fila.rotulos[ent.indice("imagen")]);
    assert!(e.db.registro_por_id(ent, 9, None).unwrap().is_none(), "el codigor no cambia");

    e.registro_inhabilitar(Cual::Raza, 2);
    assert_eq!(e.db.registro_por_id(ent, 2, None).unwrap().unwrap().estado, INACTIVO, "borrado lógico");
}

#[test]
fn hu04_el_productor_solo_consulta_las_razas_activas_en_orden_alfabetico() {
    let mut t = con_admin();
    let e = &mut t.e;
    for (codigo, nombre) in [("1", "Pietrain"), ("2", "Duroc"), ("3", "Landrace")] {
        assert!(crear(e, Cual::Raza, &[("codigor", codigo), ("descripcion", nombre)]).is_some(), "{}", texto(e));
    }
    e.registro_inhabilitar(Cual::Raza, 1);
    assert!(crear_productor(e, productor_de_prueba("88123456", "Ramiro", "")));
    crear_usuario_para(e, 88_123_456, "ramiro", "Campo2026", 2);
    e.cerrar_sesion();
    ingresar(e, "ramiro", "Campo2026");
    e.ir_a(Pantalla::Gestion(Cual::Raza));
    let nombres: Vec<String> = e.registro(Cual::Raza).lista.iter().map(|r| r.valores[1].clone()).collect();
    assert_eq!(nombres, ["Duroc", "Landrace"]);
    e.registro_crear(Cual::Raza);
    assert!(texto(e).contains("solo puede consultar"), "{}", texto(e));
    let r = e.registro(Cual::Raza).lista[0].clone();
    e.registro_mostrar(Cual::Raza, &r);
    e.registro_modificar(Cual::Raza);
    assert_eq!(e.registro(Cual::Raza).modo, Modo::Consulta);
    e.registro_inhabilitar(Cual::Raza, r.id);
    assert_eq!(e.db.registro_por_id(Cual::Raza.entidad(), r.id, None).unwrap().unwrap().estado, ACTIVO);
}

#[test]
fn la_imagen_se_valida_por_firma_peso_y_contenido() {
    let png = imagen::validar(PNG.to_vec()).unwrap();
    assert_eq!((png.formato, png.ancho, png.alto), (imagen::Formato::Png, 120, 80));
    let jpg = imagen::validar(JPG.to_vec()).unwrap();
    assert_eq!((jpg.formato, jpg.ancho, jpg.alto), (imagen::Formato::Jpg, 120, 80));
    assert!(imagen::validar(Vec::new()).is_err());
    assert!(imagen::validar(b"hola".to_vec()).unwrap_err().contains("JPG o PNG"));
    let mini = imagen::miniatura(PNG, 60).unwrap();
    assert_eq!((mini.ancho, mini.alto, mini.rgba.len()), (60, 40, 60 * 40 * 4));
    let centro = &mini.rgba[(20 * 60 + 30) * 4..][..4];
    assert!(centro[0] > 200 && centro[3] == 255, "el centro de la ilustración es rosado: {centro:?}");
    assert_eq!(imagen::peso(2 * 1024 * 1024 + 300_000), "2,3 MB");
    assert_eq!(png.descripcion(), "PNG · 120 × 80 · 1 KB");
}

#[test]
fn hu05_crud_de_lote_con_sus_validaciones() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    let ent = Cual::Lote.entidad();
    e.ir_a(Pantalla::Gestion(Cual::Lote));
    e.registro_crear(Cual::Lote);
    assert_eq!(e.registro(Cual::Lote).valores[ent.indice("margen")], "15", "margen por defecto");
    assert_eq!(e.registro(Cual::Lote).valores[ent.indice("fecha_inicial")], hoy());

    let base = [
        ("codigol", "1"),
        ("codigo_granja", "100"),
        ("fecha_inicial", "2026-09-01"),
        ("fecha_final", "2027-01-15"),
        ("cantidad", "30"),
        ("peso_objetivo", "110"),
        ("margen", "15"),
    ];
    let casos: [(&str, &str, &str); 7] = [
        ("cantidad", "0", "entre 1 y 10000"),
        ("cantidad", "2,5", "números enteros"),
        ("fecha_final", "2026-08-01", "posterior a la fecha inicial"),
        ("fecha_final", "15/01/2027", "formato"),
        ("fecha_inicial", "2099-01-01", "futura"),
        ("peso_objetivo", "0", "entre 1 y 350"),
        ("margen", "301", "entre 0 y 300"),
    ];
    for (columna, valor, pista) in casos {
        let mut datos = base.to_vec();
        let i = datos.iter().position(|(c, _)| *c == columna).unwrap();
        datos[i] = (columna, valor);
        assert_eq!(crear(e, Cual::Lote, &datos), None, "{columna} = {valor}");
        assert!(texto(e).contains(pista), "{columna} = {valor}: {}", texto(e));
    }

    let id = crear(e, Cual::Lote, &base).unwrap_or_else(|| panic!("{}", texto(e)));
    assert_eq!(tipo(e), Some(TipoMensaje::Advertencia), "se guarda, pero sin corrales avisa");
    assert!(texto(e).contains("todavía no tiene corrales"), "{}", texto(e));
    let fila = e.db.registro_por_id(ent, id, None).unwrap().unwrap();
    assert_eq!(ent.nombre_de_estado(&fila.estado), "En ceba");
    assert_eq!(fila.extras[LOTE_VIVOS..=LOTE_MUERTOS], ["30", "0", "0"], "vivos, vendidos y muertos");
    assert_eq!(fila.extras[LOTE_COSTO], "Sin gastos registrados");
    assert_eq!(crear(e, Cual::Lote, &base), None);
    assert!(texto(e).contains("Ya existe un lote con ese código"), "{}", texto(e));
}

#[test]
fn hu05_asigna_corrales_activos_de_su_granja_con_fecha_y_advierte_la_capacidad() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    assert_eq!(crear_granja(e, "200", "88123456", "Villa Luz", "54", "54405"), Some(200), "{}", texto(e));
    assert_eq!(crear_corral(e, "21", "200", "Corral 1", "40"), Some(21), "{}", texto(e));

    preparar_lote(e, "1", "100", "30");
    let codigos: Vec<i64> = e.registro(Cual::Lote).corrales.iter().map(|c| c.corral.codigo).collect();
    assert_eq!(codigos, [11, 12], "solo los corrales activos de la misma granja");
    elegir_corral(e, 11, "21");
    e.registro_guardar(Cual::Lote);
    assert!(texto(e).contains("tiene 20 espacio(s) libre(s)"), "{}", texto(e));
    elegir_corral(e, 11, "20");
    elegir_corral(e, 12, "15");
    e.registro_guardar(Cual::Lote);
    assert!(texto(e).contains("Ubicó 35 cerdos"), "{}", texto(e));
    elegir_corral(e, 12, "10");
    e.registro_guardar(Cual::Lote);
    assert_eq!(tipo(e), Some(TipoMensaje::Exito), "{}", texto(e));
    let asignados = e.db.asignaciones_de_lote(1).unwrap();
    assert_eq!(asignados.iter().map(|a| (a.codigo_corral, a.cupos)).collect::<Vec<_>>(), [(11, 20), (12, 10)]);
    assert!(asignados.iter().all(|a| a.fecha == hoy()), "cada corral queda con su fecha de asignación");

    preparar_lote(e, "2", "100", "25");
    let libres: Vec<(i64, i64)> = e.registro(Cual::Lote).corrales.iter().map(|c| (c.corral.codigo, c.corral.libres())).collect();
    assert_eq!(libres, [(11, 0), (12, 5)], "descuenta lo que ocupa el otro lote en ceba");
    elegir_corral(e, 12, "5");
    e.registro_guardar(Cual::Lote);
    assert_eq!(tipo(e), Some(TipoMensaje::Advertencia), "{}", texto(e));
    assert!(texto(e).contains("capacidad para 15 cerdos y el lote tiene 25"), "{}", texto(e));

    preparar_lote(e, "3", "200", "8");
    let k = e.registro(Cual::Lote).corrales.iter().position(|c| c.corral.codigo == 21).unwrap();
    e.registro_mut(Cual::Lote).corrales[k].elegido = true;
    e.lote_proponer_cupos(k);
    assert_eq!(e.registro(Cual::Lote).corrales[k].cupos, "8", "al elegir el corral propone los cerdos que caben");

    assert!(rechaza(e, "INSERT INTO lote_corral VALUES (1, 21, '2026-11-13', 1, 'A')"), "la base no acepta un corral de otra granja");
}

#[test]
fn hu05_en_ceba_solo_cambian_observaciones_fecha_final_peso_margen_y_corrales() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    preparar_lote(e, "1", "100", "30");
    elegir_corral(e, 11, "20");
    elegir_corral(e, 12, "10");
    e.registro_guardar(Cual::Lote);
    assert!(guardado(e), "{}", texto(e));
    let ent = Cual::Lote.entidad();

    abrir(e, Cual::Lote, 1);
    e.registro_modificar(Cual::Lote);
    assert_eq!(e.registro(Cual::Lote).modo, Modo::Edicion);
    assert_eq!(e.registro(Cual::Lote).corrales.iter().filter(|c| c.elegido).count(), 2, "trae los corrales que tiene");
    llenar(
        e,
        Cual::Lote,
        &[
            ("cantidad", "99"),
            ("fecha_inicial", "2026-01-01"),
            ("observaciones", "Lechones de 25 kg"),
            ("fecha_final", "2027-02-01"),
            ("peso_objetivo", "115"),
            ("margen", "20"),
        ],
    );
    let k = e.registro(Cual::Lote).corrales.iter().position(|c| c.corral.codigo == 12).unwrap();
    e.registro_mut(Cual::Lote).corrales[k].elegido = false;
    e.registro_guardar(Cual::Lote);
    assert!(guardado(e), "{}", texto(e));

    let fila = e.db.registro_por_id(ent, 1, None).unwrap().unwrap();
    let valor = |c: &str| fila.valores[ent.indice(c)].clone();
    assert_eq!((valor("cantidad"), valor("fecha_inicial")), ("30".to_string(), "2026-09-01".to_string()), "no cambian");
    assert_eq!(valor("observaciones"), "Lechones de 25 kg");
    assert_eq!((valor("fecha_final"), valor("peso_objetivo"), valor("margen")), ("2027-02-01".to_string(), "115".to_string(), "20".to_string()));
    assert_eq!(e.db.asignaciones_de_lote(1).unwrap().len(), 1, "se liberó el corral 12");
    assert_eq!(
        contar(e, "SELECT COUNT(*) FROM lote_corral WHERE codigol = 1 AND codigo_corral = 12 AND estado = 'I'"),
        1,
        "la asignación queda inhabilitada, no se borra"
    );
}

#[test]
fn hu05_el_lote_se_cierra_solo_cuando_no_quedan_cerdos_vivos() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    preparar_lote(e, "1", "100", "30");
    e.registro_guardar(Cual::Lote);
    let ent = Cual::Lote.entidad();
    abrir(e, Cual::Lote, 1);
    e.lote_pedir_cierre();
    assert_eq!(e.registro(Cual::Lote).cerrando.as_deref(), Some(hoy().as_str()));
    e.lote_cerrar();
    assert!(texto(e).contains("tiene 30 cerdos vivos"), "{}", texto(e));
    assert_eq!(e.db.registro_por_id(ent, 1, None).unwrap().unwrap().estado, EN_CEBA);

    // Cuando las ventas y las muertes del Final dejen el lote sin vivos, el cierre lo pasa a Cerrado.
    e.db.cerrar_lote(1, "2026-11-10").unwrap();
    let fila = e.db.registro_por_id(ent, 1, None).unwrap().unwrap();
    assert_eq!((fila.estado.as_str(), fila.extras[LOTE_CIERRE].as_str()), (CERRADO, "2026-11-10"));
    assert!(e.db.cerrar_lote(1, "2026-11-11").is_err(), "un lote cerrado no se vuelve a cerrar");
    e.registro_mostrar(Cual::Lote, &fila);
    e.registro_modificar(Cual::Lote);
    assert!(texto(e).contains("Solo se modifica un lote en ceba"), "{}", texto(e));
    assert!(rechaza(e, "UPDATE lote SET fecha_cierre = '2026-08-01' WHERE codigol = 1"), "no antes de la fecha inicial");
}

#[test]
fn hu05_los_lotes_se_filtran_por_granja_y_por_estado() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    assert_eq!(crear_granja(e, "200", "88123456", "Villa Luz", "54", "54405"), Some(200), "{}", texto(e));
    for (codigo, granja) in [("1", "100"), ("2", "100"), ("3", "200")] {
        preparar_lote(e, codigo, granja, "10");
        e.registro_guardar(Cual::Lote);
        assert!(guardado(e), "{}", texto(e));
    }
    e.ir_a(Pantalla::Gestion(Cual::Lote));
    e.registro_inhabilitar(Cual::Lote, 2);
    e.registro_mut(Cual::Lote).filtros = vec!["100".into(), String::new()];
    e.registro_consultar(Cual::Lote);
    assert!(texto(e).contains("Lotes en la lista: 2"), "{}", texto(e));
    e.registro_mut(Cual::Lote).filtros = vec!["100".into(), "I".into()];
    e.registro_consultar(Cual::Lote);
    assert!(texto(e).contains("Lotes en la lista: 1"), "{}", texto(e));
    e.registro_mut(Cual::Lote).filtros = vec![String::new(), EN_CEBA.into()];
    e.registro_consultar(Cual::Lote);
    assert!(texto(e).contains("Lotes en la lista: 2"), "{}", texto(e));
}

#[test]
fn la_base_impone_las_reglas_aunque_falle_la_interfaz() {
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    preparar_lote(e, "1", "100", "30");
    elegir_corral(e, 11, "20");
    e.registro_guardar(Cual::Lote);
    assert!(guardado(e), "{}", texto(e));
    let granja = "INSERT INTO granja (codigo, idproductor, nombre, idmunicipio, vereda) VALUES";
    assert!(rechaza(e, &format!("{granja} (5, 1091352918, 'X', 54001, 'Y')")), "quien no es productor no tiene granjas");
    assert!(rechaza(e, &format!("{granja} (5, 88123456, 'X', 99999, 'Y')")), "municipio inexistente");
    assert!(rechaza(e, &format!("{granja} (100, 88123456, 'X', 54001, 'Y')")), "código repetido");
    assert!(rechaza(e, "INSERT INTO corral VALUES (13, 100, 'Corral 3', 0, 'A')"));
    assert!(rechaza(
        e,
        "INSERT INTO lote (codigol, codigo_granja, fecha_inicial, fecha_final, cantidad, peso_objetivo) VALUES (2, 100, '2026-10-01', '2026-09-01', 5, 100)"
    ));
    assert!(rechaza(e, "INSERT INTO lote_corral VALUES (1, 12, '2026-11-13', 11, 'A')"), "pasa la cantidad del lote");
    assert!(rechaza(e, "UPDATE corral SET capacidad = 10 WHERE codigo = 11"), "bajo lo que aloja");
    assert!(rechaza(e, "UPDATE corral SET estado = 'I' WHERE codigo = 11"), "con cerdos");
    assert!(rechaza(e, "UPDATE granja SET estado = 'I' WHERE codigo = 100"), "con lotes en ceba");
    assert!(rechaza(e, "UPDATE lote SET cantidad = 40 WHERE codigol = 1"), "la cantidad no cambia");
    assert!(rechaza(e, "UPDATE lote SET estado = 'X' WHERE codigol = 1"));
    assert!(rechaza(e, "INSERT INTO raza VALUES (5, 'Hampshire', zeroblob(2097153), 'A')"), "imagen de más de 2 MB");
    let fallo = e.db.conexion.execute("UPDATE corral SET estado = 'I' WHERE codigo = 11", []).unwrap_err();
    assert_eq!(crate::db::amable(fallo), "No se puede inhabilitar un corral con cerdos activos.", "el mensaje llega listo para el usuario");
}

#[test]
fn el_vigia_lee_la_llave_compuesta_y_la_cardinalidad_del_modelo() {
    let mut t = con_admin();
    let ruta = t.e.db.ruta.clone();
    let mut mirar = vigia::pasada_de_prueba(&ruta);
    let e = &mut t.e;
    escenario(e);
    preparar_lote(e, "1", "100", "30");
    elegir_corral(e, 11, "20");
    elegir_corral(e, 12, "10");
    e.registro_guardar(Cual::Lote);
    assert!(guardado(e), "{}", texto(e));

    let foto = mirar();
    let relacion = |tabla: &str, columna: &str| foto.relaciones.iter().find(|r| r.tabla == tabla && r.columna == columna).cloned().unwrap();
    assert!(relacion("productor", "idproductor").uno_a_uno, "un productor es una persona");
    assert!(relacion("usuario", "idusuario").uno_a_uno);
    assert!(!relacion("lote_corral", "codigol").uno_a_uno, "un lote, muchos corrales");
    assert!(!relacion("lote_corral", "codigo_corral").uno_a_uno, "un corral, muchos lotes");
    assert_eq!(relacion("granja", "idmunicipio").tabla_destino, "municipio");
    assert_eq!(foto.tabla("lote_corral").unwrap().claves, ["1 · 11", "1 · 12"]);
    assert_eq!(foto.tabla("municipio").unwrap().filas.len(), 1122);

    e.db.conexion.execute("UPDATE lote_corral SET cupos = 9 WHERE codigol = 1 AND codigo_corral = 12", []).unwrap();
    let foto = mirar();
    let cambio = foto.cambios.front().expect("detecta el cambio");
    assert_eq!((cambio.tabla.as_str(), cambio.clave.as_str()), ("lote_corral", "1 · 12"));
}

#[test]
fn la_base_se_consulta_desde_el_programa_sin_poder_cambiarla() {
    use crate::db::consulta::GUIADAS;
    let mut t = con_admin();
    let e = &mut t.e;
    escenario(e);
    preparar_lote(e, "1", "100", "30");
    elegir_corral(e, 11, "20");
    elegir_corral(e, 12, "10");
    e.registro_guardar(Cual::Lote);
    assert_eq!(tipo(e), Some(TipoMensaje::Exito), "{}", texto(e));
    e.ir_a(Pantalla::Mapa);

    // Cada consulta guiada corre sin error sobre el modelo real.
    for i in 0..GUIADAS.len() {
        e.consulta_guiada(i);
        let resultado = e.consulta.resultado.clone().expect("hay resultado");
        assert!(resultado.is_ok(), "{}: {:?}", GUIADAS[i].titulo, resultado);
        assert_eq!(e.consulta.guiada, Some(i));
        assert!(e.consulta.tabla.is_some(), "{} se muestra en la rejilla", GUIADAS[i].titulo);
    }
    let guiada = |e: &mut AppState, titulo: &str| -> Vec<Vec<String>> {
        e.consulta_guiada(GUIADAS.iter().position(|g| g.titulo == titulo).expect("existe la guiada"));
        e.consulta.resultado.clone().unwrap().unwrap().filas
    };
    let corrales = guiada(e, "Ocupación de los corrales");
    assert_eq!(corrales, [["11", "Corral 1", "La Esperanza", "20", "20", "0"], ["12", "Corral 2", "La Esperanza", "15", "10", "5"]]);
    assert_eq!(guiada(e, "Municipios por departamento").len(), 33, "los 33 departamentos del DANE");
    assert_eq!(guiada(e, "Productores y sus granjas")[0][4], "1", "Ramiro tiene una granja");

    // Una consulta propia; al editar el cuadro deja de marcarse la guiada.
    e.consulta.sql = "-- corrales con espacio\nSELECT nombre, capacidad FROM corral WHERE capacidad > 16;".into();
    e.consulta_ejecutar();
    assert_eq!(e.consulta.guiada, None);
    let r = e.consulta.resultado.clone().unwrap().unwrap();
    assert_eq!((r.columnas.clone(), r.filas.clone()), (vec!["nombre".to_string(), "capacidad".into()], vec![vec!["Corral 1".to_string(), "20".into()]]));

    // Las contraseñas no se muestran aunque la consulta las renombre.
    e.consulta.sql = "SELECT contrasena AS c FROM usuario".into();
    e.consulta_ejecutar();
    let r = e.consulta.resultado.clone().unwrap().unwrap();
    assert!(r.filas.iter().all(|f| f[0].starts_with("argon2id…") && f[0].len() < 20), "{:?}", r.filas);

    // Más de 500 filas: se recorta y se avisa.
    e.consulta.sql = "SELECT m.idmunicipio FROM municipio m".into();
    e.consulta_ejecutar();
    let r = e.consulta.resultado.clone().unwrap().unwrap();
    assert!(r.recortada && r.filas.len() == crate::db::consulta::MAXIMO_FILAS);

    // Nada de esto cambia datos ni pasa de una sentencia.
    let antes = contar(e, "SELECT COUNT(*) FROM corral");
    let error = |e: &mut AppState, sql: &str| -> String {
        e.consulta.sql = sql.into();
        e.consulta_ejecutar();
        e.consulta.resultado.clone().unwrap().expect_err(sql)
    };
    assert!(error(e, "DELETE FROM corral").contains("solo se consulta"));
    assert!(error(e, "WITH x AS (SELECT 1) DELETE FROM corral").contains("cambiaría datos"));
    assert!(error(e, "SELECT 1; DELETE FROM corral").contains("una sola consulta"));
    assert!(error(e, "ATTACH DATABASE 'otra.db' AS otra").contains("solo se consulta"));
    assert!(error(e, "PRAGMA query_only = OFF").contains("solo se consulta"));
    assert!(error(e, "SELECT * FROM cerdos").contains("No existe la tabla «cerdos»"));
    assert!(error(e, "SELECT * FROM corral WHERE codigo = ?").contains("parámetros"));
    assert!(error(e, "   ").contains("Escriba una consulta"));
    assert_eq!(contar(e, "SELECT COUNT(*) FROM corral"), antes);
    assert!(e.consulta.tabla.is_none(), "un error no deja la rejilla anterior");

    e.consulta_limpiar();
    assert!(e.consulta.sql.is_empty() && e.consulta.resultado.is_none());

    // El productor no tiene la pantalla ni puede consultar.
    crear_usuario_para(e, 88_123_456, "ramiro", "Campo2026", 2);
    e.cerrar_sesion();
    ingresar(e, "ramiro", "Campo2026");
    e.ir_a(Pantalla::Mapa);
    assert_ne!(e.pantalla, Pantalla::Mapa);
    e.consulta_guiada(0);
    assert!(e.consulta.resultado.is_none(), "{}", texto(e));
}
