//! Parcial II, requisitos aprobados HU-02 a HU-05: granjas porcícolas, corrales, razas y lotes de
//! cerdos. HU-01 (productores) usa el formulario de persona y está en ui/productor.rs.
//!
//! Cada formulario se describe una sola vez aquí: sus campos con el tipo de dato, las columnas de
//! su lista, los filtros, las reglas que impiden inhabilitarlo y los nombres de sus estados. De esa
//! descripción salen las consultas (db/registros.rs), las validaciones (validaciones::valor_de) y
//! la pantalla (ui/registro.rs).

/// Los cuatro formularios del motor del Parcial II.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Cual {
    Granja,
    Corral,
    Raza,
    Lote,
}

impl Cual {
    pub const TODAS: [Cual; 4] = [Cual::Granja, Cual::Corral, Cual::Raza, Cual::Lote];

    pub fn indice(self) -> usize {
        self as usize
    }

    pub fn entidad(self) -> &'static Entidad {
        match self {
            Cual::Granja => &GRANJA,
            Cual::Corral => &CORRAL,
            Cual::Raza => &RAZA,
            Cual::Lote => &LOTE,
        }
    }
}

/// Tabla a la que apunta una llave foránea. En `opciones`, `?1` es la persona en sesión (nulo si
/// es el Administrador, que ve todo) y `?2` es el valor ya guardado, para que siga saliendo aunque
/// su registro esté inhabilitado.
pub struct Fuente {
    /// Expresión SQL con el texto que se muestra en las listas.
    pub rotulo: &'static str,
    /// Consulta de las opciones que se pueden elegir: código y texto.
    pub opciones: &'static str,
}

