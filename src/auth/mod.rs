//! Contraseñas: se guardan solo como hash Argon2id y nunca se pueden leer de vuelta.
//! Por eso la recuperación genera una contraseña temporal nueva.

pub mod correo;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use argon2::password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString};
use argon2::Argon2;

pub fn cifrar(contrasena: &str) -> Result<String, String> {
    let sal = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(contrasena.as_bytes(), &sal)
        .map(|hash| hash.to_string())
        .map_err(|e| format!("No se pudo cifrar la contraseña: {e}"))
}

pub fn verificar(contrasena: &str, hash: &str) -> bool {
    PasswordHash::new(hash)
        .map(|h| Argon2::default().verify_password(contrasena.as_bytes(), &h).is_ok())
        .unwrap_or(false)
}

/// Escoge un carácter al azar sin sesgo: descarta los valores que no caben en múltiplos exactos.
fn caracter_al_azar(alfabeto: &[u8]) -> char {
    let limite = 256 - (256 % alfabeto.len());
    loop {
        let valor = (OsRng.next_u32() & 0xFF) as usize;
        if valor < limite {
            return alfabeto[valor % alfabeto.len()] as char;
        }
    }
}

/// Contraseña temporal de 12 caracteres que cumple la política de los formularios.
pub fn clave_temporal() -> String {
    const MAYUSCULAS: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ";
    const MINUSCULAS: &[u8] = b"abcdefghijkmnopqrstuvwxyz";
    const NUMEROS: &[u8] = b"23456789";
    let mut clave = vec![
        caracter_al_azar(MAYUSCULAS),
        caracter_al_azar(MINUSCULAS),
        caracter_al_azar(NUMEROS),
    ];
    let todos: Vec<u8> = [MAYUSCULAS, MINUSCULAS, NUMEROS].concat();
    clave.extend((0..9).map(|_| caracter_al_azar(&todos)));
    for i in (1..clave.len()).rev() {
        let j = OsRng.next_u32() as usize % (i + 1);
        clave.swap(i, j);
    }
    clave.into_iter().collect()
}

#[cfg(test)]
mod pruebas {
    use super::*;
    use crate::validaciones;

    #[test]
    fn verifica_solo_la_contrasena_correcta() {
        let hash = cifrar("Campo2026").unwrap();
        assert!(hash.starts_with("$argon2id$"));
        assert!(verificar("Campo2026", &hash));
        assert!(!verificar("campo2026", &hash));
        assert!(!verificar("Campo2026", "no es un hash"));
    }

    #[test]
    fn la_clave_temporal_cumple_la_politica_y_no_se_repite() {
        let a = clave_temporal();
        let b = clave_temporal();
        assert_eq!(a.len(), 12);
        assert_ne!(a, b);
        for _ in 0..200 {
            assert_eq!(validaciones::contrasena(&clave_temporal()), Ok(()));
        }
    }
}
