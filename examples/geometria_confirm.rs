//! Confirmación publicada (secundaria): entrenar sobre aristas del árbol
//! (nodos internos incluidos) y medir distorsión. No decide el valor.
use poincare_fs::ajuste::{self, ParamsAjuste, Rng};
use poincare_fs::bola;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::Instant;

const DIMS: [usize; 4] = [2, 4, 8, 16];
const LRS: [f64; 5] = [0.03, 0.1, 0.3, 1.0, 3.0];
const RADIOS: [f64; 2] = [0.95, 0.999];
const SEMILLAS: [u64; 3] = [0x5EED_F00D, 0xA11CE, 0xB0B];
const DIST_SEED: u64 = 20261027;
const DIST_PARES: usize = 20_000;

fn dist_arbol(a: &[usize], b: &[usize]) -> f64 {
    let mut lcp = 0;
    while lcp < a.len() && lcp < b.len() && a[lcp] == b[lcp] {
        lcp += 1;
    }
    (a.len() + b.len() - 2 * lcp) as f64
}

fn distorsion(xs: &[Vec<f64>], caminos: &[Vec<usize>], hojas: &[usize], geom: &str) -> f64 {
    let mut rng = Rng::new(DIST_SEED);
    let m = hojas.len();
    let mut dt = Vec::new();
    let mut de = Vec::new();
    for _ in 0..DIST_PARES {
        let i = hojas[rng.indice(m)];
        let j = hojas[rng.indice(m)];
        if i == j {
            continue;
        }
        let t = dist_arbol(&caminos[i], &caminos[j]);
        if t <= 0.0 {
            continue;
        }
        let e = if geom == "H" {
            bola::distancia(&xs[i], &xs[j])
        } else {
            xs[i].iter().zip(&xs[j]).map(|(a, b)| (a - b) * (a - b)).sum::<f64>().sqrt()
        };
        if e > 0.0 && e.is_finite() {
            dt.push(t);
            de.push(e);
        }
    }
    let mut ratios: Vec<f64> = dt.iter().zip(&de).map(|(t, e)| t / e).collect();
    ratios.sort_by(|a, b| a.total_cmp(b));
    let s = ratios[ratios.len() / 2];
    dt.iter().zip(&de).map(|(t, e)| (s * e - t).abs() / t).sum::<f64>() / dt.len() as f64
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mani = Path::new(&args[1]);
    let out = Path::new(&args[2]);
    fs::create_dir_all(out).unwrap();
    // nodos = todos los prefijos de camino; aristas padre→hijo
    let mut id: HashMap<String, usize> = HashMap::new();
    let mut caminos_nodo: Vec<Vec<usize>> = Vec::new();
    let mut hojas = Vec::new();
    let mut edges: Vec<(u32, u32)> = Vec::new();
    for l in fs::read_to_string(mani).unwrap().lines().skip(1) {
        let cam = l.split('\t').nth(2).unwrap();
        let partes: Vec<&str> = cam.split('/').collect();
        let mut path_ids = Vec::new();
        let mut acc = String::new();
        let mut parent = None;
        for (i, p) in partes.iter().enumerate() {
            if i > 0 {
                acc.push('/');
            }
            acc.push_str(p);
            let nid = *id.entry(acc.clone()).or_insert_with(|| {
                let n = caminos_nodo.len();
                caminos_nodo.push(Vec::new());
                n
            });
            path_ids.push(nid);
            if let Some(par) = parent {
                edges.push((par as u32, nid as u32));
                edges.push((nid as u32, par as u32));
            }
            parent = Some(nid);
        }
        let leaf = *path_ids.last().unwrap();
        caminos_nodo[leaf] = path_ids.clone();
        // también guardar camino en nodos internos si vacío
        for (depth, &nid) in path_ids.iter().enumerate() {
            if caminos_nodo[nid].is_empty() {
                caminos_nodo[nid] = path_ids[..=depth].to_vec();
            }
        }
        hojas.push(leaf);
    }
    edges.sort_unstable();
    edges.dedup();
    let n = id.len();
    let mut vecinos: Vec<Vec<u32>> = vec![Vec::new(); n];
    for (a, b) in edges {
        if !vecinos[a as usize].contains(&b) {
            vecinos[a as usize].push(b);
        }
    }
    for v in &mut vecinos {
        v.sort_unstable();
        v.truncate(5);
    }
    eprintln!("confirmación {}: {} nodos, {} hojas, {:.1} vecinos/nodo",
        mani.file_name().unwrap().to_string_lossy(), n, hojas.len(),
        vecinos.iter().map(|v| v.len()).sum::<usize>() as f64 / n as f64);
    let t0 = Instant::now();
    let mut lineas = String::from("geom\tdim\tlr\tradio\tsemilla\tdistorsión\n");
    let mut elegidas = Vec::new();
    for geom in ["H", "E"] {
        for &dim in &DIMS {
            let mut mejor: Option<(f64, f64, f64, Vec<f64>)> = None; // -meanD, -lr, -r, Ds
            for &lr in &LRS {
                for &r in &RADIOS {
                    let mut ds = Vec::new();
                    for &sem in &SEMILLAS {
                        let radio = if geom == "H" { r } else { 2.0 * r.atanh() };
                        let p = ParamsAjuste { dim, vecinos: 5, negativos: 10, epocas: 50, lr, semilla: sem, radio };
                        let xs = if geom == "H" { ajuste::ajustar(&vecinos, p) } else { ajuste::ajustar_euclideo(&vecinos, p) };
                        let d = distorsion(&xs, &caminos_nodo, &hojas, geom);
                        lineas.push_str(&format!("{geom}\t{dim}\t{lr}\t{r}\t{sem:x}\t{d:.6}\n"));
                        ds.push(d);
                    }
                    let mean = ds.iter().sum::<f64>() / 3.0;
                    let cand = (-mean, -lr, -r, ds);
                    if mejor.as_ref().map(|b| (b.0, b.1, b.2) < (cand.0, cand.1, cand.2)).unwrap_or(true) {
                        mejor = Some(cand);
                    }
                }
            }
            let (neg_mean, neg_lr, neg_r, ds) = mejor.unwrap();
            let mut ds = ds;
            ds.sort_by(|a, b| a.total_cmp(b));
            elegidas.push(format!(
                "| {geom} | {dim} | {} | {} | {:.4},{:.4},{:.4} | {:.4} |\n",
                -neg_lr, -neg_r, ds[0], ds[1], ds[2], ds[1]
            ));
            eprintln!("  {geom} d={dim}: D mediana={:.4} (mean val={:.4})", ds[1], -neg_mean);
        }
    }
    fs::write(out.join("confirmacion-crudo.tsv"), lineas).unwrap();
    let mut res = String::from("# Confirmación publicada (grafo = aristas del árbol)\n\n| geom | dim | lr | r | D por semilla | mediana D |\n|---|---:|---:|---:|---|---:|\n");
    for e in elegidas {
        res.push_str(&e);
    }
    fs::write(out.join("confirmacion.md"), res).unwrap();
    eprintln!("listo en {:.1}s", t0.elapsed().as_secs_f64());
}