pub enum Clase {
    /// Llave primaria que escribe el usuario: entero de 1 a 999.999.999. No se modifica.
    Codigo,
    /// Texto que debe llevar letras.
    Texto { min: usize, max: usize },
    /// Texto libre, como las observaciones.
    TextoLibre { max: usize },
    Entero { min: i64, max: i64 },
    Decimal { min: f64, max: f64 },
    /// Fecha AAAA-MM-DD. Si `futura` es falso no admite fechas posteriores a hoy.
    Fecha { futura: bool },
    /// Diez dígitos: celular que empieza por 3 o fijo que empieza por 60.
    Telefono,
    Correo,
    /// Llave foránea.
    Opcion(&'static Fuente),
    /// Departamento y luego municipio, en dos listas. Solo se guarda el municipio.
    Municipio,
    /// JPG o PNG de máximo 2 MB.
    Imagen,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cambia {
    /// Se escribe al crear y al modificar.
    Siempre,
    /// Solo se escribe al crear el registro.
    AlCrear,
}

pub struct Campo {
    pub columna: &'static str,
    pub etiqueta: &'static str,
    pub clase: Clase,
    pub obligatorio: bool,
    pub unidad: &'static str,
    /// Solo lo elige el Administrador; al productor se le asigna su propio documento.
    pub solo_admin: bool,
    pub cambia: Cambia,
}

const fn campo(columna: &'static str, etiqueta: &'static str, clase: Clase, obligatorio: bool) -> Campo {
    Campo { columna, etiqueta, clase, obligatorio, unidad: "", solo_admin: false, cambia: Cambia::Siempre }
}

const fn fijo(columna: &'static str, etiqueta: &'static str, clase: Clase) -> Campo {
    Campo { columna, etiqueta, clase, obligatorio: true, unidad: "", solo_admin: false, cambia: Cambia::AlCrear }
}

/// Dato de una fila que muestra una columna de la lista o que compara un filtro.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dato {
    Campo(&'static str),
    /// Posición en `Entidad::calculadas`.
    Calculada(usize),
    Estado,
}

pub struct Columna {
    pub titulo: &'static str,
    pub peso: f32,
    pub dato: Dato,
    pub solo_admin: bool,
}

const fn col(titulo: &'static str, peso: f32, dato: Dato) -> Columna {
    Columna { titulo, peso, dato, solo_admin: false }
}

const fn col_admin(titulo: &'static str, peso: f32, dato: Dato) -> Columna {
    Columna { titulo, peso, dato, solo_admin: true }
}

pub enum OpcionesFiltro {
    /// Consulta con `?1` (persona en sesión o nulo) y `?2` (valor del filtro del que depende).
    Consulta(&'static str),
    Fijas(&'static [(&'static str, &'static str)]),
}

pub struct Filtro {
    pub etiqueta: &'static str,
    pub opciones: OpcionesFiltro,
    pub dato: Dato,
    pub solo_admin: bool,
    /// Filtro anterior cuyo valor limita estas opciones, como el departamento al municipio.
    pub depende: Option<usize>,
}

/// Regla que impide inhabilitar: consulta que cuenta (con `?1` el código) y su mensaje.
pub struct Bloqueo {
    pub cuenta: &'static str,
    pub mensaje: &'static str,
}

pub struct Entidad {
    pub tabla: &'static str,
    /// Columna de la llave primaria: siempre es el primer campo.
    pub pk: &'static str,
    pub plural: &'static str,
    /// Título de la lista para el productor, por ejemplo «Mis granjas».
    pub plural_propio: &'static str,
    pub singular: &'static str,
    pub femenino: bool,
    pub campos: &'static [Campo],
    /// Expresiones SQL que se calculan por fila al consultar; nunca se guardan.
    pub calculadas: &'static [&'static str],
    pub lista: &'static [Columna],
    pub filtros: &'static [Filtro],
    /// Condición SQL que deja solo las filas que puede ver la persona `?1`.
    pub propio: &'static str,
    pub orden: &'static str,
    /// Campo que sirve de título en la ficha.
    pub titulo: usize,
    pub bloqueos: &'static [Bloqueo],
    pub pista: &'static str,
    /// Código y nombre de cada estado.
    pub estados: &'static [(&'static str, &'static str)],
    /// Quien no administra solo consulta.
    pub solo_consulta_sin_admin: bool,
}

impl Entidad {
    pub fn indice(&self, columna: &str) -> usize {
        self.campos.iter().position(|c| c.columna == columna).expect("la columna existe en la definición")
    }

    pub fn guardado(&self) -> String {
        format!("{} {} en la base de datos.", self.singular, if self.femenino { "guardada" } else { "guardado" })
    }

    pub fn inhabilitado(&self) -> String {
        let palabra = if self.femenino { "inhabilitada" } else { "inhabilitado" };
        format!("{} {palabra}. El registro sigue en la base de datos.", self.singular)
    }

    pub fn articulo(&self) -> &'static str {
        if self.femenino {
            "la"
        } else {
            "el"
        }
    }

    pub fn nombre_de_estado(&self, codigo: &str) -> &'static str {
        self.estados.iter().find(|(c, _)| *c == codigo).map(|(_, n)| *n).unwrap_or("Inhabilitado")
    }
}

const ESTADOS: &[(&str, &str)] = &[("A", "Activo"), ("I", "Inhabilitado")];

/// HU-05: un lote está En ceba, Cerrado o Inhabilitado.
pub const EN_CEBA: &str = "A";
pub const CERRADO: &str = "C";
const ESTADOS_DE_LOTE: &[(&str, &str)] = &[(EN_CEBA, "En ceba"), (CERRADO, "Cerrado"), ("I", "Inhabilitado")];

const PRODUCTORES: Fuente = Fuente {
    rotulo: "(SELECT p.nom1 || ' ' || p.apell1 FROM persona p WHERE p.idpersona = t.idproductor)",
    opciones: "SELECT pr.idproductor, p.nom1 || ' ' || p.apell1 || ' · ' || pr.idproductor
               FROM productor pr JOIN persona p ON p.idpersona = pr.idproductor
               WHERE (p.estado = 'A' OR pr.idproductor = ?2) AND (?1 IS NULL OR pr.idproductor = ?1) ORDER BY 2",
};

const GRANJAS: Fuente = Fuente {
    rotulo: "(SELECT g.nombre FROM granja g WHERE g.codigo = t.codigo_granja)",
    opciones: "SELECT g.codigo, g.nombre || ' · ' || g.codigo FROM granja g
               WHERE (g.estado = 'A' OR g.codigo = ?2) AND (?1 IS NULL OR g.idproductor = ?1) ORDER BY g.nombre",
};

/// HU-02. Gestión de Granjas Porcícolas.
static GRANJA: Entidad = Entidad {
    tabla: "granja",
    pk: "codigo",
    plural: "Granjas",
    plural_propio: "Mis granjas",
    singular: "Granja",
    femenino: true,
    campos: &[
        fijo("codigo", "Código", Clase::Codigo),
        Campo { solo_admin: true, ..fijo("idproductor", "Productor", Clase::Opcion(&PRODUCTORES)) },
        campo("nombre", "Nombre", Clase::Texto { min: 3, max: 60 }, true),
        campo("direccion", "Dirección", Clase::TextoLibre { max: 80 }, false),
        campo("telefono", "Teléfono", Clase::Telefono, false),
        campo("correo", "Correo electrónico", Clase::Correo, false),
        campo("idmunicipio", "Municipio", Clase::Municipio, true),
        campo("vereda", "Vereda", Clase::Texto { min: 3, max: 60 }, true),
    ],
    calculadas: &[
        "(SELECT m.iddepartamento FROM municipio m WHERE m.idmunicipio = t.idmunicipio)",
        "(SELECT d.nombre FROM municipio m JOIN departamento d ON d.iddepartamento = m.iddepartamento
          WHERE m.idmunicipio = t.idmunicipio)",
        "(SELECT COUNT(*) FROM corral c WHERE c.codigo_granja = t.codigo AND c.estado = 'A')",
        "(SELECT COUNT(*) FROM lote l WHERE l.codigo_granja = t.codigo AND l.estado = 'A')",
    ],
    lista: &[
        col("Código", 0.7, Dato::Campo("codigo")),
        col("Nombre", 1.8, Dato::Campo("nombre")),
        col_admin("Productor", 1.6, Dato::Campo("idproductor")),
        col_admin("Departamento", 1.5, Dato::Calculada(1)),
        col("Municipio", 1.6, Dato::Campo("idmunicipio")),
        col("Vereda", 1.4, Dato::Campo("vereda")),
        col("Corrales", 0.9, Dato::Calculada(2)),
        col("Lotes en ceba", 1.1, Dato::Calculada(3)),
        col("Estado", 1.2, Dato::Estado),
    ],
    filtros: &[
        Filtro {
            etiqueta: "Departamento",
            opciones: OpcionesFiltro::Consulta("SELECT iddepartamento, nombre FROM departamento ORDER BY nombre"),
            dato: Dato::Calculada(0),
            solo_admin: true,
            depende: None,
        },
        Filtro {
            etiqueta: "Municipio",
            opciones: OpcionesFiltro::Consulta("SELECT idmunicipio, nombre FROM municipio WHERE iddepartamento = ?2 ORDER BY nombre"),
            dato: Dato::Campo("idmunicipio"),
            solo_admin: true,
            depende: Some(0),
        },
        Filtro {
            etiqueta: "Productor",
            opciones: OpcionesFiltro::Consulta(
                "SELECT pr.idproductor, p.nom1 || ' ' || p.apell1 FROM productor pr
                 JOIN persona p ON p.idpersona = pr.idproductor ORDER BY 2",
            ),
            dato: Dato::Campo("idproductor"),
            solo_admin: true,
            depende: None,
        },
    ],
    propio: "t.idproductor = ?1",
    orden: "t.nombre COLLATE NOCASE, t.codigo",
    titulo: 2,
    bloqueos: &[Bloqueo {
        cuenta: "SELECT COUNT(*) FROM lote WHERE codigo_granja = ?1 AND estado = 'A'",
        mensaje: "No se puede inhabilitar: la granja tiene {n} lote(s) en ceba.",
    }],
    pista: "Consultar por código, nombre, municipio o vereda",
    estados: ESTADOS,
    solo_consulta_sin_admin: false,
};

