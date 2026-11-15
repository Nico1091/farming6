-- Farming 6 · Parcial II. Requisitos funcionales aprobados, HU-01 a HU-05: productores, granjas
-- porcícolas, corrales, razas y lotes de cerdos. Completa el modelo del Parcial I (perfil,
-- persona y usuario) y queda en tercera forma normal.
--
-- 1FN: cada columna guarda un solo valor. Los corrales de un lote no se escriben como una lista
--      dentro de lote: cada asignación es una fila de lote_corral.
-- 2FN: lote_corral es la única tabla con llave compuesta (lote y corral). La fecha de asignación
--      y los cupos dependen de los dos campos juntos, no de uno solo.
-- 3FN: ningún dato depende de otro que no sea la llave. La granja guarda el municipio y no el
--      departamento, porque el departamento depende del municipio. Los datos personales del
--      productor siguen en persona; productor solo registra quién cría cerdos y desde cuándo.
--      Tampoco se guardan totales: los corrales y lotes de una granja, los cerdos de un corral y
--      los cerdos vivos de un lote se cuentan al consultarlos.
--
-- estado es el interruptor del borrado lógico: 'A' activo, 'I' inhabilitado. En lote, 'A' es En
-- ceba y 'C' es Cerrado.

-- HU-01. El productor es una persona (1:1), igual que usuario: idproductor es a la vez la llave
-- primaria y la llave foránea hacia persona. Su estado es el de la persona.
CREATE TABLE IF NOT EXISTS productor (
    idproductor    INTEGER PRIMARY KEY REFERENCES persona (idpersona),
    fecha_registro TEXT NOT NULL
);

-- HU-02. Catálogos oficiales del DANE (DIVIPOLA): la llave es el código de cada uno.
CREATE TABLE IF NOT EXISTS departamento (
    iddepartamento INTEGER PRIMARY KEY,
    nombre         TEXT NOT NULL UNIQUE COLLATE NOCASE
);

CREATE TABLE IF NOT EXISTS municipio (
    idmunicipio    INTEGER PRIMARY KEY,
    iddepartamento INTEGER NOT NULL REFERENCES departamento (iddepartamento),
    nombre         TEXT NOT NULL COLLATE NOCASE,
    UNIQUE (iddepartamento, nombre)
);

-- HU-02. Granja porcícola de un productor (1:N), ubicada en un municipio (1:N).
CREATE TABLE IF NOT EXISTS granja (
    codigo      INTEGER PRIMARY KEY CHECK (codigo BETWEEN 1 AND 999999999),
    idproductor INTEGER NOT NULL REFERENCES productor (idproductor),
    nombre      TEXT NOT NULL,
    direccion   TEXT,
    telefono    TEXT CHECK (telefono IS NULL OR (length(telefono) = 10 AND telefono NOT GLOB '*[^0-9]*')),
    correo      TEXT,
    idmunicipio INTEGER NOT NULL REFERENCES municipio (idmunicipio),
    vereda      TEXT NOT NULL,
    estado      TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I'))
);

-- HU-03. Corral de una granja (1:N). El nombre no se repite dentro de la misma granja.
CREATE TABLE IF NOT EXISTS corral (
    codigo        INTEGER PRIMARY KEY CHECK (codigo BETWEEN 1 AND 999999999),
    codigo_granja INTEGER NOT NULL REFERENCES granja (codigo),
    nombre        TEXT NOT NULL COLLATE NOCASE,
    capacidad     INTEGER NOT NULL CHECK (capacidad > 0),
    estado        TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I')),
    UNIQUE (codigo_granja, nombre)
);

-- HU-04. Raza de cerdo con su imagen JPG o PNG de máximo 2 MB (2.097.152 bytes).
CREATE TABLE IF NOT EXISTS raza (
    codigor     INTEGER PRIMARY KEY CHECK (codigor BETWEEN 1 AND 999999999),
    descripcion TEXT NOT NULL UNIQUE COLLATE NOCASE,
    imagen      BLOB CHECK (imagen IS NULL OR length(imagen) <= 2097152),
    estado      TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I'))
);

