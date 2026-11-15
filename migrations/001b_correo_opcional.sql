-- Farming 6 · Parcial II. HU-01 deja el correo del productor como opcional, y el productor es
-- una persona. SQLite no cambia una restricción NOT NULL en su lugar, así que la tabla persona se
-- reconstruye con el procedimiento de la documentación de SQLite: tabla nueva, copia de las filas,
-- borrado de la vieja y cambio de nombre. El correo sigue siendo único: la restricción UNIQUE
-- admite varios nulos, pero no dos correos iguales.
--
-- db/mod.rs solo la ejecuta cuando persona todavía exige el correo, con las llaves foráneas
-- apagadas, y después comprueba con PRAGMA foreign_key_check que usuario sigue apuntando a
-- personas que existen.

BEGIN;

CREATE TABLE persona_nueva (
    idpersona  INTEGER PRIMARY KEY,
    nom1       TEXT NOT NULL,
    nom2       TEXT,
    apell1     TEXT NOT NULL,
    apell2     TEXT,
    direccion  TEXT,
    tele       TEXT,
    movil      TEXT NOT NULL,
    correo     TEXT UNIQUE COLLATE NOCASE,
    fecha_nac  TEXT,
    estado     TEXT NOT NULL DEFAULT 'A' CHECK (estado IN ('A', 'I'))
);

INSERT INTO persona_nueva (idpersona, nom1, nom2, apell1, apell2, direccion, tele, movil, correo, fecha_nac, estado)
SELECT idpersona, nom1, nom2, apell1, apell2, direccion, tele, movil, correo, fecha_nac, estado FROM persona;

DROP TABLE persona;
ALTER TABLE persona_nueva RENAME TO persona;

COMMIT;
