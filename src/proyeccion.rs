//! Lógica del sistema de archivos sin FUSE: corpus, rutas y vistas.
//!
//! **Ruta = trayectoria.** `/a/b/c` es la secuencia de `cd` aplicados desde el
//! origen. La transformación acumulada es `T = L_{−T_{k−1}(x_k)} ∘ … ∘ L_{−x_a}`
//! (traslaciones a izquierda de Möbius); cada composición agrega una giración,
//! así que `T = (t, R)` con `R` ortogonal que depende de todo el camino.
//!
//! **Contenido de un directorio** (definición):
//! * foco actual `f` = último documento de la ruta (o el origen en `/`);
//! * trayectoria `p_0 = 0, p_1 = x_{a}, …, p_n = x_{f}`;
//! * `puntaje(x) = Σ_k γ^{n−k} d(p_k, x) / Σ_k γ^{n−k}` (γ = `memoria`, por
//!   defecto 0.5): pesa más el foco actual pero recuerda por dónde se vino;
//! * se listan los `k` documentos de menor puntaje (empate: nombre), excluido
//!   el foco y solo los que están a distancia hiperbólica ≤ `radio` del foco.
//! * Con γ = 0 se recupera la semántica anterior (solo importa el último `cd`).
//! * La posición 2D de `.vista` es `exp_0(proyección de log_0(T x))` sobre un
//!   plano fijo: depende de la giración acumulada, o sea, del camino completo.

use crate::ajuste::{self, ParamsAjuste};
use crate::bola::{self, Transf, V};
use crate::embed;
use crate::es::{self, Modo};
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

pub const MAX_ARCHIVO: u64 = 1 << 20;
pub const MAX_DOCS: usize = 20_000;
pub const MAX_PROFUNDIDAD: usize = 64;
pub const RESERVADOS: [&str; 2] = [".foco", ".vista"];
const MAGIA_INDICE: &[u8; 8] = b"PFSIDX02";

#[derive(Debug, Clone)]
pub struct Doc {
    pub nombre: String,
    pub contenido: Vec<u8>,
    /// Punto en la bola de Poincaré de dimensión `Corpus::dim`.
    pub x: V,
}

/// Cómo se cargó el corpus (para informar).
#[derive(Debug, Clone, Default)]
pub struct InfoCarga {
    pub modo_io: Option<Modo>,
    pub indice: Option<String>,
    pub segundos_lectura: f64,
    pub segundos_embedding: f64,
}

#[derive(Debug, Clone)]
pub struct Corpus {
    pub docs: Vec<Doc>,
    pub dim: usize,
    pub base: [V; 2],
    pub info: InfoCarga,
}

#[derive(Debug, Clone)]
pub struct Opciones {
    pub ajuste: ParamsAjuste,
    pub modo_io: Modo,
    /// Archivo de índice: si existe y coincide se carga; si no, se escribe.
    pub indice: Option<PathBuf>,
}

impl Default for Opciones {
    fn default() -> Self {
        Opciones { ajuste: ParamsAjuste::default(), modo_io: Modo::Auto, indice: None }
    }
}

fn nombre_valido(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 200
        && !n.starts_with('.')
        && !n.starts_with('@')
        && !n.contains('/')
        && !n.contains('\0')
        && !n.ends_with(".txt")
}

fn fnv(h: &mut u64, bytes: &[u8]) {
    for &b in bytes {
        *h ^= b as u64;
        *h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
}

/// Huella del corpus + parámetros del ajuste (valida el índice).
pub fn huella(pares: &[(String, Vec<u8>)], p: &ParamsAjuste) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    fnv(&mut h, MAGIA_INDICE);
    for v in [p.dim as u64, p.vecinos as u64, p.negativos as u64, p.epocas as u64, p.lr.to_bits(), p.semilla] {
        fnv(&mut h, &v.to_le_bytes());
    }
    for (n, c) in pares {
        fnv(&mut h, n.as_bytes());
        fnv(&mut h, &[0]);
        fnv(&mut h, &(c.len() as u64).to_le_bytes());
        fnv(&mut h, c);
    }
    h
}

