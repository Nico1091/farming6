//! HU-04: imagen de la raza, JPG o PNG de máximo 2 MB.
//!
//! La firma de los primeros bytes dice si el archivo dice ser JPG o PNG; GDI+, la biblioteca de
//! imágenes que trae Windows, lo decodifica de verdad, con lo cual se rechaza un archivo dañado o
//! uno que solo tiene la extensión cambiada. GDI+ también saca la miniatura que se muestra en la
//! aplicación, y el diálogo de abrir archivo es el de Windows. No se usa ninguna librería externa:
//! build.rs deja gdiplus.dll y shlwapi.dll del sistema para el enlazador, igual que la SQLite.

use std::ffi::c_void;
use std::os::windows::ffi::OsStringExt;
use std::path::{Path, PathBuf};

/// 2 MB = 2.097.152 bytes, el mismo límite del CHECK de la tabla raza.
pub const MAXIMO_BYTES: usize = 2 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Formato {
    Jpg,
    Png,
}

impl Formato {
    pub fn nombre(self) -> &'static str {
        match self {
            Formato::Jpg => "JPG",
            Formato::Png => "PNG",
        }
    }
}

/// Imagen ya validada, lista para guardarse.
#[derive(Clone, Debug, PartialEq)]
pub struct Imagen {
    pub bytes: Vec<u8>,
    pub formato: Formato,
    pub ancho: u32,
    pub alto: u32,
}

impl Imagen {
    /// «PNG · 640 × 480 · 84 KB».
    pub fn descripcion(&self) -> String {
        format!("{} · {} × {} · {}", self.formato.nombre(), self.ancho, self.alto, peso(self.bytes.len()))
    }
}

/// Píxeles RGBA de una miniatura, fila por fila.
pub struct Pixeles {
    pub ancho: usize,
    pub alto: usize,
    pub rgba: Vec<u8>,
}

pub fn formato_por_firma(bytes: &[u8]) -> Option<Formato> {
    if bytes.starts_with(&[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]) {
        Some(Formato::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Some(Formato::Jpg)
    } else {
        None
    }
}

/// «84 KB» o «2,4 MB».
pub fn peso(bytes: usize) -> String {
    if bytes < 1024 * 1024 {
        format!("{} KB", bytes.div_ceil(1024))
    } else {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0).replace('.', ",")
    }
}

/// Valida una imagen: que no pase de 2 MB, que su firma sea JPG o PNG y que Windows la lea.
pub fn validar(bytes: Vec<u8>) -> Result<Imagen, String> {
    if bytes.is_empty() {
        return Err("El archivo está vacío.".into());
    }
    if bytes.len() > MAXIMO_BYTES {
        return Err(format!("La imagen pesa {} y el máximo es 2 MB.", peso(bytes.len())));
    }
    let Some(formato) = formato_por_firma(&bytes) else {
        return Err("Solo se aceptan imágenes JPG o PNG.".into());
    };
    let (ancho, alto, leido) = gdiplus::medidas(&bytes)?;
    if leido != Some(formato) {
        return Err("El contenido del archivo no corresponde a una imagen JPG o PNG.".into());
    }
    if ancho == 0 || alto == 0 || ancho > 12_000 || alto > 12_000 {
        return Err("La imagen tiene unas medidas que no se pueden mostrar.".into());
    }
    Ok(Imagen { bytes, formato, ancho, alto })
}

/// Lee y valida un archivo elegido con el diálogo o arrastrado a la ventana.
pub fn leer_archivo(ruta: &Path) -> Result<Imagen, String> {
    let extension = ruta.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    if !matches!(extension.as_str(), "jpg" | "jpeg" | "png") {
        return Err("Solo se aceptan archivos .jpg, .jpeg o .png.".into());
    }
    let tamano = std::fs::metadata(ruta).map_err(|_| "No se pudo leer el archivo.".to_string())?.len();
    if tamano > MAXIMO_BYTES as u64 {
        return Err(format!("La imagen pesa {} y el máximo es 2 MB.", peso(tamano as usize)));
    }
    let bytes = std::fs::read(ruta).map_err(|_| "No se pudo leer el archivo.".to_string())?;
    validar(bytes)
}