/// Cerdos que un corral aloja hoy: los cupos asignados a lotes en ceba.
pub const OCUPACION_DEL_CORRAL: &str = "(SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc
    JOIN lote l ON l.codigol = lc.codigol
    WHERE lc.codigo_corral = t.codigo AND lc.estado = 'A' AND l.estado = 'A')";

const FILTRO_DE_GRANJA: Filtro = Filtro {
    etiqueta: "Granja",
    opciones: OpcionesFiltro::Consulta(
        "SELECT g.codigo, g.nombre FROM granja g WHERE (?1 IS NULL OR g.idproductor = ?1) ORDER BY g.nombre",
    ),
    dato: Dato::Campo("codigo_granja"),
    solo_admin: false,
    depende: None,
};

const DE_SUS_GRANJAS: &str = "t.codigo_granja IN (SELECT codigo FROM granja WHERE idproductor = ?1)";

/// HU-03. Gestión de Corrales.
static CORRAL: Entidad = Entidad {
    tabla: "corral",
    pk: "codigo",
    plural: "Corrales",
    plural_propio: "Corrales",
    singular: "Corral",
    femenino: false,
    campos: &[
        fijo("codigo", "Código", Clase::Codigo),
        fijo("codigo_granja", "Granja", Clase::Opcion(&GRANJAS)),
        campo("nombre", "Nombre", Clase::Texto { min: 2, max: 40 }, true),
        Campo { unidad: "cerdos", ..campo("capacidad", "Capacidad", Clase::Entero { min: 1, max: 10_000 }, true) },
    ],
    calculadas: &[
        "(SELECT group_concat(codigol, ', ') FROM (SELECT l.codigol FROM lote_corral lc
          JOIN lote l ON l.codigol = lc.codigol
          WHERE lc.codigo_corral = t.codigo AND lc.estado = 'A' AND l.estado = 'A' ORDER BY l.codigol))",
        OCUPACION_DEL_CORRAL,
    ],
    lista: &[
        col("Código", 0.7, Dato::Campo("codigo")),
        col("Nombre", 1.6, Dato::Campo("nombre")),
        col("Granja", 1.7, Dato::Campo("codigo_granja")),
        col("Capacidad", 1.0, Dato::Campo("capacidad")),
        col("Lotes asignados", 1.4, Dato::Calculada(0)),
        col("Cerdos que aloja", 1.2, Dato::Calculada(1)),
        col("Estado", 1.2, Dato::Estado),
    ],
    filtros: &[FILTRO_DE_GRANJA],
    propio: DE_SUS_GRANJAS,
    orden: "t.codigo_granja, t.nombre COLLATE NOCASE",
    titulo: 2,
    bloqueos: &[Bloqueo {
        cuenta: "SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc JOIN lote l ON l.codigol = lc.codigol
                 WHERE lc.codigo_corral = ?1 AND lc.estado = 'A' AND l.estado = 'A'",
        mensaje: "No se puede inhabilitar: el corral aloja {n} cerdo(s) activo(s).",
    }],
    pista: "Consultar por código, nombre o granja",
    estados: ESTADOS,
    solo_consulta_sin_admin: false,
};

