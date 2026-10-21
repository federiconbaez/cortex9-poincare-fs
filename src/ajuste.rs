//! Embedding hiperbólico: ajuste por SGD Riemanniano en la bola de Poincaré
//! (estilo Nickel & Kiela 2017) contra el grafo de vecinos por coseno TF‑IDF.
//!
//! 1. Para cada documento, sus `vecinos` más parecidos por coseno TF‑IDF
//!    (cálculo exacto O(n²), repartido en hilos; resultado determinista).
//! 2. Pérdida por par positivo (i, j) con `negativos` muestras k:
//!    `L = d(i,j) + log Σ_{k ∈ {j} ∪ neg} exp(−d(i,k))`.
//! 3. Gradiente euclídeo de `d = arcosh(1 + 2|u−v|² / ((1−|u|²)(1−|v|²)))`,
//!    reescalado por la métrica `((1−|u|²)²/4)` y retracción dentro de la bola.
//! 4. Semilla fija (splitmix64): mismo corpus y parámetros ⇒ mismos puntos.

use crate::bola::{self, V};

/// Radio máximo de los puntos ajustados (mantiene la aritmética bien condicionada).
pub const RADIO_AJUSTE: f64 = 0.95;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamsAjuste {
    pub dim: usize,
    pub vecinos: usize,
    pub negativos: usize,
    /// 0 = automático según el tamaño del corpus.
    pub epocas: usize,
    pub lr: f64,
    pub semilla: u64,
}

impl Default for ParamsAjuste {
    fn default() -> Self {
        ParamsAjuste { dim: 16, vecinos: 5, negativos: 10, epocas: 0, lr: 0.3, semilla: 0x5EED_F00D }
    }
}

impl ParamsAjuste {
    pub fn epocas_efectivas(&self, n: usize) -> usize {
        if self.epocas > 0 {
            return self.epocas;
        }
        let pares = (n * self.vecinos.min(n.saturating_sub(1))).max(1);
        (300_000 / pares).clamp(30, 400)
    }
}

