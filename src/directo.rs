//! Acceso directo y alcance completo (v0.3).
//!
//! * [`IndiceBola`]: k vecinos hiperbólicos exactos. Disposición por dimensión
//!   (SoA, f32 contiguo) y factores `a_i = 1/(1−|x_i|²)` precalculados: como
//!   `cosh d(x,y) = 1 + 2|x−y|² a_x a_y`, ordenar por `|q−x_i|²·a_i` es ordenar por
//!   distancia hiperbólica. Prefiltro en f32 (k + 16 candidatos) y reordenamiento
//!   exacto en f64 con [`bola::distancia`].
//! * [`IndiceLexico`]: TF‑IDF (mismos vectores que `embed::vectorizar`) guardado
//!   por columnas en f32; una consulta es dispersa, así que puntuar cuesta
//!   O(no‑ceros × n) en vez de O(1024 × n).
//! * [`Cobertura`]: árbol de alcance. Cada documento tiene un padre; la vista de
//!   cualquier ruta cuyo foco es el padre incluye al hijo (si no estaba ya entre
//!   los k). Así todo documento aparece en alguna vista y es alcanzable con
//!   `ls`/`cd` desde `/`, sin importar γ.

use crate::bola;
use crate::embed;
use crate::proyeccion::{Corpus, Params, MAX_PROFUNDIDAD};
use std::collections::VecDeque;

const MARGEN: usize = 16;

pub struct IndiceBola {
    pub dim: usize,
    pub n: usize,
    /// `cols[d*n + i]` = coordenada d del documento i.
    cols: Vec<f32>,
    a: Vec<f32>,
}

impl IndiceBola {
    pub fn new(c: &Corpus) -> IndiceBola {
        let (n, dim) = (c.docs.len(), c.dim);
        let mut cols = vec![0f32; n * dim];
        let mut a = vec![0f32; n];
        for (i, d) in c.docs.iter().enumerate() {
            for k in 0..dim {
                cols[k * n + i] = d.x[k] as f32;
            }
            a[i] = (1.0 / (1.0 - bola::norma2(&d.x))) as f32;
        }
        IndiceBola { dim, n, cols, a }
    }

    /// Los `k` documentos más cercanos (distancia hiperbólica) a `q`, sin `excluir`.
    /// Orden total: (distancia, índice). Devuelve (doc, distancia).
    pub fn knn(&self, c: &Corpus, q: &[f64], k: usize, excluir: Option<u32>) -> Vec<(u32, f64)> {
        let n = self.n;
        let mut acc = vec![0f32; n];
        for (d, &qd) in q.iter().enumerate().take(self.dim) {
            let qd = qd as f32;
            let col = &self.cols[d * n..(d + 1) * n];
            for (s, &x) in acc.iter_mut().zip(col) {
                let t = x - qd;
                *s += t * t;
            }
        }
        let mut cand: Vec<(f32, u32)> = acc
            .iter()
            .zip(&self.a)
            .enumerate()
            .filter(|(i, _)| Some(*i as u32) != excluir)
            .map(|(i, (s, a))| (s * a, i as u32))
            .collect();
        let m = (k + MARGEN).min(cand.len());
        if m == 0 {
            return Vec::new();
        }
        if cand.len() > m {
            cand.select_nth_unstable_by(m - 1, |x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
            cand.truncate(m);
        }
        let mut exacto: Vec<(u32, f64)> =
            cand.into_iter().map(|(_, i)| (i, bola::distancia(q, &c.docs[i as usize].x))).collect();
        exacto.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));
        exacto.truncate(k);
        exacto
    }
}

pub struct IndiceLexico {
    pub n: usize,
    idf: Vec<f64>,
    /// `cols[d*n + i]` = componente d del TF‑IDF del documento i.
    cols: Vec<f32>,
    /// Para cada documento, sus `VECINOS_LEXICOS` vecinos por coseno TF‑IDF (sin él
    /// mismo), precalculados al montar si n ≤ `MAX_PRECALCULO`; si no, vacío y se calcula por consulta.
    vecinos: Vec<Vec<(u32, f32)>>,
}