-- HU-05. Lote de cerdos de una granja (1:N): un ciclo de ceba. fecha_final es la estimada y
-- fecha_cierre la real, que se escribe al cerrar el lote.
CREATE TABLE IF NOT EXISTS lote (
    codigol       INTEGER PRIMARY KEY CHECK (codigol BETWEEN 1 AND 999999999),
    codigo_granja INTEGER NOT NULL REFERENCES granja (codigo),
    fecha_inicial TEXT NOT NULL,
    fecha_final   TEXT NOT NULL CHECK (fecha_final > fecha_inicial),
    cantidad      INTEGER NOT NULL CHECK (cantidad > 0),
    observaciones TEXT,
    peso_objetivo REAL NOT NULL CHECK (peso_objetivo > 0),
    margen        REAL NOT NULL DEFAULT 15 CHECK (margen BETWEEN 0 AND 300),
    fecha_cierre  TEXT CHECK (fecha_cierre IS NULL OR fecha_cierre >= fecha_inicial),
    estado        TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'C', 'I'))
);

-- HU-05. Corrales asignados a un lote: relación de muchos a muchos entre lote y corral. cupos son
-- los cerdos del lote que se ubicaron en ese corral al asignarlo.
CREATE TABLE IF NOT EXISTS lote_corral (
    codigol          INTEGER NOT NULL REFERENCES lote (codigol),
    codigo_corral    INTEGER NOT NULL REFERENCES corral (codigo),
    fecha_asignacion TEXT NOT NULL,
    cupos            INTEGER NOT NULL CHECK (cupos > 0),
    estado           TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I')),
    PRIMARY KEY (codigol, codigo_corral)
);

CREATE INDEX IF NOT EXISTS municipio_de_departamento ON municipio (iddepartamento);
CREATE INDEX IF NOT EXISTS granja_de_productor ON granja (idproductor);
CREATE INDEX IF NOT EXISTS granja_de_municipio ON granja (idmunicipio);
CREATE INDEX IF NOT EXISTS corral_de_granja ON corral (codigo_granja);
CREATE INDEX IF NOT EXISTS lote_de_granja ON lote (codigo_granja);
CREATE INDEX IF NOT EXISTS lote_corral_de_corral ON lote_corral (codigo_corral);

-- Reglas que la base cumple aunque falle la interfaz. La ocupación de un corral es la suma de los
-- cupos que tiene asignados a lotes en ceba.

CREATE TRIGGER IF NOT EXISTS lote_corral_misma_granja
BEFORE INSERT ON lote_corral
WHEN (SELECT codigo_granja FROM lote WHERE codigol = NEW.codigol)
     IS NOT (SELECT codigo_granja FROM corral WHERE codigo = NEW.codigo_corral)