/// Miniatura que cabe en un cuadrado de `lado` píxeles, sin deformar la imagen ni agrandarla.
pub fn miniatura(bytes: &[u8], lado: u32) -> Result<Pixeles, String> {
    gdiplus::miniatura(bytes, lado)
}

/// Llamadas a GDI+ (gdiplus.dll) y a SHCreateMemStream (shlwapi.dll), que convierte los bytes en
/// el flujo que GDI+ sabe leer.
mod gdiplus {
    use super::{Formato, Pixeles};
    use std::ffi::c_void;
    use std::ptr::null_mut;
    use std::sync::OnceLock;

    // Los campos los lee Windows, no el código de Rust.
    #[allow(dead_code)]
    #[repr(C)]
    struct Entrada {
        version: u32,
        retrollamada: *mut c_void,
        sin_hilo_de_fondo: i32,
        sin_codecs_externos: i32,
    }

    // Los campos los lee Windows, no el código de Rust.
    #[allow(dead_code)]
    #[repr(C)]
    #[derive(Clone, Copy, Default, PartialEq, Eq)]
    struct Guid {
        a: u32,
        b: u16,
        c: u16,
        d: [u8; 8],
    }

    // Los campos los lee Windows, no el código de Rust.
    #[allow(dead_code)]
    #[repr(C)]
    struct Rectangulo {
        x: i32,
        y: i32,
        ancho: i32,
        alto: i32,
    }

    // Los campos los lee Windows, no el código de Rust.
    #[allow(dead_code)]
    #[repr(C)]
    struct DatosDeBits {
        ancho: u32,
        alto: u32,
        paso: i32,
        formato: i32,
        scan0: *mut u8,
        reservado: usize,
    }

    /// Primeras entradas de la tabla de IUnknown; solo se usa Release para soltar el flujo.
    // Los campos los lee Windows, no el código de Rust.
    #[allow(dead_code)]
    #[repr(C)]
    struct TablaIUnknown {
        consultar: usize,
        sumar: usize,
        soltar: unsafe extern "system" fn(*mut c_void) -> u32,
    }

    const ARGB_32: i32 = 0x0026_200A;
    const SOLO_LECTURA: u32 = 1;
    const BICUBICA_DE_ALTA_CALIDAD: i32 = 7;
    const DESFASE_DE_ALTA_CALIDAD: i32 = 2;
    const PNG: Guid = Guid { a: 0xb96b_3caf, b: 0x0728, c: 0x11d3, d: [0x9d, 0x7b, 0x00, 0x00, 0xf8, 0x1e, 0xf3, 0x2e] };
    const JPG: Guid = Guid { a: 0xb96b_3cae, b: 0x0728, c: 0x11d3, d: [0x9d, 0x7b, 0x00, 0x00, 0xf8, 0x1e, 0xf3, 0x2e] };

