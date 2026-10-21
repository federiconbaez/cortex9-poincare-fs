//! Lógica del sistema de archivos sin FUSE: corpus, rutas y vistas.
//!
//! Una ruta `/a/b/c` NO es un árbol guardado en disco: es la secuencia de
//! `cd` aplicados. Partiendo de la identidad, cada `cd x` toma la posición
//! actual (transformada) de `x` y compone la transformación de Möbius que la
//! lleva al origen. El contenido del directorio es la proyección: los
//! documentos más cercanos al origen en distancia hiperbólica.
//!
//! Como la ruta determina la transformación, `ls` es determinista para la
//! misma posición (lo que pidió la revisión de seguridad) y la caché de
//! dentries del kernel nunca queda inconsistente.

use crate::embed;
use crate::hiper::{distancia_al_origen, Mobius, C};
use std::fmt::Write as _;
use std::io;
use std::path::Path;

pub const MAX_ARCHIVO: u64 = 1 << 20;
pub const MAX_DOCS: usize = 10_000;
pub const MAX_PROFUNDIDAD: usize = 64;
pub const RESERVADOS: [&str; 2] = [".foco", ".vista"];

#[derive(Debug, Clone)]
pub struct Doc {
    pub nombre: String,
    pub contenido: Vec<u8>,
    pub z: C,
}

#[derive(Debug, Clone)]
pub struct Corpus {
    pub docs: Vec<Doc>,
}

fn nombre_valido(n: &str) -> bool {
    !n.is_empty()
        && n.len() <= 200
        && !n.starts_with('.')
        && !n.contains('/')
        && !n.contains('\0')
        && !n.ends_with(".txt")
}

impl Corpus {
    /// Construye el corpus a partir de pares (nombre, contenido).
    pub fn desde_textos(pares: Vec<(String, Vec<u8>)>) -> io::Result<Corpus> {
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
        let textos: Vec<String> = pares.iter().map(|(_, c)| String::from_utf8_lossy(c).into_owned()).collect();
        let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
        let zs = embed::puntos(&refs);
        Ok(Corpus {
            docs: pares
                .into_iter()
                .zip(zs)
                .map(|((nombre, contenido), z)| Doc { nombre, contenido, z })
                .collect(),
        })
    }

