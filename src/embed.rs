//! Embedding semántico local y determinista (sin red, sin modelos externos).
//!
//! 1. Rasgos: palabras (en minúsculas) + trigramas de caracteres de cada palabra
//!    con marcadores de borde (`<pa`, `pal`, …, `ra>`).
//! 2. "Hashing trick" con signo: FNV-1a de 64 bits a `DIM` cubetas.
//! 3. Peso TF-IDF: `(1 + ln tf) · (ln((1+N)/(1+df)) + 1)`, normalizado L2.
//! 4. Proyección a 2D con PCA (iteración de potencia con vector inicial fijo y
//!    signos fijados), luego escala para que el documento más lejano quede en
//!    |z| = `RADIO_MAX`. Resultado: un punto del disco de Poincaré por documento.
//!
//! Todo es determinista: el mismo corpus produce exactamente los mismos puntos.

use crate::hiper::C;

pub const DIM: usize = 1024;
pub const RADIO_MAX: f64 = 0.9;
const ITER_PCA: usize = 300;

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Rasgos (palabras y trigramas) de un texto.
pub fn rasgos(texto: &str) -> Vec<String> {
    let mut out = Vec::new();
    for palabra in texto
        .split(|c: char| !c.is_alphanumeric())
        .filter(|p| p.chars().count() >= 2)
    {
        let p = palabra.to_lowercase();
        out.push(format!("w:{p}"));
        let cs: Vec<char> = format!("<{p}>").chars().collect();
        for v in cs.windows(3) {
            out.push(format!("t:{}", v.iter().collect::<String>()));
        }
    }
    out
}

/// Vector de frecuencias con hashing con signo.
fn tf(texto: &str) -> Vec<f64> {
    let mut v = vec![0.0; DIM];
    for r in rasgos(texto) {
        let h = fnv1a(r.as_bytes());
        let i = (h % DIM as u64) as usize;
        let signo = if (h >> 63) == 0 { 1.0 } else { -1.0 };
        v[i] += signo;
    }
    v
}

fn norma(v: &[f64]) -> f64 {
    v.iter().map(|x| x * x).sum::<f64>().sqrt()
}

fn normalizar(v: &mut [f64]) {
    let n = norma(v);
    if n > 0.0 {
        v.iter_mut().for_each(|x| *x /= n);
    }
}

fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Vectores TF-IDF normalizados (alta dimensión) de un corpus.
pub fn tfidf(textos: &[&str]) -> Vec<Vec<f64>> {
    let n = textos.len() as f64;
    let crudos: Vec<Vec<f64>> = textos.iter().map(|t| tf(t)).collect();
    let mut df = vec![0.0; DIM];
    for v in &crudos {
        for (i, x) in v.iter().enumerate() {
            if *x != 0.0 {
                df[i] += 1.0;
            }
        }
    }
    let idf: Vec<f64> = df.iter().map(|d| ((1.0 + n) / (1.0 + d)).ln() + 1.0).collect();
    crudos
        .into_iter()
        .map(|v| {
            let mut w: Vec<f64> = v
                .iter()
                .zip(&idf)
                .map(|(x, i)| if *x == 0.0 { 0.0 } else { x.signum() * (1.0 + x.abs().ln()) * i })
                .collect();
            normalizar(&mut w);
            w
        })
        .collect()
}

/// Componente principal por iteración de potencia, ortogonal a `previas`.
fn componente(x: &[Vec<f64>], previas: &[Vec<f64>]) -> Vec<f64> {
    // vector inicial fijo, no aleatorio
    let mut v: Vec<f64> = (0..DIM).map(|i| 1.0 + (i % 7) as f64 * 0.1).collect();
    let ortogonalizar = |v: &mut Vec<f64>| {
        for p in previas {
            let k = dot(v, p);
            v.iter_mut().zip(p).for_each(|(a, b)| *a -= k * b);
        }
    };
    ortogonalizar(&mut v);
    normalizar(&mut v);
    for _ in 0..ITER_PCA {
        // w = Xᵀ (X v)
        let mut w = vec![0.0; DIM];
        for fila in x {
            let s = dot(fila, &v);
            w.iter_mut().zip(fila).for_each(|(a, b)| *a += s * b);
        }
        ortogonalizar(&mut w);
        if norma(&w) == 0.0 {
            break;
        }
        normalizar(&mut w);
        v = w;
    }
    // Fijar el signo: el documento con mayor |proyección| queda positivo.
    let proy: Vec<f64> = x.iter().map(|f| dot(f, &v)).collect();
    if let Some(m) = proy
        .iter()
        .copied()
        .max_by(|a, b| a.abs().partial_cmp(&b.abs()).unwrap_or(std::cmp::Ordering::Equal))
    {
        if m < 0.0 {
            v.iter_mut().for_each(|a| *a = -*a);
        }
    }
    v
}

/// Puntos del disco de Poincaré para cada texto.
pub fn puntos(textos: &[&str]) -> Vec<C> {
    if textos.is_empty() {
        return Vec::new();
    }
    let mut x = tfidf(textos);
    // centrar
    let n = x.len() as f64;
    let mut media = vec![0.0; DIM];
    for f in &x {
        media.iter_mut().zip(f).for_each(|(a, b)| *a += b / n);
    }
    for f in &mut x {
        f.iter_mut().zip(&media).for_each(|(a, b)| *a -= b);
    }
    let v1 = componente(&x, &[]);
    let v2 = componente(&x, std::slice::from_ref(&v1));
    let ys: Vec<C> = x.iter().map(|f| C::new(dot(f, &v1), dot(f, &v2))).collect();
    let rmax = ys.iter().map(|y| y.abs()).fold(0.0, f64::max);
    if rmax == 0.0 || !rmax.is_finite() {
        return vec![C::CERO; ys.len()];
    }
    ys.into_iter().map(|y| y.escalar(RADIO_MAX / rmax)).collect()
}

/// Similitud coseno entre vectores TF-IDF (para medir calidad, no se usa en el FS).
pub fn coseno(a: &[f64], b: &[f64]) -> f64 {
    dot(a, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rasgos_basicos() {
        let r = rasgos("Hola, mundo!");
        assert!(r.contains(&"w:hola".to_string()));
        assert!(r.contains(&"t:<ho".to_string()));
        assert!(r.contains(&"t:do>".to_string()));
    }

    #[test]
    fn determinista() {
        let t = ["el gato duerme", "kernel linux ebpf", "receta de pan", "gato y perro"];
        assert_eq!(puntos(&t), puntos(&t));
    }

    #[test]
    fn todos_dentro_del_disco() {
        let t = ["a b c", "d e f", "g h i", "aa bb", "cc dd ee ff"];
        for p in puntos(&t) {
            assert!(p.abs() <= RADIO_MAX + 1e-12);
        }
    }

    #[test]
    fn corpus_trivial() {
        assert!(puntos(&[]).is_empty());
        assert_eq!(puntos(&["solo uno"]), vec![C::CERO]);
        assert_eq!(puntos(&["igual", "igual"]), vec![C::CERO, C::CERO]);
    }

    #[test]
    fn tfidf_normalizado() {
        for v in tfidf(&["uno dos", "tres cuatro cinco"]) {
            assert!((norma(&v) - 1.0).abs() < 1e-12);
        }
    }
}