    #[link(name = "gdiplus")]
    extern "system" {
        fn GdiplusStartup(token: *mut usize, entrada: *const Entrada, salida: *mut c_void) -> i32;
        fn GdipCreateBitmapFromStream(flujo: *mut c_void, imagen: *mut *mut c_void) -> i32;
        fn GdipGetImageWidth(imagen: *mut c_void, ancho: *mut u32) -> i32;
        fn GdipGetImageHeight(imagen: *mut c_void, alto: *mut u32) -> i32;
        fn GdipGetImageRawFormat(imagen: *mut c_void, formato: *mut Guid) -> i32;
        fn GdipCreateBitmapFromScan0(ancho: i32, alto: i32, paso: i32, formato: i32, scan0: *mut u8, imagen: *mut *mut c_void) -> i32;
        fn GdipGetImageGraphicsContext(imagen: *mut c_void, grafico: *mut *mut c_void) -> i32;
        fn GdipSetInterpolationMode(grafico: *mut c_void, modo: i32) -> i32;
        fn GdipSetPixelOffsetMode(grafico: *mut c_void, modo: i32) -> i32;
        fn GdipDrawImageRectI(grafico: *mut c_void, imagen: *mut c_void, x: i32, y: i32, ancho: i32, alto: i32) -> i32;
        fn GdipDeleteGraphics(grafico: *mut c_void) -> i32;
        fn GdipBitmapLockBits(imagen: *mut c_void, rect: *const Rectangulo, banderas: u32, formato: i32, datos: *mut DatosDeBits) -> i32;
        fn GdipBitmapUnlockBits(imagen: *mut c_void, datos: *mut DatosDeBits) -> i32;
        fn GdipDisposeImage(imagen: *mut c_void) -> i32;
    }

    #[link(name = "shlwapi")]
    extern "system" {
        fn SHCreateMemStream(datos: *const u8, largo: u32) -> *mut c_void;
    }

    /// GDI+ se inicia una sola vez por proceso.
    fn iniciar() -> Result<(), String> {
        static INICIO: OnceLock<i32> = OnceLock::new();
        let estado = *INICIO.get_or_init(|| {
            let entrada = Entrada { version: 1, retrollamada: null_mut(), sin_hilo_de_fondo: 0, sin_codecs_externos: 0 };
            let mut token = 0usize;
            // SAFETY: la entrada vive durante la llamada y la salida puede ser nula porque no se
            // suprime el hilo de fondo de GDI+.
            unsafe { GdiplusStartup(&mut token, &entrada, null_mut()) }
        });
        if estado == 0 {
            Ok(())
        } else {
            Err(format!("Windows no pudo iniciar su lector de imágenes (GDI+, estado {estado})."))
        }
    }

    struct Flujo(*mut c_void);