fn serializar(h: u64, dim: usize, xs: &[V], base: &[V; 2]) -> Vec<u8> {
    let mut b = Vec::with_capacity(32 + 8 * dim * (xs.len() + 2));
    b.extend_from_slice(MAGIA_INDICE);
    b.extend_from_slice(&h.to_le_bytes());
    b.extend_from_slice(&(dim as u64).to_le_bytes());
    b.extend_from_slice(&(xs.len() as u64).to_le_bytes());
    for v in xs.iter().chain(base.iter()) {
        for x in v {
            b.extend_from_slice(&x.to_le_bytes());
        }
    }
    b
}

fn deserializar(b: &[u8], h: u64, dim: usize, n: usize) -> Option<(Vec<V>, [V; 2])> {
    let u = |i: usize| -> Option<u64> { Some(u64::from_le_bytes(b.get(i..i + 8)?.try_into().ok()?)) };
    if b.get(0..8)? != MAGIA_INDICE || u(8)? != h || u(16)? != dim as u64 || u(24)? != n as u64 {
        return None;
    }
    if b.len() != 32 + 8 * dim * (n + 2) {
        return None;
    }
    let mut vs: Vec<V> = b[32..]
        .chunks_exact(8 * dim)
        .map(|c| c.chunks_exact(8).map(|x| f64::from_le_bytes(x.try_into().unwrap())).collect())
        .collect();
    if vs.iter().flatten().any(|x| !x.is_finite()) || vs[..n].iter().any(|v| bola::norma(v) >= 1.0) {
        return None;
    }
    let e2 = vs.pop()?;
    let e1 = vs.pop()?;
    Some((vs, [e1, e2]))
}

impl Corpus {
    /// Construye el corpus a partir de pares (nombre, contenido) con opciones por defecto.
    pub fn desde_textos(pares: Vec<(String, Vec<u8>)>) -> io::Result<Corpus> {
        Corpus::desde_textos_con(pares, &Opciones::default())
    }

    pub fn desde_textos_con(pares: Vec<(String, Vec<u8>)>, o: &Opciones) -> io::Result<Corpus> {
        let mut pares = pares;
        pares.sort_by(|a, b| a.0.cmp(&b.0));
        for w in pares.windows(2) {
            if w[0].0 == w[1].0 {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("nombre repetido: {}", w[0].0)));
            }
        }
        if pares.len() > MAX_DOCS {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "demasiados documentos"));
        }
        for (n, _) in &pares {
            if !nombre_valido(n) {
                return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("nombre inválido: {n:?}")));
            }
        }
        let dim = o.ajuste.dim;
        if !(2..=64).contains(&dim) {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "dim debe estar entre 2 y 64"));
        }
        let t0 = std::time::Instant::now();
        let h = huella(&pares, &o.ajuste);
        let mut info = InfoCarga::default();
        let mut cargado = None;
        if let Some(ruta) = &o.indice {
            if ruta.exists() {
                let (m, mut datos) = es::leer_todos(std::slice::from_ref(ruta), o.modo_io)?;
                info.modo_io = Some(m);
                cargado = deserializar(&datos.pop().unwrap_or_default(), h, dim, pares.len());
                info.indice = Some(if cargado.is_some() {
                    format!("cargado de {}", ruta.display())
                } else {
                    format!("{} no coincide: se recalcula", ruta.display())
                });
            }
        }
        let (xs, base) = match cargado {
            Some(c) => c,
            None => {
                let textos: Vec<String> = pares.iter().map(|(_, c)| String::from_utf8_lossy(c).into_owned()).collect();
                let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
                let tf = embed::tfidf(&refs);
                let vec = ajuste::vecinos_tfidf(&tf, o.ajuste.vecinos);
                let xs = ajuste::ajustar(&vec, o.ajuste);
                let base = ajuste::base_2d(&xs, dim);
                if let Some(ruta) = &o.indice {
                    let tmp = ruta.with_extension("tmp");
                    std::fs::write(&tmp, serializar(h, dim, &xs, &base))?;
                    std::fs::rename(&tmp, ruta)?;
                    info.indice.get_or_insert_with(|| format!("escrito en {}", ruta.display()));
                }
                (xs, base)
            }
        };
        info.segundos_embedding = t0.elapsed().as_secs_f64();
        Ok(Corpus {
            docs: pares
                .into_iter()
                .zip(xs)
                .map(|((nombre, contenido), x)| Doc { nombre, contenido, x })
                .collect(),
            dim,
            base,
            info,
        })
    }

    /// Lee los `*.txt` (no recursivo) de un directorio con io_uring o pread.
    pub fn desde_directorio(dir: &Path) -> io::Result<Corpus> {
        Corpus::desde_directorio_con(dir, &Opciones::default())
    }

    pub fn desde_directorio_con(dir: &Path, o: &Opciones) -> io::Result<Corpus> {
        let (modo, pares, segs) = leer_directorio(dir, o.modo_io)?;
        let mut c = Corpus::desde_textos_con(pares, o)?;
        c.info.modo_io = Some(modo);
        c.info.segundos_lectura = segs;
        Ok(c)
    }

    pub fn indice(&self, nombre: &str) -> Option<u32> {
        self.docs
            .binary_search_by(|d| d.nombre.as_str().cmp(nombre))
            .ok()
            .map(|i| i as u32)
    }
}