pub const VECINOS_LEXICOS: usize = 50;
pub const MAX_PRECALCULO: usize = 5000;

impl IndiceLexico {
    pub fn new(c: &Corpus) -> IndiceLexico {
        let textos: Vec<String> = c.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
        let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
        let idf = embed::idf(&refs);
        let n = c.docs.len();
        let mut cols = vec![0f32; n * embed::DIM];
        for (i, t) in refs.iter().enumerate() {
            for (d, x) in embed::vectorizar_rapido(t, &idf).into_iter().enumerate() {
                cols[d * n + i] = x as f32;
            }
        }
        let vecinos = if n <= MAX_PRECALCULO { vecinos_lexicos(&cols, n) } else { Vec::new() };
        IndiceLexico { n, idf, cols, vecinos }
    }

    /// Vecinos léxicos del documento `j` (coseno desc., empate índice), sin `j`.
    pub fn vecinos_de(&self, j: u32) -> Vec<(u32, f32)> {
        if let Some(v) = self.vecinos.get(j as usize) {
            return v.clone();
        }
        let n = self.n;
        let mut s = vec![0f32; n];
        for d in 0..embed::DIM {
            let w = self.cols[d * n + j as usize];
            if w == 0.0 {
                continue;
            }
            for (a, &x) in s.iter_mut().zip(&self.cols[d * n..(d + 1) * n]) {
                *a += w * x;
            }
        }
        mejores(s, Some(j), VECINOS_LEXICOS)
    }

    pub fn precalculado(&self) -> bool {
        !self.vecinos.is_empty() || self.n == 0
    }

    /// Coseno de la consulta con cada documento (f32, misma aritmética que FAISS IP sobre f32).
    pub fn puntajes(&self, texto: &str) -> Vec<f32> {
        let q = embed::vectorizar_rapido(texto, &self.idf);
        let n = self.n;
        let mut s = vec![0f32; n];
        for (d, &w) in q.iter().enumerate() {
            if w == 0.0 {
                continue;
            }
            let w = w as f32;
            for (a, &x) in s.iter_mut().zip(&self.cols[d * n..(d + 1) * n]) {
                *a += w * x;
            }
        }
        s
    }

    /// Los `k` de mayor coseno (empate: índice). Devuelve (doc, coseno).
    pub fn top(&self, texto: &str, k: usize) -> Vec<(u32, f32)> {
        let s = self.puntajes(texto);
        let mut v: Vec<(u32, f32)> = s.into_iter().enumerate().map(|(i, x)| (i as u32, x)).collect();
        let cmp = |a: &(u32, f32), b: &(u32, f32)| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0));
        let k = k.min(v.len());
        if k == 0 {
            return Vec::new();
        }
        if v.len() > k {
            v.select_nth_unstable_by(k - 1, cmp);
            v.truncate(k);
        }
        v.sort_by(cmp);
        v
    }
}

fn mejores(s: Vec<f32>, excluir: Option<u32>, k: usize) -> Vec<(u32, f32)> {
    let mut v: Vec<(u32, f32)> = s.into_iter().enumerate().map(|(i, x)| (i as u32, x)).filter(|x| Some(x.0) != excluir).collect();
    let cmp = |a: &(u32, f32), b: &(u32, f32)| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0));
    let k = k.min(v.len());
    if k == 0 {
        return Vec::new();
    }
    if v.len() > k {
        v.select_nth_unstable_by(k - 1, cmp);
        v.truncate(k);
    }
    v.sort_by(cmp);
    v
}