/// HU-04. Gestión de Razas. El productor solo consulta las activas, en orden alfabético.
static RAZA: Entidad = Entidad {
    tabla: "raza",
    pk: "codigor",
    plural: "Razas",
    plural_propio: "Razas",
    singular: "Raza",
    femenino: true,
    campos: &[
        fijo("codigor", "Código", Clase::Codigo),
        campo("descripcion", "Descripción", Clase::Texto { min: 3, max: 50 }, true),
        campo("imagen", "Imagen", Clase::Imagen, false),
    ],
    calculadas: &[],
    lista: &[
        col("Código", 0.7, Dato::Campo("codigor")),
        col("Imagen", 1.5, Dato::Campo("imagen")),
        col("Descripción", 2.6, Dato::Campo("descripcion")),
        col("Estado", 1.2, Dato::Estado),
    ],
    filtros: &[],
    propio: "t.estado = 'A'",
    orden: "t.descripcion COLLATE NOCASE",
    titulo: 1,
    bloqueos: &[],
    pista: "Consultar por código o descripción",
    estados: ESTADOS,
    solo_consulta_sin_admin: true,
};

/// HU-05. Gestión de Lotes de Cerdos. Las ventas (HU-10) y las muertes (HU-06) se registran en el
/// Final; hasta entonces un lote no tiene cerdos vendidos ni muertos y todos siguen vivos.
static LOTE: Entidad = Entidad {
    tabla: "lote",
    pk: "codigol",
    plural: "Lotes",
    plural_propio: "Lotes",
    singular: "Lote",
    femenino: false,
    campos: &[
        fijo("codigol", "Código", Clase::Codigo),
        fijo("codigo_granja", "Granja", Clase::Opcion(&GRANJAS)),
        fijo("fecha_inicial", "Fecha inicial", Clase::Fecha { futura: false }),
        campo("fecha_final", "Fecha final estimada", Clase::Fecha { futura: true }, true),
        Campo { unidad: "cerdos", ..fijo("cantidad", "Cantidad de cerdos", Clase::Entero { min: 1, max: 10_000 }) },
        campo("observaciones", "Observaciones", Clase::TextoLibre { max: 200 }, false),
        Campo { unidad: "kg", ..campo("peso_objetivo", "Peso objetivo de venta", Clase::Decimal { min: 1.0, max: 350.0 }, true) },
        Campo { unidad: "%", ..campo("margen", "Margen de ganancia deseado", Clase::Decimal { min: 0.0, max: 300.0 }, true) },
    ],
    calculadas: &[
        // Cerdos vivos: la cantidad inicial menos los vendidos y los muertos. Mientras no existan
        // las ventas (HU-10) ni las muertes (HU-06), que llegan en el Final, son todos.
        "t.cantidad",
        // Vendidos: se contarán en el detalle de ventas (HU-10).
        "0",
        // Muertos: se contarán en el registro de animales (HU-06).
        "0",
        "(SELECT group_concat(nombre, ', ') FROM (SELECT c.nombre FROM lote_corral lc
          JOIN corral c ON c.codigo = lc.codigo_corral
          WHERE lc.codigol = t.codigol AND lc.estado = 'A' ORDER BY c.nombre COLLATE NOCASE))",
        // Costo por kilo actual: sale de los gastos (HU-08 y HU-09, Final).
        "'Sin gastos registrados'",
        "t.fecha_cierre",
    ],
    lista: &[
        col("Código", 0.7, Dato::Campo("codigol")),
        col("Granja", 1.4, Dato::Campo("codigo_granja")),
        col("Inicio", 1.1, Dato::Campo("fecha_inicial")),
        col("Fin estimado", 1.1, Dato::Campo("fecha_final")),
        col("Cerdos", 0.8, Dato::Campo("cantidad")),
        col("Vivos", 0.7, Dato::Calculada(0)),
        col("Vendidos", 0.85, Dato::Calculada(1)),
        col("Muertos", 0.8, Dato::Calculada(2)),
        col("Corrales", 1.5, Dato::Calculada(3)),
        col("Costo por kilo", 1.3, Dato::Calculada(4)),
        col("Estado", 1.1, Dato::Estado),
    ],
    filtros: &[
        FILTRO_DE_GRANJA,
        Filtro {
            etiqueta: "Estado",
            opciones: OpcionesFiltro::Fijas(ESTADOS_DE_LOTE),
            dato: Dato::Estado,
            solo_admin: false,
            depende: None,
        },
    ],
    propio: DE_SUS_GRANJAS,
    orden: "t.fecha_inicial DESC, t.codigol",
    titulo: 0,
    bloqueos: &[],
    pista: "Consultar por código, granja o corral",
    estados: ESTADOS_DE_LOTE,
    solo_consulta_sin_admin: false,
};

