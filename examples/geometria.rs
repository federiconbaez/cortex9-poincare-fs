//! Arnés del experimento pre-registrado en `bench/geometria/PREREGISTRO-geometria.md`.
//! Uso: geometria <corpus> <manifiesto.tsv> <val.tsv> <test.tsv> <outdir>
use poincare_fs::ajuste::{self, ParamsAjuste, Rng};
use poincare_fs::bola;
use poincare_fs::embed;
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

const DIMS: [usize; 4] = [2, 4, 8, 16];
const LRS: [f64; 5] = [0.03, 0.1, 0.3, 1.0, 3.0];
const RADIOS: [f64; 2] = [0.95, 0.999];
const SEMILLAS: [u64; 3] = [0x5EED_F00D, 0xA11CE, 0xB0B];
const EPOCAS: usize = 50;
const BOOT: usize = 10_000;
const BOOT_SEED: u64 = 20261026;
const DIST_SEED: u64 = 20261027;
const DIST_PARES: usize = 20_000;
const K: usize = 10;

#[derive(Clone, Copy)]
enum Geom {
    H,
    E,
}
impl Geom {
    fn tag(self) -> &'static str {
        match self {
            Geom::H => "H",
            Geom::E => "E",
        }
    }
}

struct Mani {
    clase: Vec<String>,
    camino: Vec<Vec<String>>,
    idx: HashMap<String, usize>,
}