    impl Drop for Flujo {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: es un IStream vivo creado por SHCreateMemStream; Release lo libera.
                unsafe {
                    let tabla = *(self.0 as *const *const TablaIUnknown);
                    ((*tabla).soltar)(self.0);
                }
            }
        }
    }

    struct Objeto(*mut c_void);

    impl Drop for Objeto {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: imagen creada por GDI+ que nadie más usa.
                unsafe { GdipDisposeImage(self.0) };
            }
        }
    }

    struct Grafico(*mut c_void);

    impl Drop for Grafico {
        fn drop(&mut self) {
            if !self.0.is_null() {
                // SAFETY: contexto de dibujo creado por GDI+ que nadie más usa.
                unsafe { GdipDeleteGraphics(self.0) };
            }
        }
    }

    /// Abre la imagen. El flujo va primero en la tupla para que se suelte después de la imagen,
    /// que lo sigue leyendo mientras exista.
    fn abrir(bytes: &[u8]) -> Result<(Flujo, Objeto), String> {
        iniciar()?;
        let largo = u32::try_from(bytes.len()).map_err(|_| "La imagen es demasiado grande.".to_string())?;
        // SAFETY: SHCreateMemStream copia los bytes; el puntero es válido durante la llamada.
        let flujo = Flujo(unsafe { SHCreateMemStream(bytes.as_ptr(), largo) });
        if flujo.0.is_null() {
            return Err("Windows no pudo leer la imagen en memoria.".into());
        }
        let mut imagen = null_mut();
        // SAFETY: el flujo es un IStream válido y `imagen` recibe el puntero de salida.
        let estado = unsafe { GdipCreateBitmapFromStream(flujo.0, &mut imagen) };
        let imagen = Objeto(imagen);
        if estado != 0 || imagen.0.is_null() {
            return Err("El archivo no es una imagen válida o está dañado.".into());
        }
        Ok((flujo, imagen))
    }

    fn tamano(imagen: &Objeto) -> (u32, u32) {
        let (mut ancho, mut alto) = (0u32, 0u32);
        // SAFETY: imagen viva; las salidas son variables locales.
        unsafe {
            GdipGetImageWidth(imagen.0, &mut ancho);
            GdipGetImageHeight(imagen.0, &mut alto);
        }
        (ancho, alto)
    }

    /// Ancho, alto y formato que GDI+ reconoce al decodificar.
    pub fn medidas(bytes: &[u8]) -> Result<(u32, u32, Option<Formato>), String> {
        let (_flujo, imagen) = abrir(bytes)?;
        let (ancho, alto) = tamano(&imagen);
        let mut guid = Guid::default();
        // SAFETY: imagen viva; `guid` recibe el formato.
        unsafe { GdipGetImageRawFormat(imagen.0, &mut guid) };
        let formato = match guid {
            g if g == PNG => Some(Formato::Png),
            g if g == JPG => Some(Formato::Jpg),
            _ => None,
        };
        Ok((ancho, alto, formato))
    }

    pub fn miniatura(bytes: &[u8], lado: u32) -> Result<Pixeles, String> {
        let error = || "Windows no pudo preparar la vista previa de la imagen.".to_string();
        let (_flujo, imagen) = abrir(bytes)?;
        let (ancho, alto) = tamano(&imagen);
        if ancho == 0 || alto == 0 {
            return Err(error());
        }
        let escala = (f64::from(lado) / f64::from(ancho.max(alto))).min(1.0);
        let destino_ancho = ((f64::from(ancho) * escala).round() as i32).max(1);
        let destino_alto = ((f64::from(alto) * escala).round() as i32).max(1);

        let mut puntero = null_mut();
        // SAFETY: GDI+ reserva los píxeles porque scan0 es nulo.
        if unsafe { GdipCreateBitmapFromScan0(destino_ancho, destino_alto, 0, ARGB_32, null_mut(), &mut puntero) } != 0 {
            return Err(error());
        }
        let lienzo = Objeto(puntero);
        {
            let mut puntero = null_mut();
            // SAFETY: el lienzo está vivo; el contexto se borra al salir de este bloque.
            if unsafe { GdipGetImageGraphicsContext(lienzo.0, &mut puntero) } != 0 {
                return Err(error());
            }
            let grafico = Grafico(puntero);
            // SAFETY: contexto e imagen vivos.
            let dibujo = unsafe {
                GdipSetInterpolationMode(grafico.0, BICUBICA_DE_ALTA_CALIDAD);
                GdipSetPixelOffsetMode(grafico.0, DESFASE_DE_ALTA_CALIDAD);
                GdipDrawImageRectI(grafico.0, imagen.0, 0, 0, destino_ancho, destino_alto)
            };
            if dibujo != 0 {
                return Err(error());
            }
        }

        let rect = Rectangulo { x: 0, y: 0, ancho: destino_ancho, alto: destino_alto };
        let mut datos = DatosDeBits { ancho: 0, alto: 0, paso: 0, formato: 0, scan0: null_mut(), reservado: 0 };
        // SAFETY: el lienzo está vivo y el rectángulo cabe en él.
        if unsafe { GdipBitmapLockBits(lienzo.0, &rect, SOLO_LECTURA, ARGB_32, &mut datos) } != 0 || datos.scan0.is_null() {
            return Err(error());
        }
        let (w, h) = (destino_ancho as usize, destino_alto as usize);
        let mut rgba = Vec::with_capacity(w * h * 4);
        for y in 0..h {
            // SAFETY: GDI+ entrega `h` filas de `paso` bytes desde scan0, con 4 bytes por píxel.
            let fila = unsafe { std::slice::from_raw_parts(datos.scan0.offset(y as isize * datos.paso as isize), w * 4) };
            for p in fila.chunks_exact(4) {
                rgba.extend_from_slice(&[p[2], p[1], p[0], p[3]]);
            }
        }
        // SAFETY: desbloquea lo que se bloqueó arriba.
        unsafe { GdipBitmapUnlockBits(lienzo.0, &mut datos) };
        Ok(Pixeles { ancho: w, alto: h, rgba })
    }
}

