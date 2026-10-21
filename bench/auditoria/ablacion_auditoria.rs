//! Ablación de auditoría (Research): ¿la geometría hiperbólica aporta frente a euclídea 16d
//! sobre las MISMAS coordenadas, y de dónde sale la ventaja de tiempo?
//! Uso: cargo run --release --example ablacion_auditoria -- /tmp/pfs-valor/c1k bench/heldout
use poincare_fs::ajuste::Rng;
use poincare_fs::bola;
use poincare_fs::directo::{IndiceBola, IndiceLexico};
use poincare_fs::proyeccion::Corpus;
use std::collections::HashMap;
use std::hint::black_box;
use std::path::Path;
use std::time::Instant;

fn med(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.total_cmp(b));
    1e3 * v[v.len() / 2]
}

fn eucl2(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let original = Path::new(&args[1]);
    let heldout = Path::new(&args[2]);
    let mut nombres: Vec<String> = std::fs::read_dir(original)
        .unwrap()
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".txt").map(String::from))
        .collect();
    nombres.sort();
    let mut rng = Rng::new(2026);
    for i in (1..nombres.len()).rev() {
        let j = rng.indice(i + 1);
        nombres.swap(i, j);
    }
    let anon = std::env::temp_dir().join(format!("pfs-ablacion-{}", std::process::id()));
    std::fs::create_dir_all(&anon).unwrap();
    let mut tema_de: HashMap<String, String> = HashMap::new();
    for (i, n) in nombres.iter().enumerate() {
        let a = format!("d{i:05}");
        std::fs::copy(original.join(format!("{n}.txt")), anon.join(format!("{a}.txt"))).unwrap();
        tema_de.insert(a, n.split('-').next().unwrap().to_string());
    }
    let c = Corpus::desde_directorio(&anon).unwrap();
    let n = c.docs.len();
    let temas: Vec<String> = c.docs.iter().map(|d| tema_de[&d.nombre].clone()).collect();
    let mut tam: HashMap<String, usize> = HashMap::new();
    for t in &temas {
        *tam.entry(t.clone()).or_default() += 1;
    }
    let idx: HashMap<String, usize> = c.docs.iter().enumerate().map(|(i, d)| (d.nombre.clone(), i)).collect();
    let ib = IndiceBola::new(&c);
    let lx = IndiceLexico::new(&c);
    // SoA f32 16d (euclídea) y 1024d densa (TF-IDF) para tiempos
    let dim = c.dim;
    let mut cols16 = vec![0f32; dim * n];
    for (i, d) in c.docs.iter().enumerate() {
        for k in 0..dim {
            cols16[k * n + i] = d.x[k] as f32;
        }
    }
    let textos: Vec<String> = c.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
    let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
    let idf = poincare_fs::embed::idf(&refs);
    let m: Vec<Vec<f32>> = refs.iter().map(|t| poincare_fs::embed::vectorizar(t, &idf).into_iter().map(|x| x as f32).collect()).collect();
    let dd = m[0].len();
    println!("corpus {n} docs, dim bola {dim}, dim tfidf {dd}");
    let scan16 = |q: &[f64], excl: usize, k: usize| -> Vec<u32> {
        let mut acc = vec![0f32; n];
        for (d, &qd) in q.iter().enumerate() {
            let qd = qd as f32;
            for (s, &x) in acc.iter_mut().zip(&cols16[d * n..(d + 1) * n]) {
                let t = x - qd;
                *s += t * t;
            }
        }
        let mut v: Vec<(f32, u32)> = acc.iter().enumerate().filter(|(i, _)| *i != excl).map(|(i, s)| (*s, i as u32)).collect();
        v.select_nth_unstable_by(k - 1, |a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.truncate(k);
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.into_iter().map(|x| x.1).collect()
    };
    let scan1024 = |q: &[f32], excl: usize, k: usize| -> Vec<u32> {
        let mut v: Vec<(f32, u32)> = m
            .iter()
            .enumerate()
            .filter(|(i, _)| *i != excl)
            .map(|(i, x)| (-x.iter().zip(q).map(|(a, b)| a * b).sum::<f32>(), i as u32))
            .collect();
        v.select_nth_unstable_by(k - 1, |a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.truncate(k);
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.into_iter().map(|x| x.1).collect()
    };
    let puntuar = |l: &[u32], t: &str, excluir: bool| -> (u32, f64) {
        let ok = (temas[l[0] as usize] == t) as u32;
        let tope = (tam[t] - excluir as usize).clamp(1, 10) as f64;
        (ok, l.iter().take(10).filter(|&&i| temas[i as usize] == t).count() as f64 / tope)
    };
    println!("| conjunto | variante | n | recall@1 | P@10 |");
    println!("|---|---|---:|---:|---:|");
    for (conj, f) in [("T1'", "t1p-semillas.tsv"), ("T1''", "t1pp-semillas.tsv")] {
        let sem: Vec<usize> = std::fs::read_to_string(heldout.join(f))
            .unwrap()
            .lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| idx[l.split('\t').next().unwrap()])
            .collect();
        let mut acc: HashMap<&str, (u32, f64)> = HashMap::new();
        let mut add = |k: &'static str, r: (u32, f64)| {
            let e = acc.entry(k).or_default();
            e.0 += r.0;
            e.1 += r.1;
        };
        for &s in &sem {
            let t = temas[s].as_str();
            let xs = &c.docs[s].x;
            let hyp: Vec<u32> = ib.knn(&c, xs, 10, Some(s as u32)).iter().map(|x| x.0).collect();
            add("bola pura hiperbólica (=@similar)", puntuar(&hyp, t, true));
            add("bola pura EUCLÍDEA 16d (mismas coords)", puntuar(&scan16(xs, s, 10), t, true));
            let top = lx.vecinos_de(s as u32);
            let ancla = top[0].0;
            let mut h: Vec<(u32, f64)> = top[1..].iter().map(|&(i, _)| (i, bola::distancia(xs, &c.docs[i as usize].x))).collect();
            h.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let mut e: Vec<(u32, f64)> = top[1..].iter().map(|&(i, _)| (i, eucl2(xs, &c.docs[i as usize].x))).collect();
            e.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
            let lh: Vec<u32> = std::iter::once(ancla).chain(h.iter().map(|x| x.0)).take(10).collect();
            let le: Vec<u32> = std::iter::once(ancla).chain(e.iter().map(|x| x.0)).take(10).collect();
            let lc: Vec<u32> = top.iter().map(|x| x.0).take(10).collect();
            add("híbrido, 2–10 hiperbólico (=@similar-hibrido)", puntuar(&lh, t, true));
            add("híbrido, 2–10 EUCLÍDEO 16d", puntuar(&le, t, true));
            add("solo TF-IDF coseno (=FAISS)", puntuar(&lc, t, true));
        }
        let mut ks: Vec<_> = acc.into_iter().collect();
        ks.sort_by(|a, b| a.0.cmp(b.0));
        for (k, (ok, p)) in ks {
            println!("| {conj} | {k} | {} | {:.1} % ({ok}/{}) | {:.3} |", sem.len(), 100.0 * ok as f64 / sem.len() as f64, sem.len(), p / sem.len() as f64);
        }
        // tiempos en proceso (5 reps, se descarta la 1.ª)
        let mut tt: HashMap<&str, Vec<f64>> = HashMap::new();
        for rep in 0..5 {
            for &s in &sem {
                let xs = &c.docs[s].x;
                let t0 = Instant::now();
                black_box(ib.knn(&c, xs, 10, Some(s as u32)));
                let a = t0.elapsed().as_secs_f64();
                let t0 = Instant::now();
                black_box(scan16(xs, s, 10));
                let b = t0.elapsed().as_secs_f64();
                let t0 = Instant::now();
                black_box(scan1024(&m[s], s, 10));
                let d = t0.elapsed().as_secs_f64();
                let t0 = Instant::now();
                black_box(lx.vecinos_de(s as u32)[..10].to_vec());
                let e = t0.elapsed().as_secs_f64();
                if rep > 0 {
                    tt.entry("knn hiperbólico 16d (bola, f32+reorden f64)").or_default().push(a);
                    tt.entry("knn euclídeo 16d SoA f32").or_default().push(b);
                    tt.entry("knn denso TF-IDF 1024d f32 (fuerza bruta, Rust)").or_default().push(d);
                    tt.entry("lectura de vecinos TF-IDF precalculados (top-10)").or_default().push(e);
                }
            }
        }
        let mut ks: Vec<_> = tt.into_iter().collect();
        ks.sort_by(|a, b| a.0.cmp(b.0));
        for (k, v) in ks {
            println!("TIEMPO {conj} | {k} | p50 {:.4} ms", med(v));
        }
    }
    // T2': híbrido con reorden hiperbólico vs euclídeo vs coseno
    let qs: Vec<(String, String)> = std::fs::read_to_string(heldout.join("t2p-consultas.tsv"))
        .unwrap()
        .lines()
        .filter(|l| !l.starts_with('#'))
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            (f[0].to_string(), f[2].to_string())
        })
        .collect();
    let (mut a, mut b, mut cc) = ((0u32, 0f64), (0u32, 0f64), (0u32, 0f64));
    for (t, q) in &qs {
        let top: Vec<(u32, f32)> = lx.top(q, 50).into_iter().filter(|x| x.1 > 0.0).collect();
        if top.is_empty() {
            continue;
        }
        let xa = &c.docs[top[0].0 as usize].x;
        let mut h: Vec<(u32, f64)> = top[1..].iter().map(|&(i, _)| (i, bola::distancia(xa, &c.docs[i as usize].x))).collect();
        h.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));
        let mut e: Vec<(u32, f64)> = top[1..].iter().map(|&(i, _)| (i, eucl2(xa, &c.docs[i as usize].x))).collect();
        e.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));
        let lh: Vec<u32> = std::iter::once(top[0].0).chain(h.iter().map(|x| x.0)).take(10).collect();
        let le: Vec<u32> = std::iter::once(top[0].0).chain(e.iter().map(|x| x.0)).take(10).collect();
        let lc: Vec<u32> = top.iter().map(|x| x.0).take(10).collect();
        for (acc, l) in [(&mut a, lh), (&mut b, le), (&mut cc, lc)] {
            let r = puntuar(&l, t, false);
            acc.0 += r.0;
            acc.1 += r.1;
        }
    }
    let nq = qs.len() as f64;
    for (k, (ok, p)) in [("híbrido, 2–10 hiperbólico (=@buscar)", a), ("híbrido, 2–10 EUCLÍDEO 16d", b), ("solo TF-IDF coseno (=FAISS)", cc)] {
        println!("| T2' | {k} | {} | {:.1} % ({ok}/{}) | {:.3} |", qs.len(), 100.0 * ok as f64 / nq, qs.len(), p / nq);
    }
    let _ = std::fs::remove_dir_all(&anon);
}