/// Pares (nombre, contenido) de un corpus.
pub type Pares = Vec<(String, Vec<u8>)>;

/// Lista y lee los `*.txt` de `dir`. Devuelve (modo efectivo, pares, segundos).
pub fn leer_directorio(dir: &Path, modo: Modo) -> io::Result<(Modo, Pares, f64)> {
    let mut nombres = Vec::new();
    let mut rutas = Vec::new();
    for e in std::fs::read_dir(dir)? {
        let e = e?;
        if !e.file_type()?.is_file() {
            continue; // ignora symlinks, subdirectorios, etc.
        }
        let fname = e.file_name();
        let Some(fname) = fname.to_str() else { continue };
        let Some(stem) = fname.strip_suffix(".txt") else { continue };
        if !nombre_valido(stem) {
            continue;
        }
        if e.metadata()?.len() > MAX_ARCHIVO {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, format!("{fname} supera 1 MiB")));
        }
        nombres.push(stem.to_string());
        rutas.push(e.path());
    }
    let t0 = std::time::Instant::now();
    let (m, datos) = es::leer_todos(&rutas, modo)?;
    Ok((m, nombres.into_iter().zip(datos).collect(), t0.elapsed().as_secs_f64()))
}

/// Parámetros de la proyección.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    /// Máximo de documentos visibles por directorio.
    pub k: usize,
    /// Radio hiperbólico máximo (desde el foco actual) de lo visible.
    pub radio: f64,
    /// γ ∈ [0, 1): peso de la historia de la trayectoria.
    pub memoria: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params { k: 10, radio: 12.0, memoria: 0.5 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entrada {
    pub doc: u32,
    /// Distancia hiperbólica al foco actual (= |T x| en distancia al origen).
    pub distancia: f64,
    /// Puntaje de trayectoria (criterio de orden).
    pub puntaje: f64,
    /// Posición en el disco 2D de visualización (depende de la giración acumulada).
    pub w2: (f64, f64),
}

#[derive(Debug, Clone)]
pub struct Vista {
    pub ruta: Vec<u32>,
    pub transformacion: Transf,
    pub entradas: Vec<Entrada>,
}

/// Transformación acumulada de una ruta de `cd`.
pub fn transformacion(corpus: &Corpus, ruta: &[u32]) -> Transf {
    ruta.iter()
        .fold(Transf::identidad(corpus.dim), |t, &i| t.enfocar(&corpus.docs[i as usize].x))
}

/// Pesos normalizados γ^{n−k} de la trayectoria `p_0 = 0, p_1..p_n`.
pub fn pesos(n: usize, memoria: f64) -> Vec<f64> {
    let mut w: Vec<f64> = (0..=n).map(|k| memoria.powi((n - k) as i32)).collect();
    if memoria == 0.0 {
        w = (0..=n).map(|k| if k == n { 1.0 } else { 0.0 }).collect();
    }
    let s: f64 = w.iter().sum();
    w.iter().map(|x| x / s).collect()
}

/// Calcula la vista (contenido) para una ruta.
pub fn vista(corpus: &Corpus, ruta: &[u32], params: Params) -> Vista {
    let t = transformacion(corpus, ruta);
    let foco = ruta.last().copied();
    let origen = vec![0.0; corpus.dim];
    let xf: &[f64] = foco.map(|i| corpus.docs[i as usize].x.as_slice()).unwrap_or(&origen);
    let w = pesos(ruta.len(), params.memoria);
    // trayectoria con pesos no despreciables
    let tray: Vec<(&[f64], f64)> = std::iter::once(origen.as_slice())
        .chain(ruta.iter().map(|&i| corpus.docs[i as usize].x.as_slice()))
        .zip(w.iter().copied())
        .filter(|(_, w)| *w > 1e-12)
        .collect();
    let evaluar = |i: usize| -> Option<Entrada> {
        if Some(i as u32) == foco {
            return None;
        }
        let d = &corpus.docs[i];
        let dist = bola::distancia(xf, &d.x);
        if dist > params.radio {
            return None;
        }
        let puntaje = tray.iter().map(|(p, w)| w * bola::distancia(p, &d.x)).sum();
        Some(Entrada { doc: i as u32, distancia: dist, puntaje, w2: (0.0, 0.0) })
    };
    // Cada documento se evalúa por separado: repartir en hilos no cambia el resultado.
    let n = corpus.docs.len();
    let trabajo = n * tray.len();
    let hilos = if trabajo < 50_000 {
        1
    } else {
        std::thread::available_parallelism().map(|h| h.get()).unwrap_or(1).min(8)
    };
    let mut entradas: Vec<Entrada> = if hilos == 1 {
        (0..n).filter_map(evaluar).collect()
    } else {
        let trozo = n.div_ceil(hilos);
        std::thread::scope(|s| {
            let hs: Vec<_> = (0..hilos)
                .map(|h| {
                    let evaluar = &evaluar;
                    s.spawn(move || (h * trozo..((h + 1) * trozo).min(n)).filter_map(evaluar).collect::<Vec<_>>())
                })
                .collect();
            hs.into_iter().flat_map(|h| h.join().expect("hilo de vista")).collect()
        })
    };
    // Orden total (puntaje, nombre único): elegir los k primeros y ordenarlos da
    // exactamente lo mismo que ordenar todo.
    let cmp = |a: &Entrada, b: &Entrada| {
        a.puntaje
            .total_cmp(&b.puntaje)
            .then_with(|| corpus.docs[a.doc as usize].nombre.cmp(&corpus.docs[b.doc as usize].nombre))
    };
    if params.k == 0 {
        entradas.clear();
    } else if entradas.len() > params.k {
        entradas.select_nth_unstable_by(params.k - 1, cmp);
        entradas.truncate(params.k);
    }
    entradas.sort_by(cmp);
    for e in &mut entradas {
        e.w2 = ajuste::a_disco(&t.aplicar(&corpus.docs[e.doc as usize].x), &corpus.base);
    }
    Vista { ruta: ruta.to_vec(), transformacion: t, entradas }
}

pub fn ruta_texto(corpus: &Corpus, ruta: &[u32]) -> String {
    if ruta.is_empty() {
        return "/".into();
    }
    let mut s = String::new();
    for &i in ruta {
        s.push('/');
        s.push_str(&corpus.docs[i as usize].nombre);
    }
    s
}

/// Texto del archivo `.vista`: la tabla ordenada por puntaje de trayectoria.
pub fn texto_vista(corpus: &Corpus, v: &Vista) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# ruta: {}", ruta_texto(corpus, &v.ruta));
    let _ = writeln!(s, "# rango puntaje_trayectoria distancia_al_foco posicion_disco_2d nombre");
    for (r, e) in v.entradas.iter().enumerate() {
        let _ = writeln!(
            s,
            "{:>3} {:>12.9} {:>12.9} ({:+.6}, {:+.6}) {}",
            r + 1,
            e.puntaje,
            e.distancia,
            e.w2.0,
            e.w2.1,
            corpus.docs[e.doc as usize].nombre
        );
    }
    s
}