/// Posiciones de las columnas calculadas del lote que usa la ficha.
pub const LOTE_VIVOS: usize = 0;
pub const LOTE_VENDIDOS: usize = 1;
pub const LOTE_MUERTOS: usize = 2;
pub const LOTE_CORRALES: usize = 3;
pub const LOTE_COSTO: usize = 4;
pub const LOTE_CIERRE: usize = 5;

/// Valor ya validado, listo para guardarse con su tipo de dato.
#[derive(Debug, Clone, PartialEq)]
pub enum Valor {
    Nulo,
    Entero(i64),
    Real(f64),
    Texto(String),
    Bytes(Vec<u8>),
}

impl Valor {
    pub fn entero(&self) -> Option<i64> {
        match self {
            Valor::Entero(n) => Some(*n),
            _ => None,
        }
    }

    pub fn real(&self) -> Option<f64> {
        match self {
            Valor::Real(x) => Some(*x),
            Valor::Entero(n) => Some(*n as f64),
            _ => None,
        }
    }

    pub fn texto(&self) -> &str {
        match self {
            Valor::Texto(t) => t,
            _ => "",
        }
    }
}

/// Fila de cualquiera de los cuatro formularios.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Registro {
    pub id: i64,
    pub estado: String,
    /// Lo guardado, campo por campo, como texto que el formulario puede volver a leer.
    pub valores: Vec<String>,
    /// Lo que se muestra: nombres en lugar de códigos y cantidades con su unidad.
    pub rotulos: Vec<String>,
    /// Las columnas calculadas, en el orden de `Entidad::calculadas`.
    pub extras: Vec<String>,
}