BEGIN
    SELECT RAISE(ABORT, 'El corral no pertenece a la granja del lote.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_corral_activo
BEFORE INSERT ON lote_corral
WHEN NEW.estado = 'A' AND (SELECT estado FROM corral WHERE codigo = NEW.codigo_corral) IS NOT 'A'
BEGIN
    SELECT RAISE(ABORT, 'Solo se asignan corrales activos.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_reasignar_corral_activo
BEFORE UPDATE OF estado ON lote_corral
WHEN NEW.estado = 'A' AND OLD.estado = 'I' AND (SELECT estado FROM corral WHERE codigo = NEW.codigo_corral) IS NOT 'A'
BEGIN
    SELECT RAISE(ABORT, 'Solo se asignan corrales activos.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_lote_en_ceba
BEFORE INSERT ON lote_corral
WHEN NEW.estado = 'A' AND (SELECT estado FROM lote WHERE codigol = NEW.codigol) IS NOT 'A'
BEGIN
    SELECT RAISE(ABORT, 'Solo los lotes en ceba reciben corrales.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_sin_pasar_cantidad
BEFORE INSERT ON lote_corral
WHEN NEW.estado = 'A'
 AND NEW.cupos + (SELECT COALESCE(SUM(cupos), 0) FROM lote_corral
                  WHERE codigol = NEW.codigol AND codigo_corral <> NEW.codigo_corral AND estado = 'A')
     > (SELECT cantidad FROM lote WHERE codigol = NEW.codigol)
BEGIN
    SELECT RAISE(ABORT, 'Los cupos pasan la cantidad de cerdos del lote.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_cupos_sin_pasar_cantidad
BEFORE UPDATE OF cupos, estado ON lote_corral
WHEN NEW.estado = 'A'
 AND NEW.cupos + (SELECT COALESCE(SUM(cupos), 0) FROM lote_corral
                  WHERE codigol = NEW.codigol AND codigo_corral <> NEW.codigo_corral AND estado = 'A')
     > (SELECT cantidad FROM lote WHERE codigol = NEW.codigol)
BEGIN
    SELECT RAISE(ABORT, 'Los cupos pasan la cantidad de cerdos del lote.');
END;

CREATE TRIGGER IF NOT EXISTS lote_datos_de_origen_fijos
BEFORE UPDATE OF codigo_granja, fecha_inicial, cantidad ON lote
WHEN NEW.codigo_granja IS NOT OLD.codigo_granja
  OR NEW.fecha_inicial IS NOT OLD.fecha_inicial
  OR NEW.cantidad IS NOT OLD.cantidad
BEGIN
    SELECT RAISE(ABORT, 'La granja, la fecha inicial y la cantidad de un lote no se modifican.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_sin_pasar_capacidad
BEFORE INSERT ON lote_corral
WHEN NEW.estado = 'A'
 AND NEW.cupos + (SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc JOIN lote l ON l.codigol = lc.codigol
                  WHERE lc.codigo_corral = NEW.codigo_corral AND lc.codigol <> NEW.codigol
                    AND lc.estado = 'A' AND l.estado = 'A')
     > (SELECT capacidad FROM corral WHERE codigo = NEW.codigo_corral)
BEGIN
    SELECT RAISE(ABORT, 'Los cupos pasan la capacidad del corral.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_cupos_sin_pasar_capacidad
BEFORE UPDATE OF cupos, estado ON lote_corral
WHEN NEW.estado = 'A'
 AND NEW.cupos + (SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc JOIN lote l ON l.codigol = lc.codigol
                  WHERE lc.codigo_corral = NEW.codigo_corral AND lc.codigol <> NEW.codigol
                    AND lc.estado = 'A' AND l.estado = 'A')
     > (SELECT capacidad FROM corral WHERE codigo = NEW.codigo_corral)
BEGIN
    SELECT RAISE(ABORT, 'Los cupos pasan la capacidad del corral.');
END;

CREATE TRIGGER IF NOT EXISTS lote_corral_llave_fija
BEFORE UPDATE OF codigol, codigo_corral ON lote_corral
BEGIN
    SELECT RAISE(ABORT, 'Una asignación no cambia de lote ni de corral: se libera y se asigna otra.');
END;

CREATE TRIGGER IF NOT EXISTS corral_capacidad_ocupada
BEFORE UPDATE OF capacidad ON corral
WHEN NEW.capacidad < (SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc JOIN lote l ON l.codigol = lc.codigol
                      WHERE lc.codigo_corral = NEW.codigo AND lc.estado = 'A' AND l.estado = 'A')
BEGIN
    SELECT RAISE(ABORT, 'La capacidad no puede quedar por debajo de los cerdos que hay en el corral.');
END;

CREATE TRIGGER IF NOT EXISTS corral_inhabilitar_con_cerdos
BEFORE UPDATE OF estado ON corral
WHEN NEW.estado = 'I' AND OLD.estado = 'A'
 AND (SELECT COALESCE(SUM(lc.cupos), 0) FROM lote_corral lc JOIN lote l ON l.codigol = lc.codigol
      WHERE lc.codigo_corral = NEW.codigo AND lc.estado = 'A' AND l.estado = 'A') > 0
BEGIN
    SELECT RAISE(ABORT, 'No se puede inhabilitar un corral con cerdos activos.');
END;

CREATE TRIGGER IF NOT EXISTS granja_inhabilitar_con_lotes
BEFORE UPDATE OF estado ON granja
WHEN NEW.estado = 'I' AND OLD.estado = 'A'
 AND EXISTS (SELECT 1 FROM lote WHERE codigo_granja = NEW.codigo AND estado = 'A')
BEGIN
    SELECT RAISE(ABORT, 'No se puede inhabilitar una granja con lotes en ceba.');
END;

CREATE TRIGGER IF NOT EXISTS lote_inhabilitado_libera_corrales
AFTER UPDATE OF estado ON lote
WHEN NEW.estado = 'I'
BEGIN
    UPDATE lote_corral SET estado = 'I' WHERE codigol = NEW.codigol AND estado = 'A';
END;