/// Texto del archivo `.foco`.
pub fn texto_foco(corpus: &Corpus, v: &Vista, params: Params) -> String {
    let t = &v.transformacion;
    let mut s = String::new();
    let _ = writeln!(s, "ruta: {}", ruta_texto(corpus, &v.ruta));
    let _ = writeln!(s, "profundidad: {}", v.ruta.len());
    let _ = writeln!(s, "dimension_bola: {}", corpus.dim);
    match v.ruta.last() {
        None => {
            let _ = writeln!(s, "foco: origen (sin transformación)");
        }
        Some(&i) => {
            let img = t.aplicar(&corpus.docs[i as usize].x);
            let _ = writeln!(s, "foco: {}", corpus.docs[i as usize].nombre);
            let _ = writeln!(s, "foco_imagen_norma: {:.3e}", bola::norma(&img));
        }
    }
    let _ = writeln!(s, "transformacion: z -> t (+) R z   (suma de Möbius, R = giración acumulada)");
    let _ = writeln!(s, "t_norma: {:.9}", bola::norma(&t.t));
    let _ = writeln!(s, "R_error_ortogonal: {:.3e}", t.error_ortogonal());
    let _ = writeln!(s, "memoria_gamma: {}", params.memoria);
    let _ = writeln!(s, "trayectoria (peso, distancia_al_foco_actual, nombre):");
    let w = pesos(v.ruta.len(), params.memoria);
    let origen = vec![0.0; corpus.dim];
    for (k, peso) in w.iter().enumerate() {
        let (p, nombre): (&[f64], &str) = if k == 0 {
            (&origen, "(origen)")
        } else {
            let d = &corpus.docs[v.ruta[k - 1] as usize];
            (&d.x, &d.nombre)
        };
        let dd = bola::distancia_al_origen(&t.aplicar(p));
        let _ = writeln!(s, "  {peso:.6} {dd:.9} {nombre}");
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let t = [
            ("gato", "el gato duerme en el sillón y ronronea"),
            ("perro", "el perro ladra y juega en el parque con el gato"),
            ("kernel", "el kernel linux planifica hilos con ebpf"),
            ("ebpf", "programas ebpf verificados corren en el kernel linux"),
            ("pan", "receta de pan con harina agua sal y levadura"),
            ("pizza", "la pizza lleva harina tomate y queso"),
        ];
        Corpus::desde_textos(t.iter().map(|(n, c)| (n.to_string(), c.as_bytes().to_vec())).collect()).unwrap()
    }
    const TODO: Params = Params { k: 100, radio: 100.0, memoria: 0.5 };

    #[test]
    fn raiz_ordenada_por_distancia() {
        // En la raíz la trayectoria es solo el origen: puntaje = distancia.
        let c = corpus();
        let v = vista(&c, &[], TODO);
        assert_eq!(v.entradas.len(), 6);
        for w in v.entradas.windows(2) {
            assert!(w[0].distancia <= w[1].distancia);
        }
        for e in &v.entradas {
            assert!((e.puntaje - e.distancia).abs() < 1e-12);
        }
    }

    #[test]
    fn cd_lleva_el_foco_al_origen() {
        let c = corpus();
        for i in 0..6u32 {
            for j in 0..6u32 {
                let t = transformacion(&c, &[i, j]);
                assert!(bola::norma(&t.aplicar(&c.docs[j as usize].x)) < 1e-12);
            }
        }
    }

    #[test]
    fn distancias_de_la_vista_son_isometricas() {
        // La distancia al foco medida en el marco transformado (|T x|) es la original.
        let c = corpus();
        let v = vista(&c, &[2, 4], TODO);
        for e in &v.entradas {
            let en_marco = bola::distancia_al_origen(&v.transformacion.aplicar(&c.docs[e.doc as usize].x));
            assert!((en_marco - e.distancia).abs() < 1e-9);
        }
        assert!(v.entradas.iter().all(|e| e.doc != 4), "el foco no se lista a sí mismo");
    }

    #[test]
    fn determinista_para_la_misma_ruta() {
        let c = corpus();
        let p = Params::default();
        let a = texto_vista(&c, &vista(&c, &[0, 3, 1], p));
        assert_eq!(a, texto_vista(&c, &vista(&c, &[0, 3, 1], p)));
        let c2 = corpus();
        assert_eq!(a, texto_vista(&c2, &vista(&c2, &[0, 3, 1], p)));
    }

    #[test]
    fn el_camino_importa() {
        // Mismo foco final (pan), distinto camino: cambian los puntajes y la
        // giración acumulada (posiciones 2D), no las distancias al foco.
        let c = corpus();
        let pan = c.indice("pan").unwrap();
        let v1 = vista(&c, &[c.indice("gato").unwrap(), pan], TODO);
        let v2 = vista(&c, &[c.indice("kernel").unwrap(), pan], TODO);
        let dist = |v: &Vista, j: u32| v.entradas.iter().find(|e| e.doc == j).unwrap().clone();
        let pizza = c.indice("pizza").unwrap();
        assert!((dist(&v1, pizza).distancia - dist(&v2, pizza).distancia).abs() < 1e-9);
        assert!((dist(&v1, pizza).puntaje - dist(&v2, pizza).puntaje).abs() > 1e-6);
        assert_ne!(texto_vista(&c, &v1), texto_vista(&c, &v2));
        assert!(bola::norma(&v1.transformacion.t) < 1e-12 || v1.transformacion.r != v2.transformacion.r);
    }

    #[test]
    fn memoria_cero_es_la_semantica_anterior() {
        let c = corpus();
        let p0 = Params { memoria: 0.0, ..TODO };
        let v = vista(&c, &[1, 3], p0);
        for w in v.entradas.windows(2) {
            assert!(w[0].distancia <= w[1].distancia);
        }
        for e in &v.entradas {
            assert!((e.puntaje - e.distancia).abs() < 1e-12);
        }
    }

    #[test]
    fn seleccion_k_igual_a_orden_completo() {
        let c = corpus();
        for k in 0..7 {
            let p = Params { k, ..TODO };
            let completa = vista(&c, &[2], TODO);
            let parcial = vista(&c, &[2], p);
            assert_eq!(parcial.entradas[..], completa.entradas[..k.min(completa.entradas.len())]);
        }
    }

    #[test]
    fn pesos_normalizados() {
        for n in 0..10 {
            for g in [0.0, 0.3, 0.5, 0.9] {
                let w = pesos(n, g);
                assert!((w.iter().sum::<f64>() - 1.0).abs() < 1e-12);
                assert!(w[n] >= *w.iter().take(n).fold(&0.0, |a, b| if b > a { b } else { a }));
            }
        }
    }

    #[test]
    fn filtros_k_y_radio() {
        let c = corpus();
        assert_eq!(vista(&c, &[], Params { k: 2, ..TODO }).entradas.len(), 2);
        assert!(vista(&c, &[], Params { radio: 0.0, ..TODO }).entradas.iter().all(|e| e.distancia == 0.0));
    }

    #[test]
    fn nombres_invalidos_y_repetidos() {
        let mk = |n: &str| Corpus::desde_textos(vec![(n.to_string(), b"x".to_vec())]);
        assert!(mk(".foco").is_err());
        assert!(mk("a/b").is_err());
        assert!(mk("x.txt").is_err());
        assert!(mk("").is_err());
        assert!(Corpus::desde_textos(vec![("a".into(), vec![]), ("a".into(), vec![])]).is_err());
    }

    #[test]
    fn foco_texto_reporta_origen() {
        let c = corpus();
        let i = c.indice("perro").unwrap();
        let s = texto_foco(&c, &vista(&c, &[i], Params::default()), Params::default());
        assert!(s.contains("foco: perro"));
        assert!(s.contains("dimension_bola: 16"));
    }

    #[test]
    fn indice_ida_y_vuelta_y_rechazo() {
        let dir = std::env::temp_dir().join(format!("poincare-idx-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let idx = dir.join("indice.bin");
        let pares: Vec<(String, Vec<u8>)> =
            corpus().docs.iter().map(|d| (d.nombre.clone(), d.contenido.clone())).collect();
        let o = Opciones { indice: Some(idx.clone()), ..Default::default() };
        let a = Corpus::desde_textos_con(pares.clone(), &o).unwrap();
        assert!(a.info.indice.as_deref().unwrap().starts_with("escrito"));
        let b = Corpus::desde_textos_con(pares.clone(), &o).unwrap();
        assert!(b.info.indice.as_deref().unwrap().starts_with("cargado"));
        for (x, y) in a.docs.iter().zip(&b.docs) {
            assert_eq!(x.x, y.x);
        }
        // otro corpus con el mismo índice: no coincide la huella, se recalcula
        let mut otros = pares.clone();
        otros[0].1.push(b'!');
        let c = Corpus::desde_textos_con(otros, &o).unwrap();
        assert!(c.info.indice.as_deref().unwrap().contains("no coincide"));
        // índice corrupto (del corpus correcto): se rechaza
        let e = Corpus::desde_textos_con(pares.clone(), &o).unwrap();
        assert!(e.info.indice.as_deref().unwrap().contains("no coincide"));
        assert!(Corpus::desde_textos_con(pares.clone(), &o).unwrap().info.indice.unwrap().starts_with("cargado"));
        let mut bytes = std::fs::read(&idx).unwrap();
        let n = bytes.len();
        bytes[n - 1] ^= 0xFF;
        bytes.truncate(n - 3);
        std::fs::write(&idx, bytes).unwrap();
        let d = Corpus::desde_textos_con(pares, &o).unwrap();
        assert!(d.info.indice.as_deref().unwrap().contains("no coincide"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