    /// Lee los `*.txt` (no recursivo) de un directorio. El nombre es el archivo sin `.txt`.
    pub fn desde_directorio(dir: &Path) -> io::Result<Corpus> {
        let mut pares = Vec::new();
        for e in std::fs::read_dir(dir)? {
            let e = e?;
            let ft = e.file_type()?;
            if !ft.is_file() {
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
            pares.push((stem.to_string(), std::fs::read(e.path())?));
        }
        Corpus::desde_textos(pares)
    }

    pub fn indice(&self, nombre: &str) -> Option<u32> {
        self.docs
            .binary_search_by(|d| d.nombre.as_str().cmp(nombre))
            .ok()
            .map(|i| i as u32)
    }
}

/// Parámetros de la proyección.
#[derive(Debug, Clone, Copy)]
pub struct Params {
    /// Máximo de documentos visibles por directorio.
    pub k: usize,
    /// Radio hiperbólico máximo de lo visible.
    pub radio: f64,
}

impl Default for Params {
    fn default() -> Self {
        Params { k: 10, radio: 6.0 }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Entrada {
    pub doc: u32,
    pub distancia: f64,
    /// Posición del documento después de la transformación acumulada.
    pub w: C,
}

#[derive(Debug, Clone)]
pub struct Vista {
    pub ruta: Vec<u32>,
    pub transformacion: Mobius,
    pub entradas: Vec<Entrada>,
}

/// Transformación acumulada de una ruta de `cd`.
pub fn transformacion(corpus: &Corpus, ruta: &[u32]) -> Mobius {
    let mut t = Mobius::IDENTIDAD;
    for &i in ruta {
        let p = t.aplicar(corpus.docs[i as usize].z);
        t = Mobius::al_origen(p).componer(&t);
    }
    t
}

/// Calcula la vista (contenido) para una ruta.
pub fn vista(corpus: &Corpus, ruta: &[u32], params: Params) -> Vista {
    let t = transformacion(corpus, ruta);
    let foco = ruta.last().copied();
    let mut entradas: Vec<Entrada> = corpus
        .docs
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i as u32) != foco)
        .map(|(i, d)| {
            let w = t.aplicar(d.z);
            Entrada { doc: i as u32, distancia: distancia_al_origen(w), w }
        })
        .filter(|e| e.distancia <= params.radio)
        .collect();
    // Orden total y determinista: distancia, y a igual distancia, nombre.
    entradas.sort_by(|a, b| {
        a.distancia
            .total_cmp(&b.distancia)
            .then_with(|| corpus.docs[a.doc as usize].nombre.cmp(&corpus.docs[b.doc as usize].nombre))
    });
    entradas.truncate(params.k);
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

fn fmt_c(z: C) -> String {
    format!("({:+.9}, {:+.9})", z.re, z.im)
}

/// Texto del archivo `.vista`: la tabla ordenada por distancia.
pub fn texto_vista(corpus: &Corpus, v: &Vista) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# ruta: {}", ruta_texto(corpus, &v.ruta));
    let _ = writeln!(s, "# rango distancia_hiperbolica posicion_transformada nombre");
    for (r, e) in v.entradas.iter().enumerate() {
        let _ = writeln!(
            s,
            "{:>3} {:>12.9} {} {}",
            r + 1,
            e.distancia,
            fmt_c(e.w),
            corpus.docs[e.doc as usize].nombre
        );
    }
    s
}

/// Texto del archivo `.foco`: dónde estamos y con qué transformación.
pub fn texto_foco(corpus: &Corpus, v: &Vista) -> String {
    let t = v.transformacion;
    let mut s = String::new();
    let _ = writeln!(s, "ruta: {}", ruta_texto(corpus, &v.ruta));
    let _ = writeln!(s, "profundidad: {}", v.ruta.len());
    match v.ruta.last() {
        None => {
            let _ = writeln!(s, "foco: origen (sin transformación)");
        }
        Some(&i) => {
            let d = &corpus.docs[i as usize];
            let img = t.aplicar(d.z);
            let _ = writeln!(s, "foco: {}", d.nombre);
            let _ = writeln!(s, "foco_coordenada_original: {}", fmt_c(d.z));
            let _ = writeln!(s, "foco_imagen: {}  |imagen| = {:.3e}", fmt_c(img), img.abs());
        }
    }
    let _ = writeln!(s, "mobius: z -> (alfa*z + beta) / (conj(beta)*z + conj(alfa))");
    let _ = writeln!(s, "alfa: {}", fmt_c(t.alfa));
    let _ = writeln!(s, "beta: {}", fmt_c(t.beta));
    let _ = writeln!(s, "det: {:.15}", t.det());
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hiper::distancia;

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

    #[test]
    fn raiz_ordenada_por_distancia() {
        let c = corpus();
        let v = vista(&c, &[], Params { k: 100, radio: 100.0 });
        assert_eq!(v.entradas.len(), 6);
        for w in v.entradas.windows(2) {
            assert!(w[0].distancia <= w[1].distancia);
        }
    }

    #[test]
    fn cd_lleva_el_foco_al_origen() {
        let c = corpus();
        for i in 0..6u32 {
            for j in 0..6u32 {
                let t = transformacion(&c, &[i, j]);
                assert!(t.aplicar(c.docs[j as usize].z).abs() < 1e-12);
            }
        }
    }

    #[test]
    fn distancias_de_la_vista_son_isometricas() {
        // Tras `cd a/b`, la distancia de x al origen es d(z_b, z_x) en el disco original.
        let c = corpus();
        let v = vista(&c, &[2, 4], Params { k: 100, radio: 100.0 });
        for e in &v.entradas {
            let d0 = distancia(c.docs[4].z, c.docs[e.doc as usize].z);
            assert!((d0 - e.distancia).abs() < 1e-9 * d0.max(1.0));
        }
        assert!(v.entradas.iter().all(|e| e.doc != 4), "el foco no se lista a sí mismo");
    }

    #[test]
    fn determinista_para_la_misma_ruta() {
        let c = corpus();
        let p = Params::default();
        let a = texto_vista(&c, &vista(&c, &[0, 3, 1], p));
        let b = texto_vista(&c, &vista(&c, &[0, 3, 1], p));
        assert_eq!(a, b);
        let c2 = corpus();
        assert_eq!(a, texto_vista(&c2, &vista(&c2, &[0, 3, 1], p)));
    }

    #[test]
    fn filtros_k_y_radio() {
        let c = corpus();
        assert_eq!(vista(&c, &[], Params { k: 2, radio: 100.0 }).entradas.len(), 2);
        assert!(vista(&c, &[], Params { k: 100, radio: 0.0 }).entradas.iter().all(|e| e.distancia == 0.0));
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
        let s = texto_foco(&c, &vista(&c, &[i], Params::default()));
        assert!(s.contains("foco: perro"));
        assert!(s.contains("det: 1.0000000"));
    }
}
