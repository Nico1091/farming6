//! Enlaza con bibliotecas que ya trae Windows, sin compilador de C.
//!
//! rusqlite busca una biblioteca llamada sqlite3 en la carpeta que indica .cargo/config.toml,
//! y el enlazador de Rust para Windows no trae shlwapi. Se dejan ahí copias de las DLL del
//! sistema: el enlazador lee el nombre interno de cada una, así que al ejecutar Windows carga
//! las originales de System32 (winsqlite3.dll y shlwapi.dll).

use std::{env, fs, path::Path};

const BIBLIOTECAS: [(&str, &str); 2] = [("winsqlite3.dll", "sqlite3.dll"), ("shlwapi.dll", "shlwapi.dll")];

fn main() {
    let proyecto = env::var("CARGO_MANIFEST_DIR").expect("cargo define CARGO_MANIFEST_DIR");
    let raiz = env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
    let sistema = Path::new(&raiz).join("System32");
    let carpeta = Path::new(&proyecto).join("target").join("sqlite");
    fs::create_dir_all(&carpeta).expect("crear target/sqlite");
    for (original, copia) in BIBLIOTECAS {
        fs::copy(sistema.join(original), carpeta.join(copia)).expect("Windows debe traer esta DLL en System32");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