impl Registro {
    /// Texto de una columna de la lista o del dato que compara un filtro.
    pub fn dato(&self, e: &Entidad, dato: Dato, para_mostrar: bool) -> String {
        match dato {
            Dato::Campo(columna) => {
                let i = e.indice(columna);
                if para_mostrar {
                    self.rotulos[i].clone()
                } else {
                    self.valores[i].clone()
                }
            }
            Dato::Calculada(i) => self.extras.get(i).cloned().unwrap_or_default(),
            Dato::Estado => {
                if para_mostrar {
                    e.nombre_de_estado(&self.estado).to_string()
                } else {
                    self.estado.clone()
                }
            }
        }
    }
}

/// Corral que se puede asignar a un lote, con lo que ya ocupan los demás lotes en ceba.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CorralDisponible {
    pub codigo: i64,
    pub nombre: String,
    pub capacidad: i64,
    /// Cupos que ocupan otros lotes en ceba.
    pub ocupados: i64,
    pub activo: bool,
}

impl CorralDisponible {
    pub fn libres(&self) -> i64 {
        (self.capacidad - self.ocupados).max(0)
    }
}

/// Corral asignado a un lote.
#[derive(Debug, Clone, PartialEq)]
pub struct Asignacion {
    pub codigo_corral: i64,
    pub nombre: String,
    pub capacidad: i64,
    pub cupos: i64,
    pub fecha: String,
}

/// Número sin ceros de sobra y con punto decimal: «350», «12.5», «7.89391».
pub fn decimal(x: f64) -> String {
    let texto = format!("{x:.6}");
    let texto = texto.trim_end_matches('0').trim_end_matches('.');
    if texto.is_empty() || texto == "-" {
        "0".to_string()
    } else {
        texto.to_string()
    }
}

fn miles(entero: u64) -> String {
    let digitos = entero.to_string();
    let mut salida = String::new();
    for (i, c) in digitos.chars().enumerate() {
        if i > 0 && (digitos.len() - i) % 3 == 0 {
            salida.push('.');
        }
        salida.push(c);
    }
    salida
}

/// Cantidad con coma decimal y hasta dos decimales, como se escribe en Colombia: «1.250,5».
pub fn cantidad(x: f64) -> String {
    let centesimas = (x.abs() * 100.0).round() as u64;
    let (entero, fraccion) = (centesimas / 100, centesimas % 100);
    let signo = if x < 0.0 && centesimas > 0 { "-" } else { "" };
    match fraccion {
        0 => format!("{signo}{}", miles(entero)),
        f if f % 10 == 0 => format!("{signo}{},{}", miles(entero), f / 10),
        f => format!("{signo}{},{f:02}", miles(entero)),
    }
}

/// Texto de un valor guardado tal como se muestra en listas y fichas.
pub fn rotulo_de(campo: &Campo, crudo: &str) -> String {
    if crudo.is_empty() {
        return String::new();
    }
    let numero = crudo.parse::<f64>().ok();
    match (&campo.clase, numero) {
        (Clase::Decimal { .. }, Some(x)) if campo.unidad == "%" => format!("{} %", cantidad(x)),
        (Clase::Decimal { .. }, Some(x)) if campo.unidad.is_empty() => cantidad(x),
        (Clase::Decimal { .. }, Some(x)) => format!("{} {}", cantidad(x), campo.unidad),
        _ => crudo.to_string(),
    }
}

/// Minúsculas, sin tildes y con la eñe como ene, para que «jose» encuentre a «José» y
/// «penaranda» a «Peñaranda». Solo se usa para buscar, nunca para guardar.
pub fn sin_tildes(texto: &str) -> String {
    texto
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            otro => otro,
        })
        .collect()
}