/// OPENFILENAMEW de comdlg32: los datos del diálogo de abrir archivo de Windows.
// Los campos los lee Windows, no el código de Rust.
#[allow(dead_code)]
#[repr(C)]
struct AbrirArchivo {
    tamano: u32,
    dueno: *mut c_void,
    instancia: *mut c_void,
    filtro: *const u16,
    filtro_propio: *mut u16,
    max_filtro_propio: u32,
    indice_filtro: u32,
    archivo: *mut u16,
    max_archivo: u32,
    titulo_archivo: *mut u16,
    max_titulo_archivo: u32,
    carpeta_inicial: *const u16,
    titulo: *const u16,
    banderas: u32,
    desfase_archivo: u16,
    desfase_extension: u16,
    extension: *const u16,
    datos: isize,
    gancho: *mut c_void,
    plantilla: *const u16,
    reservado: *mut c_void,
    reservado2: u32,
    banderas_ex: u32,
}

#[link(name = "comdlg32")]
extern "system" {
    fn GetOpenFileNameW(datos: *mut AbrirArchivo) -> i32;
}

#[link(name = "user32")]
extern "system" {
    fn GetActiveWindow() -> *mut c_void;
}

/// Abre el diálogo de Windows para elegir una imagen JPG o PNG. Devuelve la ruta o nada si la
/// persona cancela.
pub fn elegir_archivo() -> Option<PathBuf> {
    const EXPLORADOR: u32 = 0x0008_0000;
    const ARCHIVO_DEBE_EXISTIR: u32 = 0x1000;
    const RUTA_DEBE_EXISTIR: u32 = 0x0800;
    const SIN_SOLO_LECTURA: u32 = 0x0004;
    const SIN_CAMBIAR_CARPETA: u32 = 0x0008;
    let filtro: Vec<u16> = "Imágenes JPG o PNG\0*.jpg;*.jpeg;*.png\0\0".encode_utf16().collect();
    let titulo: Vec<u16> = "Elegir la imagen de la raza\0".encode_utf16().collect();
    let mut archivo = vec![0u16; 1024];
    let mut datos = AbrirArchivo {
        tamano: std::mem::size_of::<AbrirArchivo>() as u32,
        // SAFETY: GetActiveWindow no recibe argumentos; devuelve la ventana de Farming 6 o nulo.
        dueno: unsafe { GetActiveWindow() },
        instancia: std::ptr::null_mut(),
        filtro: filtro.as_ptr(),
        filtro_propio: std::ptr::null_mut(),
        max_filtro_propio: 0,
        indice_filtro: 1,
        archivo: archivo.as_mut_ptr(),
        max_archivo: archivo.len() as u32,
        titulo_archivo: std::ptr::null_mut(),
        max_titulo_archivo: 0,
        carpeta_inicial: std::ptr::null(),
        titulo: titulo.as_ptr(),
        banderas: EXPLORADOR | ARCHIVO_DEBE_EXISTIR | RUTA_DEBE_EXISTIR | SIN_SOLO_LECTURA | SIN_CAMBIAR_CARPETA,
        desfase_archivo: 0,
        desfase_extension: 0,
        extension: std::ptr::null(),
        datos: 0,
        gancho: std::ptr::null_mut(),
        plantilla: std::ptr::null(),
        reservado: std::ptr::null_mut(),
        reservado2: 0,
        banderas_ex: 0,
    };
    // SAFETY: todos los punteros apuntan a búferes vivos hasta el final de esta función.
    if unsafe { GetOpenFileNameW(&mut datos) } == 0 {
        return None;
    }
    let largo = archivo.iter().position(|&c| c == 0).unwrap_or(archivo.len());
    Some(PathBuf::from(std::ffi::OsString::from_wide(&archivo[..largo])))
}