fn leer_lista(p: &Path) -> Vec<String> {
    fs::read_to_string(p)
        .unwrap()
        .lines()
        .skip(1)
        .filter(|l| !l.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn leer_mani(p: &Path, docs: &[String]) -> Mani {
    let mut clase_de = HashMap::new();
    let mut camino_de = HashMap::new();
    for l in fs::read_to_string(p).unwrap().lines().skip(1) {
        let mut it = l.split('\t');
        let n = it.next().unwrap().to_string();
        let c = it.next().unwrap().to_string();
        let cam = it.next().unwrap().split('/').map(String::from).collect::<Vec<_>>();
        clase_de.insert(n.clone(), c);
        camino_de.insert(n, cam);
    }
    let clase: Vec<String> = docs.iter().map(|n| clase_de[n].clone()).collect();
    let camino: Vec<Vec<String>> = docs.iter().map(|n| camino_de[n].clone()).collect();
    let idx: HashMap<String, usize> = docs.iter().enumerate().map(|(i, n)| (n.clone(), i)).collect();
    Mani { clase, camino, idx }
}

fn knn(xs: &[Vec<f64>], q: usize, k: usize, geom: Geom) -> Vec<usize> {
    let n = xs.len();
    let mut v: Vec<(f64, usize)> = (0..n)
        .filter(|&j| j != q)
        .map(|j| {
            let d = match geom {
                Geom::H => bola::distancia(&xs[q], &xs[j]),
                Geom::E => xs[q].iter().zip(&xs[j]).map(|(a, b)| (a - b) * (a - b)).sum::<f64>().sqrt(),
            };
            (d, j)
        })
        .collect();
    let m = k.min(v.len());
    v.select_nth_unstable_by(m - 1, |a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    v.truncate(m);
    v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    v.into_iter().map(|x| x.1).collect()
}

fn metricas(rels: &[bool]) -> (f64, f64) {
    // rels length = n_consultas * K, packed as top-K per query
    let nq = rels.len() / K;
    let mut r1 = 0usize;
    let mut p10 = 0.0;
    for q in 0..nq {
        let sl = &rels[q * K..(q + 1) * K];
        if sl[0] {
            r1 += 1;
        }
        p10 += sl.iter().filter(|&&x| x).count() as f64 / K as f64;
    }
    (100.0 * r1 as f64 / nq as f64, p10 / nq as f64)
}

fn eval_rels(xs: &[Vec<f64>], geom: Geom, qs: &[usize], clases: &[String]) -> Vec<bool> {
    let mut out = Vec::with_capacity(qs.len() * K);
    for &q in qs {
        for j in knn(xs, q, K, geom) {
            out.push(clases[j] == clases[q]);
        }
        while out.len() % K != 0 {
            out.push(false);
        }
    }
    out
}

fn dist_arbol(a: &[String], b: &[String]) -> f64 {
    let mut lcp = 0;
    while lcp < a.len() && lcp < b.len() && a[lcp] == b[lcp] {
        lcp += 1;
    }
    (a.len() + b.len() - 2 * lcp) as f64
}

fn distorsion(xs: &[Vec<f64>], caminos: &[Vec<String>], qs: &[usize], geom: Geom) -> (f64, f64) {
    let mut rng = Rng::new(DIST_SEED);
    let m = qs.len();
    let mut dt = Vec::with_capacity(DIST_PARES);
    let mut de = Vec::with_capacity(DIST_PARES);
    for _ in 0..DIST_PARES {
        let i = qs[rng.indice(m)];
        let j = qs[rng.indice(m)];
        if i == j {
            continue;
        }
        let t = dist_arbol(&caminos[i], &caminos[j]);
        if t <= 0.0 {
            continue;
        }
        let e = match geom {
            Geom::H => bola::distancia(&xs[i], &xs[j]),
            Geom::E => xs[i].iter().zip(&xs[j]).map(|(a, b)| (a - b) * (a - b)).sum::<f64>().sqrt(),
        };
        if e > 0.0 && e.is_finite() {
            dt.push(t);
            de.push(e);
        }
    }
    if dt.is_empty() {
        return (f64::NAN, f64::NAN);
    }
    let mut ratios: Vec<f64> = dt.iter().zip(&de).map(|(t, e)| t / e).collect();
    ratios.sort_by(|a, b| a.total_cmp(b));
    let s = ratios[ratios.len() / 2];
    let d: f64 = dt.iter().zip(&de).map(|(t, e)| (s * e - t).abs() / t).sum::<f64>() / dt.len() as f64;
    // Spearman
    let mut ri: Vec<usize> = (0..dt.len()).collect();
    let mut rj = ri.clone();
    ri.sort_by(|&a, &b| dt[a].total_cmp(&dt[b]).then(a.cmp(&b)));
    rj.sort_by(|&a, &b| de[a].total_cmp(&de[b]).then(a.cmp(&b)));
    let mut rank_t = vec![0.0; dt.len()];
    let mut rank_e = vec![0.0; dt.len()];
    for (r, &i) in ri.iter().enumerate() {
        rank_t[i] = r as f64;
    }
    for (r, &i) in rj.iter().enumerate() {
        rank_e[i] = r as f64;
    }
    let n = dt.len() as f64;
    let mt = (n - 1.0) / 2.0;
    let (mut num, mut dt2, mut de2) = (0.0, 0.0, 0.0);
    for i in 0..dt.len() {
        let a = rank_t[i] - mt;
        let b = rank_e[i] - mt;
        num += a * b;
        dt2 += a * a;
        de2 += b * b;
    }
    let rho = if dt2 > 0.0 && de2 > 0.0 { num / (dt2 * de2).sqrt() } else { f64::NAN };
    (d, rho)
}

fn bootstrap_diff(h: &[bool], e: &[bool]) -> (f64, f64, f64) {
    // h,e: packed top-K; we only need top-1
    let n = h.len() / K;
    let mut dh = Vec::with_capacity(n);
    let mut de = Vec::with_capacity(n);
    for i in 0..n {
        dh.push(if h[i * K] { 1.0 } else { 0.0 });
        de.push(if e[i * K] { 1.0 } else { 0.0 });
    }
    let mean: f64 = dh.iter().zip(&de).map(|(a, b)| a - b).sum::<f64>() / n as f64 * 100.0;
    let mut rng = Rng::new(BOOT_SEED);
    let mut diffs = Vec::with_capacity(BOOT);
    for _ in 0..BOOT {
        let mut s = 0.0;
        for _ in 0..n {
            let i = rng.indice(n);
            s += dh[i] - de[i];
        }
        diffs.push(s / n as f64 * 100.0);
    }
    diffs.sort_by(|a, b| a.total_cmp(b));
    let lo = diffs[(0.025 * BOOT as f64) as usize];
    let hi = diffs[(0.975 * BOOT as f64) as usize];
    (mean, lo, hi)
}

fn entrenar(vecinos: &[Vec<u32>], geom: Geom, dim: usize, lr: f64, r_bola: f64, semilla: u64) -> Vec<Vec<f64>> {
    let radio = match geom {
        Geom::H => r_bola,
        Geom::E => 2.0 * r_bola.atanh(),
    };
    let p = ParamsAjuste {
        dim,
        vecinos: 5,
        negativos: 10,
        epocas: EPOCAS,
        lr,
        semilla,
        radio,
    };
    match geom {
        Geom::H => ajuste::ajustar(vecinos, p),
        Geom::E => ajuste::ajustar_euclideo(vecinos, p),
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert!(args.len() == 6, "uso: geometria <corpus> <mani> <val> <test> <outdir>");
    let dir = PathBuf::from(&args[1]);
    let out = PathBuf::from(&args[5]);
    fs::create_dir_all(&out).unwrap();
    let t0 = Instant::now();
    // Carga de textos (sin embedding): solo para TF-IDF
    let mut pares = poincare_fs::proyeccion::leer_directorio(&dir, poincare_fs::es::Modo::Auto).unwrap().1;
    pares.sort_by(|a, b| a.0.cmp(&b.0));
    let docs: Vec<String> = pares.iter().map(|p| p.0.clone()).collect();
    let textos: Vec<String> = pares.iter().map(|p| String::from_utf8_lossy(&p.1).into_owned()).collect();
    let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
    let mani = leer_mani(Path::new(&args[2]), &docs);
    let val_n = leer_lista(Path::new(&args[3]));
    let test_n = leer_lista(Path::new(&args[4]));
    let val: Vec<usize> = val_n.iter().map(|n| mani.idx[n]).collect();
    let test: Vec<usize> = test_n.iter().map(|n| mani.idx[n]).collect();
    eprintln!("[{}] docs={} val={} test={} (TF-IDF…)", dir.display(), docs.len(), val.len(), test.len());
    let tf = embed::tfidf(&refs);
    let vecinos = ajuste::vecinos_tfidf(&tf, 5);
    eprintln!("[{}] vecinos listos en {:.1}s", dir.display(), t0.elapsed().as_secs_f64());

    // Baseline TF-IDF
    let mut r_tf = Vec::new();
    for &q in &test {
        let mut v: Vec<(f64, usize)> = (0..docs.len())
            .filter(|&j| j != q)
            .map(|j| (-embed::coseno(&tf[q], &tf[j]), j))
            .collect();
        v.select_nth_unstable_by(K - 1, |a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        v.truncate(K);
        v.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
        for (_, j) in v {
            r_tf.push(mani.clase[j] == mani.clase[q]);
        }
    }
    let (r1_tf, p10_tf) = metricas(&r_tf);
    eprintln!("[{}] TF-IDF baseline: R@1={r1_tf:.1} P@10={p10_tf:.3}", dir.display());

    #[derive(Clone)]
    struct Celda {
        geom: &'static str,
        dim: usize,
        lr: f64,
        radio: f64,
        semilla: u64,
        r1_val: f64,
        p10_val: f64,
        r1_test: f64,
        p10_test: f64,
        d_dist: f64,
        rho: f64,
        rels_test: Vec<bool>,
    }
    let resultados: Mutex<Vec<Celda>> = Mutex::new(Vec::new());
    let trabajos: Vec<(Geom, usize, f64, f64, u64)> = {
        let mut v = Vec::new();
        for g in [Geom::H, Geom::E] {
            for &d in &DIMS {
                for &lr in &LRS {
                    for &r in &RADIOS {
                        for &s in &SEMILLAS {
                            v.push((g, d, lr, r, s));
                        }
                    }
                }
            }
        }
        v
    };
    let hilos = std::thread::available_parallelism().map(|h| h.get()).unwrap_or(1).clamp(1, 8);
    let trozo = trabajos.len().div_ceil(hilos).max(1);
    eprintln!("[{}] {} entrenamientos en {hilos} hilos…", dir.display(), trabajos.len());
    std::thread::scope(|sc| {
        for bloque in trabajos.chunks(trozo) {
            let vecinos = &vecinos;
            let mani = &mani;
            let val = &val;
            let test = &test;
            let resultados = &resultados;
            sc.spawn(move || {
                for &(geom, dim, lr, r, semilla) in bloque {
                    let t1 = Instant::now();
                    let xs = entrenar(vecinos, geom, dim, lr, r, semilla);
                    let rels_v = eval_rels(&xs, geom, val, &mani.clase);
                    let (r1v, p10v) = metricas(&rels_v);
                    let rels_t = eval_rels(&xs, geom, test, &mani.clase);
                    let (r1t, p10t) = metricas(&rels_t);
                    let (dd, rho) = distorsion(&xs, &mani.camino, test, geom);
                    eprintln!(
                        "  {} d={dim} lr={lr} r={r} s={semilla:x}: val R@1={r1v:.1} test R@1={r1t:.1} D={dd:.3} ρ={rho:.3} ({:.1}s)",
                        geom.tag(),
                        t1.elapsed().as_secs_f64()
                    );
                    resultados.lock().unwrap().push(Celda {
                        geom: geom.tag(),
                        dim,
                        lr,
                        radio: r,
                        semilla,
                        r1_val: r1v,
                        p10_val: p10v,
                        r1_test: r1t,
                        p10_test: p10t,
                        d_dist: dd,
                        rho,
                        rels_test: rels_t,
                    });
                }
            });
        }
    });
    let mut todas = resultados.into_inner().unwrap();
    todas.sort_by(|a, b| {
        (a.geom, a.dim, a.lr.to_bits(), a.radio.to_bits(), a.semilla).cmp(&(
            b.geom,
            b.dim,
            b.lr.to_bits(),
            b.radio.to_bits(),
            b.semilla,
        ))
    });
    // crudo
    {
        let mut f = fs::File::create(out.join("crudo.tsv")).unwrap();
        writeln!(f, "geom\tdim\tlr\tradio\tsemilla\tr1_val\tp10_val\tr1_test\tp10_test\tdistorsión\trho").unwrap();
        for c in &todas {
            writeln!(
                f,
                "{}\t{}\t{}\t{}\t{:x}\t{:.4}\t{:.4}\t{:.4}\t{:.4}\t{:.6}\t{:.6}",
                c.geom, c.dim, c.lr, c.radio, c.semilla, c.r1_val, c.p10_val, c.r1_test, c.p10_test, c.d_dist, c.rho
            )
            .unwrap();
        }
    }
    // elegir config por (geom, dim)
    let mut resumen = String::new();
    resumen.push_str(&format!(
        "# Resultados {}\nTF-IDF baseline test: R@1={r1_tf:.2} P@10={p10_tf:.4}\n\n",
        dir.file_name().unwrap().to_string_lossy()
    ));
    resumen.push_str("| geom | dim | lr | r | semillas R@1 test | mediana R@1 | mediana P@10 | D | ρ | Δ vs E (pp) | IC95 Δ | veredicto |\n");
    resumen.push_str("|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---|---|\n");
    // Also write chosen configs
    let mut elegidas: HashMap<(String, usize), (f64, f64)> = HashMap::new();
    let mut rels_elegidas: HashMap<(String, usize, u64), Vec<bool>> = HashMap::new();
    for geom in ["H", "E"] {
        for &dim in &DIMS {
            // agrupar por (lr, radio)
            let mut por_cfg: HashMap<(u64, u64), Vec<&Celda>> = HashMap::new();
            for c in &todas {
                if c.geom == geom && c.dim == dim {
                    por_cfg.entry((c.lr.to_bits(), c.radio.to_bits())).or_default().push(c);
                }
            }
            let mut mejor: Option<(f64, f64, f64, f64, Vec<&Celda>)> = None; // mean_r1, mean_p10, lr, r, cells
            for cells in por_cfg.values() {
                assert_eq!(cells.len(), 3);
                let mr1: f64 = cells.iter().map(|c| c.r1_val).sum::<f64>() / 3.0;
                let mp10: f64 = cells.iter().map(|c| c.p10_val).sum::<f64>() / 3.0;
                let lr = cells[0].lr;
                let r = cells[0].radio;
                let cand = (mr1, mp10, -lr, -r, cells.clone()); // -lr,-r for tie-break smaller
                if mejor.as_ref().map(|b| (b.0, b.1, b.2, b.3) < (cand.0, cand.1, cand.2, cand.3)).unwrap_or(true) {
                    mejor = Some((mr1, mp10, -lr, -r, cells.clone()));
                }
            }
            let cells = mejor.unwrap().4;
            elegidas.insert((geom.to_string(), dim), (cells[0].lr, cells[0].radio));
            for c in &cells {
                rels_elegidas.insert((geom.to_string(), dim, c.semilla), c.rels_test.clone());
            }
        }
    }
    // write chosen
    {
        let mut f = fs::File::create(out.join("elegidas.tsv")).unwrap();
        writeln!(f, "geom\tdim\tlr\tradio").unwrap();
        for geom in ["H", "E"] {
            for &dim in &DIMS {
                let (lr, r) = elegidas[&(geom.to_string(), dim)];
                writeln!(f, "{geom}\t{dim}\t{lr}\t{r}").unwrap();
            }
        }
    }
    for &dim in &DIMS {
        for geom in ["H", "E"] {
            let (lr, r) = elegidas[&(geom.to_string(), dim)];
            let cells: Vec<&Celda> = todas
                .iter()
                .filter(|c| c.geom == geom && c.dim == dim && c.lr == lr && c.radio == r)
                .collect();
            let mut r1s: Vec<f64> = cells.iter().map(|c| c.r1_test).collect();
            let mut p10s: Vec<f64> = cells.iter().map(|c| c.p10_test).collect();
            let mut ds: Vec<f64> = cells.iter().map(|c| c.d_dist).collect();
            let mut rhos: Vec<f64> = cells.iter().map(|c| c.rho).collect();
            r1s.sort_by(|a, b| a.total_cmp(b));
            p10s.sort_by(|a, b| a.total_cmp(b));
            ds.sort_by(|a, b| a.total_cmp(b));
            rhos.sort_by(|a, b| a.total_cmp(b));
            let med = |v: &[f64]| v[v.len() / 2];
            let r1_list = cells.iter().map(|c| format!("{:.1}", c.r1_test)).collect::<Vec<_>>().join(",");
            if geom == "H" {
                // compare to E
                let (elr, er) = elegidas[&("E".to_string(), dim)];
                let ecells: Vec<&Celda> = todas
                    .iter()
                    .filter(|c| c.geom == "E" && c.dim == dim && c.lr == elr && c.radio == er)
                    .collect();
                let mut deltas = Vec::new();
                let mut todos_ge2 = true;
                let mut todos_le_m2 = true;
                let mut todos_ic_pos = true;
                let mut todos_ic_neg = true;
                for s in SEMILLAS {
                    let h = &rels_elegidas[&("H".to_string(), dim, s)];
                    let e = &rels_elegidas[&("E".to_string(), dim, s)];
                    let (d, lo, hi) = bootstrap_diff(h, e);
                    deltas.push(d);
                    // per-seed IC for the rule
                    if d < 2.0 {
                        todos_ge2 = false;
                    }
                    if d > -2.0 {
                        todos_le_m2 = false;
                    }
                    if lo <= 0.0 {
                        todos_ic_pos = false;
                    }
                    if hi >= 0.0 {
                        todos_ic_neg = false;
                    }
                }
                // also report combined IC over seed-1 for readability: mean of seed ICs
                let mut ics = Vec::new();
                for s in SEMILLAS {
                    let h = &rels_elegidas[&("H".to_string(), dim, s)];
                    let e = &rels_elegidas[&("E".to_string(), dim, s)];
                    ics.push(bootstrap_diff(h, e));
                }
                let verd = if todos_ge2 && todos_ic_pos {
                    "H GANA"
                } else if todos_le_m2 && todos_ic_neg {
                    "E GANA"
                } else {
                    "sin diferencia demostrada"
                };
                let d_str = deltas.iter().map(|d| format!("{d:+.2}")).collect::<Vec<_>>().join(",");
                let ic_str = ics.iter().map(|(m, lo, hi)| format!("{m:+.2}[{lo:+.2},{hi:+.2}]")).collect::<Vec<_>>().join("; ");
                let _ = ecells;
                resumen.push_str(&format!(
                    "| {geom} | {dim} | {lr} | {r} | {r1_list} | {:.1} | {:.3} | {:.3} | {:.3} | {d_str} | {ic_str} | {verd} |\n",
                    med(&r1s),
                    med(&p10s),
                    med(&ds),
                    med(&rhos)
                ));
            } else {
                resumen.push_str(&format!(
                    "| {geom} | {dim} | {lr} | {r} | {r1_list} | {:.1} | {:.3} | {:.3} | {:.3} | — | — | — |\n",
                    med(&r1s),
                    med(&p10s),
                    med(&ds),
                    med(&rhos)
                ));
            }
        }
    }
    fs::write(out.join("resumen.md"), &resumen).unwrap();
    // guardar embeddings elegidos (H d=2 y d=16, semilla 1) para el montaje
    for &dim in &[2usize, 16] {
        let (lr, r) = elegidas[&("H".to_string(), dim)];
        let xs = entrenar(&vecinos, Geom::H, dim, lr, r, SEMILLAS[0]);
        let mut f = fs::File::create(out.join(format!("emb-H-d{dim}-s0.bin"))).unwrap();
        for x in &xs {
            for v in x {
                f.write_all(&v.to_le_bytes()).unwrap();
            }
        }
        let mut meta = fs::File::create(out.join(format!("emb-H-d{dim}-s0.meta"))).unwrap();
        writeln!(meta, "dim={dim}\nlr={lr}\nradio={r}\nsemilla={:x}\nn={}", SEMILLAS[0], xs.len()).unwrap();
    }
    eprintln!("[{}] listo en {:.1}s\n{resumen}", dir.display(), t0.elapsed().as_secs_f64());
}