/// Generador splitmix64 (determinista, sin dependencias).
pub struct Rng(u64);
impl Rng {
    pub fn new(s: u64) -> Rng {
        Rng(s)
    }
    pub fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    pub fn uniforme(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
    pub fn indice(&mut self, n: usize) -> usize {
        (self.next_u64() % n as u64) as usize
    }
}

/// Producto interno f32 con 8 acumuladores (permite vectorizar).
fn dot32(a: &[f32], b: &[f32]) -> f32 {
    let mut acc = [0f32; 8];
    let ca = a.chunks_exact(8);
    let cb = b.chunks_exact(8);
    let (ra, rb) = (ca.remainder(), cb.remainder());
    for (x, y) in ca.zip(cb) {
        for k in 0..8 {
            acc[k] += x[k] * y[k];
        }
    }
    let mut s: f32 = acc.iter().sum();
    for (x, y) in ra.iter().zip(rb) {
        s += x * y;
    }
    s
}

/// Vecinos más parecidos por coseno (vectores ya normalizados L2).
/// Orden: coseno descendente, a igualdad índice ascendente.
pub fn vecinos_tfidf(x: &[Vec<f64>], m: usize) -> Vec<Vec<u32>> {
    let n = x.len();
    let m = m.min(n.saturating_sub(1));
    let x32: Vec<Vec<f32>> = x.iter().map(|v| v.iter().map(|&a| a as f32).collect()).collect();
    let hilos = std::thread::available_parallelism().map(|h| h.get()).unwrap_or(1).min(16);
    let mut out: Vec<Vec<u32>> = vec![Vec::new(); n];
    let trozo = n.div_ceil(hilos).max(1);
    std::thread::scope(|s| {
        for (bloque, salida) in out.chunks_mut(trozo).enumerate() {
            let x32 = &x32;
            s.spawn(move || {
                for (off, res) in salida.iter_mut().enumerate() {
                    let i = bloque * trozo + off;
                    let mut mejores: Vec<(f32, u32)> = Vec::with_capacity(m + 1);
                    for j in 0..n {
                        if j == i {
                            continue;
                        }
                        let c = dot32(&x32[i], &x32[j]);
                        if mejores.len() < m || c > mejores[m - 1].0 {
                            let pos = mejores.partition_point(|&(cc, jj)| cc > c || (cc == c && jj < j as u32));
                            mejores.insert(pos, (c, j as u32));
                            mejores.truncate(m);
                        }
                    }
                    *res = mejores.into_iter().map(|(_, j)| j).collect();
                }
            });
        }
    });
    out
}

/// Distancia (forma arcosh) y gradiente euclídeo respecto de `u`.
fn dist_grad(u: &[f64], v: &[f64]) -> (f64, V) {
    let uu = bola::norma2(u);
    let vv = bola::norma2(v);
    let uv = bola::dot(u, v);
    let sq = (uu - 2.0 * uv + vv).max(0.0);
    let alfa = 1.0 - uu;
    let beta = 1.0 - vv;
    let gamma = 1.0 + 2.0 * sq / (alfa * beta);
    let d = gamma.acosh();
    let raiz = (gamma * gamma - 1.0).max(1e-15).sqrt();
    let c = 4.0 / (beta * raiz);
    let a = (vv - 2.0 * uv + 1.0) / (alfa * alfa);
    let g = u.iter().zip(v).map(|(ui, vi)| c * (a * ui - vi / alfa)).collect();
    (d, g)
}

fn paso(u: &mut [f64], g: &[f64], coef: f64, lr: f64) {
    let metrica = (1.0 - bola::norma2(u)).powi(2) / 4.0;
    for (ui, gi) in u.iter_mut().zip(g) {
        *ui -= lr * coef * metrica * gi;
    }
    bola::proyectar(u, RADIO_AJUSTE);
}

/// Ajusta los puntos de la bola a partir del grafo de vecinos.
pub fn ajustar(vecinos: &[Vec<u32>], p: ParamsAjuste) -> Vec<V> {
    let n = vecinos.len();
    let mut rng = Rng::new(p.semilla);
    let mut x: Vec<V> = (0..n)
        .map(|_| (0..p.dim).map(|_| (rng.uniforme() - 0.5) * 2e-3).collect())
        .collect();
    if n < 2 {
        return x;
    }
    let mut pares: Vec<(u32, u32)> = vecinos
        .iter()
        .enumerate()
        .flat_map(|(i, vs)| vs.iter().map(move |&j| (i as u32, j)))
        .collect();
    let epocas = p.epocas_efectivas(n);
    let quemado = 10.min(epocas / 4);
    let mut negs: Vec<usize> = Vec::with_capacity(p.negativos);
    for ep in 0..epocas {
        let lr = if ep < quemado { p.lr / 10.0 } else { p.lr };
        // Fisher–Yates determinista
        for k in (1..pares.len()).rev() {
            let r = rng.indice(k + 1);
            pares.swap(k, r);
        }
        for &(i, j) in &pares {
            let (i, j) = (i as usize, j as usize);
            negs.clear();
            let mut intentos = 0;
            while negs.len() < p.negativos && intentos < 4 * p.negativos {
                intentos += 1;
                let k = rng.indice(n);
                if k != i && !vecinos[i].contains(&(k as u32)) {
                    negs.push(k);
                }
            }
            let (dij, gij) = dist_grad(&x[i], &x[j]);
            let (_, gji) = dist_grad(&x[j], &x[i]);
            let mut ds = vec![dij];
            let mut gs_i = vec![gij];
            let mut gs_k = vec![gji];
            for &k in &negs {
                let (dik, gik) = dist_grad(&x[i], &x[k]);
                let (_, gki) = dist_grad(&x[k], &x[i]);
                ds.push(dik);
                gs_i.push(gik);
                gs_k.push(gki);
            }
            // softmax sobre −d
            let mn = ds.iter().cloned().fold(f64::INFINITY, f64::min);
            let ex: Vec<f64> = ds.iter().map(|d| (-(d - mn)).exp()).collect();
            let z: f64 = ex.iter().sum();
            let coefs: Vec<f64> = ex
                .iter()
                .enumerate()
                .map(|(t, e)| if t == 0 { 1.0 - e / z } else { -e / z })
                .collect();
            // gradiente para i (acumulado) y para cada objetivo
            let mut gi = vec![0.0; p.dim];
            for (g, c) in gs_i.iter().zip(&coefs) {
                for (a, b) in gi.iter_mut().zip(g) {
                    *a += c * b;
                }
            }
            paso(&mut x[i], &gi, 1.0, lr);
            paso(&mut x[j], &gs_k[0], coefs[0], lr);
            for (t, &k) in negs.iter().enumerate() {
                paso(&mut x[k], &gs_k[t + 1], coefs[t + 1], lr);
            }
        }
    }
    x
}

/// Base 2D para visualizar: las dos componentes principales de `log_0(x_i)`.
pub fn base_2d(x: &[V], dim: usize) -> [V; 2] {
    let logs: Vec<V> = x.iter().map(|p| bola::log0(p)).collect();
    let comp = |previas: &[V]| -> V {
        let mut v: V = (0..dim).map(|i| 1.0 + (i % 5) as f64 * 0.1).collect();
        for _ in 0..500 {
            let mut w = vec![0.0; dim];
            for f in &logs {
                let s = bola::dot(f, &v);
                w.iter_mut().zip(f).for_each(|(a, b)| *a += s * b);
            }
            for p in previas {
                let k = bola::dot(&w, p);
                w.iter_mut().zip(p).for_each(|(a, b)| *a -= k * b);
            }
            let nw = bola::norma(&w);
            if nw == 0.0 {
                break;
            }
            v = bola::escalar(&w, 1.0 / nw);
        }
        for p in previas {
            let k = bola::dot(&v, p);
            v.iter_mut().zip(p).for_each(|(a, b)| *a -= k * b);
        }
        let nv = bola::norma(&v);
        if nv > 0.0 {
            v = bola::escalar(&v, 1.0 / nv);
        }
        v
    };
    let e1 = comp(&[]);
    let e2 = comp(std::slice::from_ref(&e1));
    [e1, e2]
}

/// Proyección al disco 2D: `exp_0` de la proyección de `log_0(w)` sobre la base.
/// Nunca aleja: la distancia al origen en 2D es ≤ la distancia real.
pub fn a_disco(w: &[f64], base: &[V; 2]) -> (f64, f64) {
    let l = bola::log0(w);
    let u = [bola::dot(&l, &base[0]), bola::dot(&l, &base[1])];
    let e = bola::exp0(&u);
    (e[0], e[1])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gradiente_coincide_con_diferencias_finitas() {
        let u = vec![0.1, -0.3, 0.2];
        let v = vec![-0.4, 0.25, 0.05];
        let (d, g) = dist_grad(&u, &v);
        assert!((d - bola::distancia(&u, &v)).abs() < 1e-12);
        for k in 0..3 {
            let h = 1e-6;
            let mut up = u.clone();
            up[k] += h;
            let mut um = u.clone();
            um[k] -= h;
            let fd = (bola::distancia(&up, &v) - bola::distancia(&um, &v)) / (2.0 * h);
            assert!((fd - g[k]).abs() < 1e-6, "k={k} fd={fd} g={}", g[k]);
        }
    }

    #[test]
    fn vecinos_exactos() {
        let x = vec![vec![1.0, 0.0], vec![0.8, 0.6], vec![0.0, 1.0], vec![-1.0, 0.0]];
        let v = vecinos_tfidf(&x, 2);
        assert_eq!(v[0], vec![1, 2]);
        assert_eq!(v[2], vec![1, 0]);
        assert_eq!(v[3], vec![2, 1]);
    }

    #[test]
    fn ajuste_determinista_y_dentro_de_la_bola() {
        let vecinos = vec![vec![1], vec![0], vec![3], vec![2]];
        let p = ParamsAjuste { dim: 4, vecinos: 1, negativos: 2, epocas: 50, ..Default::default() };
        let a = ajustar(&vecinos, p);
        let b = ajustar(&vecinos, p);
        assert_eq!(a, b);
        for x in &a {
            assert!(bola::norma(x) < 1.0);
        }
        // los pares vecinos quedan más cerca que los no vecinos
        assert!(bola::distancia(&a[0], &a[1]) < bola::distancia(&a[0], &a[2]));
        assert!(bola::distancia(&a[2], &a[3]) < bola::distancia(&a[1], &a[3]));
    }

    #[test]
    fn disco_no_aleja() {
        let base = [vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]];
        let w = vec![0.3, 0.4, 0.5];
        let (a, b) = a_disco(&w, &base);
        assert!(2.0 * (a.hypot(b)).atanh() <= bola::distancia_al_origen(&w) + 1e-12);
    }
}
