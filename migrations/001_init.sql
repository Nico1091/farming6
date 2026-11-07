-- Farming 6 · Parcial I. Modelo relacional del enunciado: perfil, usuario y persona.
-- Un perfil lo pueden tener varios usuarios (1:N).
-- Una persona tiene un solo usuario y ese usuario es de una sola persona (1:1):
-- idusuario es a la vez la llave primaria de usuario y la llave foránea hacia persona.
-- estado es el interruptor del borrado lógico: 'A' activo, 'I' inhabilitado.

CREATE TABLE IF NOT EXISTS perfil (
    idperfil   INTEGER PRIMARY KEY AUTOINCREMENT,
    descripc   TEXT NOT NULL UNIQUE COLLATE NOCASE,
    estado     TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I'))
);

CREATE TABLE IF NOT EXISTS persona (
    idpersona  INTEGER PRIMARY KEY,
    nom1       TEXT NOT NULL,
    nom2       TEXT,
    apell1     TEXT NOT NULL,
    apell2     TEXT,
    direccion  TEXT,
    tele       TEXT,
    movil      TEXT NOT NULL,
    correo     TEXT NOT NULL UNIQUE COLLATE NOCASE,
    fecha_nac  TEXT,
    estado     TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I'))
);

CREATE TABLE IF NOT EXISTS usuario (
    idusuario  INTEGER PRIMARY KEY REFERENCES persona (idpersona),
    nombreu    TEXT NOT NULL UNIQUE COLLATE NOCASE,
    contrasena TEXT NOT NULL,
    idperfil   INTEGER NOT NULL REFERENCES perfil (idperfil),
    estado     TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I'))
);

INSERT OR IGNORE INTO perfil (idperfil, descripc) VALUES
    (1, 'Administrador'),
    (2, 'Campesino');