/// Todos los pares por coseno (f32, filas contiguas), repartido en hilos.
fn vecinos_lexicos(cols: &[f32], n: usize) -> Vec<Vec<(u32, f32)>> {
    let dim = embed::DIM;
    let mut filas = vec![0f32; n * dim];
    for d in 0..dim {
        for i in 0..n {
            filas[i * dim + d] = cols[d * n + i];
        }
    }
    let hilos = std::thread::available_parallelism().map(|h| h.get()).unwrap_or(1).clamp(1, 8);
    let trozo = n.div_ceil(hilos).max(1);
    let filas = &filas;
    std::thread::scope(|sc| {
        let hs: Vec<_> = (0..n)
            .step_by(trozo)
            .map(|ini| {
                sc.spawn(move || {
                    (ini..(ini + trozo).min(n))
                        .map(|i| {
                            let a = &filas[i * dim..(i + 1) * dim];
                            let s: Vec<f32> = (0..n)
                                .map(|j| {
                                    let b = &filas[j * dim..(j + 1) * dim];
                                    let mut acc = [0f32; 8];
                                    for (ca, cb) in a.chunks_exact(8).zip(b.chunks_exact(8)) {
                                        for l in 0..8 {
                                            acc[l] += ca[l] * cb[l];
                                        }
                                    }
                                    acc.iter().sum()
                                })
                                .collect();
                            mejores(s, Some(i as u32), VECINOS_LEXICOS)
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        hs.into_iter().flat_map(|h| h.join().expect("hilo léxico")).collect()
    })
}

/// Punto medio de Einstein (vía el modelo de Klein) de puntos de la bola de Poincaré.
pub fn punto_medio(puntos: &[(&[f64], f64)], dim: usize) -> Vec<f64> {
    let mut num = vec![0.0; dim];
    let mut den = 0.0;
    for (x, w) in puntos {
        let x2 = bola::norma2(x);
        let kx: Vec<f64> = x.iter().map(|v| 2.0 * v / (1.0 + x2)).collect();
        let g = 1.0 / (1.0 - bola::norma2(&kx)).max(1e-15).sqrt();
        for (a, b) in num.iter_mut().zip(&kx) {
            *a += w * g * b;
        }
        den += w * g;
    }
    if den <= 0.0 {
        return vec![0.0; dim];
    }
    let k: Vec<f64> = num.iter().map(|v| v / den).collect();
    let k2 = bola::norma2(&k).min(1.0 - 1e-15);
    k.iter().map(|v| v / (1.0 + (1.0 - k2).sqrt())).collect()
}

/// Modo de búsqueda de un directorio de consulta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Modo {
    /// `@similar/<doc>`: vecinos hiperbólicos del documento (bola pura).
    Similar,
    /// `@similar-hibrido/<doc>`: ancla = vecino léxico más cercano; 2–10 = vecinos léxicos ordenados por la bola.
    SimilarHibrido,
    /// `@buscar/<consulta>`: híbrido léxico → bola.
    Hibrido,
    /// `@buscar-bola/<consulta>`: orden solo por la bola desde el punto medio de los 5 primeros léxicos.
    Bola,
}

pub const CANDIDATOS_HIBRIDO: usize = 50;
pub const ANCLAS_BOLA: usize = 5;

/// Resultados (doc, puntaje) de una búsqueda directa; el puntaje es una distancia
/// hiperbólica (menor = mejor) salvo el ancla del híbrido, que lleva su coseno negado.
pub fn buscar(c: &Corpus, bola_i: &IndiceBola, lex: &IndiceLexico, modo: Modo, clave: &str, k: usize) -> Option<Vec<(u32, f64)>> {
    match modo {
        Modo::Similar => {
            let j = c.indice(clave)?;
            Some(bola_i.knn(c, &c.docs[j as usize].x, k, Some(j)))
        }
        Modo::SimilarHibrido => {
            let j = c.indice(clave)?;
            let top = lex.vecinos_de(j);
            let Some(&(ancla, s0)) = top.first() else { return Some(Vec::new()) };
            let xs = &c.docs[j as usize].x;
            let mut resto: Vec<(u32, f64)> =
                top[1..].iter().map(|&(i, _)| (i, bola::distancia(xs, &c.docs[i as usize].x))).collect();
            resto.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let mut out = vec![(ancla, -(s0 as f64))];
            out.extend(resto.into_iter().take(k.saturating_sub(1)));
            out.truncate(k);
            Some(out)
        }
        Modo::Hibrido => {
            let top = lex.top(clave, CANDIDATOS_HIBRIDO);
            let Some(&(ancla, s0)) = top.first() else { return Some(Vec::new()) };
            if s0 <= 0.0 {
                return Some(Vec::new());
            }
            let xa = &c.docs[ancla as usize].x;
            let mut resto: Vec<(u32, f64)> =
                top[1..].iter().filter(|x| x.1 > 0.0).map(|&(i, _)| (i, bola::distancia(xa, &c.docs[i as usize].x))).collect();
            resto.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let mut out = vec![(ancla, -(s0 as f64))];
            out.extend(resto.into_iter().take(k.saturating_sub(1)));
            out.truncate(k);
            Some(out)
        }
        Modo::Bola => {
            let top = lex.top(clave, ANCLAS_BOLA);
            let pts: Vec<(&[f64], f64)> =
                top.iter().filter(|x| x.1 > 0.0).map(|&(i, s)| (c.docs[i as usize].x.as_slice(), s as f64)).collect();
            if pts.is_empty() {
                return Some(Vec::new());
            }
            let q = punto_medio(&pts, c.dim);
            Some(bola_i.knn(c, &q, k, None))
        }
    }
}

/// Árbol de alcance.
pub struct Cobertura {
    /// `padre[j]`: None = la raíz.
    pub padre: Vec<Option<u32>>,
    pub profundidad: Vec<usize>,
    /// Hijos por foco (índice n = la raíz), ordenados por (distancia al padre, índice).
    hijos: Vec<Vec<u32>>,
    /// Cuántos documentos se colgaron por cobertura (no eran alcanzables por el grafo de k vecinos con γ=0).
    pub agregados: usize,
}

impl Cobertura {
    /// BFS desde `/` por el grafo de k vecinos con γ = 0 (la vista de `/` no depende de γ);
    /// los que quedan sin alcanzar se cuelgan, uno por vez, del documento alcanzado más
    /// cercano (estilo Prim) cuya profundidad deje hacer `cd` al hijo (≤ MAX_PROFUNDIDAD − 1).
    pub fn new(c: &Corpus, p: Params, ib: &IndiceBola) -> Cobertura {
        let n = c.docs.len();
        let mut padre: Vec<Option<u32>> = vec![None; n];
        let mut prof = vec![usize::MAX; n];
        let mut cola = VecDeque::new();
        for e in crate::proyeccion::vista(c, &[], Params { memoria: 0.0, ..p }).entradas {
            prof[e.doc as usize] = 1;
            cola.push_back(e.doc);
        }
        while let Some(j) = cola.pop_front() {
            let dj = prof[j as usize];
            if dj >= MAX_PROFUNDIDAD - 1 {
                continue;
            }
            let xj = &c.docs[j as usize].x;
            for (i, d) in ib.knn(c, xj, p.k, Some(j)) {
                if d <= p.radio && prof[i as usize] == usize::MAX {
                    prof[i as usize] = dj + 1;
                    padre[i as usize] = Some(j);
                    cola.push_back(i);
                }
            }
        }
        // Prim: mejor padre alcanzado para cada no alcanzado
        let elegible = |d: usize| d < MAX_PROFUNDIDAD - 1;
        let mut mejor: Vec<(f64, Option<u32>)> = vec![(f64::INFINITY, None); n];
        let pendientes: Vec<usize> = (0..n).filter(|&i| prof[i] == usize::MAX).collect();
        let agregados = pendientes.len();
        if !pendientes.is_empty() {
            for &u in &pendientes {
                for (j, &pj) in prof.iter().enumerate() {
                    if pj != usize::MAX && elegible(pj) {
                        let d = bola::distancia(&c.docs[u].x, &c.docs[j].x);
                        if d < mejor[u].0 {
                            mejor[u] = (d, Some(j as u32));
                        }
                    }
                }
            }
            let mut quedan = pendientes;
            while !quedan.is_empty() {
                let (pos, &u) = quedan
                    .iter()
                    .enumerate()
                    .min_by(|a, b| mejor[*a.1].0.total_cmp(&mejor[*b.1].0).then(a.1.cmp(b.1)))
                    .unwrap();
                quedan.swap_remove(pos);
                match mejor[u].1 {
                    Some(j) => {
                        padre[u] = Some(j);
                        prof[u] = prof[j as usize] + 1;
                    }
                    None => {
                        padre[u] = None; // nada alcanzado: cuelga de la raíz
                        prof[u] = 1;
                    }
                }
                if elegible(prof[u]) {
                    for &v in &quedan {
                        let d = bola::distancia(&c.docs[u].x, &c.docs[v].x);
                        if d < mejor[v].0 {
                            mejor[v] = (d, Some(u as u32));
                        }
                    }
                }
            }
        }
        let mut hijos: Vec<Vec<u32>> = vec![Vec::new(); n + 1];
        for (i, p) in padre.iter().enumerate() {
            hijos[p.map(|x| x as usize).unwrap_or(n)].push(i as u32);
        }
        for (f, h) in hijos.iter_mut().enumerate() {
            let xf: Vec<f64> = if f == n { vec![0.0; c.dim] } else { c.docs[f].x.clone() };
            h.sort_by(|a, b| {
                bola::distancia(&xf, &c.docs[*a as usize].x)
                    .total_cmp(&bola::distancia(&xf, &c.docs[*b as usize].x))
                    .then(a.cmp(b))
            });
        }
        Cobertura { padre, profundidad: prof, hijos, agregados }
    }

    /// Hijos del foco (`None` = raíz).
    pub fn hijos(&self, foco: Option<u32>) -> &[u32] {
        let n = self.padre.len();
        &self.hijos[foco.map(|x| x as usize).unwrap_or(n)]
    }

    /// Ruta de `cd` desde `/` hasta el padre de `j` (por el árbol).
    pub fn ruta_hasta(&self, j: u32) -> Vec<u32> {
        let mut r = Vec::new();
        let mut cur = self.padre[j as usize];
        while let Some(p) = cur {
            r.push(p);
            cur = self.padre[p as usize];
        }
        r.reverse();
        r
    }

    /// Prueba de alcance por el árbol: cuántos documentos llegan a la raíz por la
    /// cadena de padres sin ciclos y con profundidad ≤ MAX_PROFUNDIDAD.
    pub fn alcanzables(&self) -> usize {
        (0..self.padre.len())
            .filter(|&j| {
                let mut cur = self.padre[j];
                let mut pasos = 1;
                while let Some(p) = cur {
                    pasos += 1;
                    if pasos > MAX_PROFUNDIDAD {
                        return false;
                    }
                    cur = self.padre[p as usize];
                }
                true
            })
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ajuste::Rng;
    use crate::proyeccion::vista;

    fn corpus() -> Corpus {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
        Corpus::desde_directorio(&dir).unwrap()
    }

    #[test]
    fn knn_igual_a_fuerza_bruta() {
        let c = corpus();
        let ib = IndiceBola::new(&c);
        let mut r = Rng::new(3);
        for _ in 0..50 {
            let q: Vec<f64> = (0..c.dim).map(|_| (r.uniforme() - 0.5) * 0.4).collect();
            let mut bruto: Vec<(u32, f64)> =
                (0..c.docs.len() as u32).map(|i| (i, bola::distancia(&q, &c.docs[i as usize].x))).collect();
            bruto.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            for k in [1, 5, 10, 18] {
                assert_eq!(ib.knn(&c, &q, k, None), bruto[..k.min(bruto.len())].to_vec());
            }
        }
    }

    #[test]
    fn similar_coincide_con_vista_gamma_cero() {
        let c = corpus();
        let ib = IndiceBola::new(&c);
        let p0 = Params { memoria: 0.0, ..Params::default() };
        for j in 0..c.docs.len() as u32 {
            let v: Vec<u32> = vista(&c, &[j], p0).entradas.iter().map(|e| e.doc).collect();
            let s: Vec<u32> = ib.knn(&c, &c.docs[j as usize].x, 10, Some(j)).iter().map(|x| x.0).collect();
            assert_eq!(v, s);
        }
    }

    #[test]
    fn lexico_coincide_con_coseno() {
        let c = corpus();
        let lx = IndiceLexico::new(&c);
        let textos: Vec<String> = c.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
        let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
        let idf = embed::idf(&refs);
        let q = "kernel linux ebpf scheduler";
        let qv = embed::vectorizar(q, &idf);
        let s = lx.puntajes(q);
        for (i, t) in refs.iter().enumerate() {
            let exacto = embed::coseno(&embed::vectorizar(t, &idf), &qv);
            assert!((exacto - s[i] as f64).abs() < 1e-5);
        }
    }

    #[test]
    fn vecinos_lexicos_precalculados_iguales_a_calcular() {
        let c = corpus();
        let lx = IndiceLexico::new(&c);
        assert!(lx.precalculado());
        let sin = IndiceLexico { vecinos: Vec::new(), ..IndiceLexico::new(&c) };
        for j in 0..c.docs.len() as u32 {
            let a = lx.vecinos_de(j);
            let b = sin.vecinos_de(j);
            assert_eq!(a.iter().map(|x| x.0).collect::<Vec<_>>()[..1], b.iter().map(|x| x.0).collect::<Vec<_>>()[..1]);
            for (x, y) in a.iter().zip(&b) {
                assert!((x.1 - y.1).abs() < 1e-5);
            }
            assert!(a.iter().all(|x| x.0 != j));
        }
        let r = buscar(&c, &IndiceBola::new(&c), &lx, Modo::SimilarHibrido, &c.docs[0].nombre, 10).unwrap();
        assert_eq!(r[0].0, lx.vecinos_de(0)[0].0);
    }

    #[test]
    fn punto_medio_de_un_punto_es_el_punto() {
        let x = vec![0.3, -0.2, 0.1];
        let m = punto_medio(&[(&x, 2.0)], 3);
        assert!(bola::distancia(&m, &x) < 1e-9);
        let o = punto_medio(&[(&x, 1.0), (&[-0.3, 0.2, -0.1], 1.0)], 3);
        assert!(bola::norma(&o) < 1e-12);
    }

    #[test]
    fn hibrido_ancla_lexica_y_resto_por_bola() {
        let c = corpus();
        let (ib, lx) = (IndiceBola::new(&c), IndiceLexico::new(&c));
        let q = "ebpf kernel";
        let r = buscar(&c, &ib, &lx, Modo::Hibrido, q, 10).unwrap();
        assert_eq!(r[0].0, lx.top(q, 1)[0].0);
        for w in r[1..].windows(2) {
            assert!(w[0].1 <= w[1].1);
        }
        assert!(buscar(&c, &ib, &lx, Modo::Hibrido, "!!", 10).unwrap().is_empty());
        assert!(buscar(&c, &ib, &lx, Modo::Similar, "no-existe", 10).is_none());
    }

    #[test]
    fn cobertura_alcanza_todo_aun_con_k_chico() {
        let c = corpus();
        let ib = IndiceBola::new(&c);
        for k in [1, 2, 3, 10] {
            let p = Params { k, ..Params::default() };
            let cob = Cobertura::new(&c, p, &ib);
            assert_eq!(cob.alcanzables(), c.docs.len(), "k={k}");
            for j in 0..c.docs.len() as u32 {
                assert!(cob.profundidad[j as usize] <= MAX_PROFUNDIDAD);
                assert!(cob.hijos(cob.padre[j as usize]).contains(&j));
                assert_eq!(cob.ruta_hasta(j).len() + 1, cob.profundidad[j as usize]);
            }
        }
    }
}
