//! Re-intento del VALOR (v0.3): acceso directo por la interfaz FUSE real.
//!
//! * T1 / T1': `ls -U @similar/<doc>` (bola pura) en el FS montado.
//! * T2 / T2': `ls -U "@buscar/<consulta>"` (híbrido léxico → bola) y
//!   `ls -U "@buscar-bola/<consulta>"` (orden solo por la bola).
//! * Líneas base: ripgrep (proceso por consulta), TF‑IDF plano en Rust, y FAISS
//!   (`bench/valor_faiss_directo.py`, sobre los vectores que se exportan acá).
//! * Conjuntos: T1 con semillas 7 (la original), 11 y 23; T1' reservado
//!   (`bench/heldout/t1p-semillas.tsv`); T2 (`bench/consultas-stdlib.tsv`);
//!   T2' reservado (`bench/heldout/t2p-consultas.tsv`). El veredicto va sobre T1'/T2'.
//! * Tiempo del FS = pared de `read_dir` completo (lookup + opendir + readdir +
//!   releasedir), que incluye calcular los resultados (no hay caché de resultados).
//!   En T2 incluye vectorizar la consulta; FAISS no la incluye.

use crate::valor::{claves_documento, escribir_f32, ls_txt, palabras_consulta, percentil, rg, tema, top_tfidf, Montaje};
use poincare_fs::ajuste::Rng;
use poincare_fs::directo::{self, Cobertura, IndiceBola, IndiceLexico, Modo};
use poincare_fs::embed;
use poincare_fs::proyeccion::{Corpus, Params};
use std::collections::{BTreeMap, HashMap};
use std::io::Write as _;
use std::path::Path;
use std::time::Instant;

#[derive(Default)]
struct Acum {
    n: usize,
    ok: usize,
    p10: f64,
    t_frio: Vec<f64>,
    t: Vec<f64>,
}

impl Acum {
    fn fila(&self, conjunto: &str, met: &str) -> String {
        let r = |v: &[f64], p: f64| if v.is_empty() { f64::NAN } else { 1e3 * percentil(v, p) };
        format!(
            "| {conjunto} | {met} | {} | {:.1} % ({}/{}) | {:.3} | {:.4} | {:.4} | {:.4} |",
            self.n,
            100.0 * self.ok as f64 / self.n.max(1) as f64,
            self.ok,
            self.n,
            self.p10 / self.n.max(1) as f64,
            r(&self.t, 50.0),
            r(&self.t, 99.0),
            r(&self.t_frio, 50.0),
        )
    }
}

fn leer_tsv(p: &Path, cols: usize) -> Result<Vec<Vec<String>>, String> {
    Ok(std::fs::read_to_string(p)
        .map_err(|e| format!("{}: {e}", p.display()))?
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .map(|l| l.splitn(cols + 1, '\t').take(cols).map(String::from).collect::<Vec<_>>())
        .filter(|v| v.len() == cols)
        .collect())
}

#[allow(clippy::too_many_arguments)]
pub fn correr(original: &Path, consultas: &Path, heldout: &Path, reps: usize, salida: &Path, exportar: &Path) -> Result<(), String> {
    let reps = reps.max(2);
    // corpus anonimizado (idéntico a valor.rs)
    let mut nombres: Vec<String> = std::fs::read_dir(original)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok()?.file_name().to_str()?.strip_suffix(".txt").map(String::from))
        .collect();
    nombres.sort();
    let mut rng = Rng::new(2026);
    for i in (1..nombres.len()).rev() {
        let j = rng.indice(i + 1);
        nombres.swap(i, j);
    }
    let base = std::env::temp_dir().join(format!("pfs-valor3-{}", std::process::id()));
    let anon = base.join("anon");
    std::fs::create_dir_all(&anon).map_err(|e| e.to_string())?;
    let mut tema_de: HashMap<String, String> = HashMap::new();
    for (i, n) in nombres.iter().enumerate() {
        let a = format!("d{i:05}");
        std::fs::copy(original.join(format!("{n}.txt")), anon.join(format!("{a}.txt"))).map_err(|e| e.to_string())?;
        tema_de.insert(a, tema(n).to_string());
    }
    let c = Corpus::desde_directorio(&anon).map_err(|e| e.to_string())?;
    let n = c.docs.len();
    let temas: Vec<String> = c.docs.iter().map(|d| tema_de[&d.nombre].clone()).collect();
    let mut tam: HashMap<String, usize> = HashMap::new();
    for t in &temas {
        *tam.entry(t.clone()).or_default() += 1;
    }
    let idx: HashMap<String, usize> = c.docs.iter().enumerate().map(|(i, d)| (d.nombre.clone(), i)).collect();
    let textos: Vec<String> = c.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
    let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
    let idf = embed::idf(&refs);
    let m: Vec<Vec<f64>> = refs.iter().map(|t| embed::vectorizar(t, &idf)).collect();
    let ib = IndiceBola::new(&c);
    let lx = IndiceLexico::new(&c);
    let p = Params::default();

    // ---- conjuntos ----
    let mut t1: Vec<(String, Vec<usize>)> = Vec::new();
    for s in [7u64, 11, 23] {
        let mut v: Vec<usize> = (0..n).collect();
        let mut r = Rng::new(s);
        for i in (1..n).rev() {
            let j = r.indice(i + 1);
            v.swap(i, j);
        }
        v.truncate(200);
        v.sort();
        t1.push((format!("T1-s{s}"), v));
    }
    let hp: Vec<usize> = leer_tsv(&heldout.join("t1p-semillas.tsv"), 3)?
        .iter()
        .map(|f| idx.get(&f[0]).copied().ok_or(format!("semilla {} no está", f[0])))
        .collect::<Result<_, _>>()?;
    t1.push(("T1'".into(), hp));
    let hpp: Vec<usize> = leer_tsv(&heldout.join("t1pp-semillas.tsv"), 3)?
        .iter()
        .map(|f| idx.get(&f[0]).copied().ok_or(format!("semilla {} no está", f[0])))
        .collect::<Result<_, _>>()?;
    t1.push(("T1''".into(), hpp));
    let q_ref: Vec<(String, String, String)> =
        leer_tsv(consultas, 3)?.into_iter().map(|f| (f[0].clone(), f[1].clone(), f[2].clone())).collect();
    let q_ho: Vec<(String, String, String)> = leer_tsv(&heldout.join("t2p-consultas.tsv"), 4)?
        .into_iter()
        .map(|f| (f[0].clone(), f[1].clone(), f[2].clone()))
        .collect();
    let mut t2 = vec![("T2".to_string(), q_ref), ("T2'".to_string(), q_ho)];
    // PFS_SOLO_REFERENCIA=1: corre solo T1/T2 (para ajustar sin mirar el conjunto reservado)
    if std::env::var("PFS_SOLO_REFERENCIA").is_ok_and(|v| v == "1") {
        t1.retain(|x| !x.0.ends_with('\''));
        t2.retain(|x| !x.0.ends_with('\''));
        println!("modo SOLO REFERENCIA: T1'/T2' no se miden");
    }

    // ---- exportar para FAISS ----
    std::fs::create_dir_all(exportar).map_err(|e| e.to_string())?;
    escribir_f32(&exportar.join("docs.f32"), &m)?;
    std::fs::write(exportar.join("temas.txt"), temas.join("\n") + "\n").map_err(|e| e.to_string())?;
    let mut lista_conj = String::new();
    for (nom, sem) in &t1 {
        let f = format!("t1_{}", nom.replace('\'', "p"));
        debug_assert!(!f.contains('\''));
        std::fs::write(exportar.join(format!("{f}.txt")), sem.iter().map(|s| s.to_string()).collect::<Vec<_>>().join("\n") + "\n")
            .map_err(|e| e.to_string())?;
        lista_conj.push_str(&format!("T1\t{nom}\t{f}\n"));
    }
    for (nom, qs) in &t2 {
        let f = format!("t2_{}", nom.replace('\'', "p"));
        let vs: Vec<Vec<f64>> = qs.iter().map(|q| embed::vectorizar(&q.2, &idf)).collect();
        escribir_f32(&exportar.join(format!("{f}.f32")), &vs)?;
        std::fs::write(exportar.join(format!("{f}.txt")), qs.iter().map(|q| format!("{}\t{}", q.0, q.1)).collect::<Vec<_>>().join("\n") + "\n")
            .map_err(|e| e.to_string())?;
        lista_conj.push_str(&format!("T2\t{nom}\t{f}\n"));
    }
    std::fs::write(exportar.join("conjuntos.tsv"), lista_conj).map_err(|e| e.to_string())?;

    let mut tsv = std::fs::File::create(salida).map_err(|e| e.to_string())?;
    let _ = writeln!(tsv, "conjunto\tmetodo\tid\trep\ttema\tok1\tp10\tsegundos\ttop1");
    let evaluar = |t: &str, lista: &[String], excluir: bool| -> (bool, f64) {
        let rel = |x: &String| idx.get(x).is_some_and(|&i| temas[i] == t);
        let ok = lista.first().is_some_and(rel);
        let tope = (tam[t] - usize::from(excluir)).clamp(1, 10);
        (ok, lista.iter().take(10).filter(|x| rel(x)).count() as f64 / tope as f64)
    };
    let mut acs: BTreeMap<(String, String), Acum> = BTreeMap::new();
    #[allow(clippy::too_many_arguments)]
    let mut anotar = |tsv: &mut std::fs::File, conj: &str, met: &str, id: &str, rep: usize, t: &str, l: &[String], excl: bool, dt: f64| {
        let (ok, p10) = evaluar(t, l, excl);
        let _ = writeln!(tsv, "{conj}\t{met}\t{id}\t{rep}\t{t}\t{}\t{p10:.3}\t{dt:.9}\t{}", ok as u8, l.first().map(|x| x.as_str()).unwrap_or("-"));
        let a = acs.entry((conj.to_string(), met.to_string())).or_default();
        if rep == 0 {
            a.n += 1;
            a.ok += ok as usize;
            a.p10 += p10;
            a.t_frio.push(dt);
        } else {
            a.t.push(dt);
        }
    };
    // PFS_INTERCALAR=1: después de cada consulta medida (salvo rg) corre un `rg` SIN medir, para
    // ver cuánto cambian los tiempos cuando la consulta no llega en un bucle caliente.
    let intercalar = std::env::var("PFS_INTERCALAR").is_ok_and(|v| v == "1");
    let molestar = |anon: &Path| {
        if intercalar {
            let _ = std::process::Command::new("rg").args(["--no-config", "--count-matches", "-i", "-F", "-e", "the"]).arg(anon).output();
        }
    };
    if intercalar {
        println!("modo INTERCALADO: un rg sin medir entre consultas");
    }
    let nombres_de = |v: &[(u32, f64)]| -> Vec<String> { v.iter().map(|x| c.docs[x.0 as usize].nombre.clone()).collect() };

    println!("corpus: {} ({n} docs, {} temas), anonimizado en {}", original.display(), tam.len(), anon.display());
    println!("carga al empezar: {}", std::fs::read_to_string("/proc/loadavg").unwrap_or_default().trim());

    // ---- alcance (en proceso y montado) ----
    let cob = Cobertura::new(&c, p, &ib);
    let prof_max = cob.profundidad.iter().max().copied().unwrap_or(0);
    let mut bfs = vec![false; n];
    {
        // γ = 0: grafo (k vecinos ∪ hijos de cobertura) desde la raíz, por CUALQUIER camino
        let p0 = Params { memoria: 0.0, ..p };
        let mut cola: std::collections::VecDeque<u32> = poincare_fs::proyeccion::vista(&c, &[], p0).entradas.iter().map(|e| e.doc).collect();
        cola.extend(cob.hijos(None).iter().copied());
        for &d in &cola {
            bfs[d as usize] = true;
        }
        while let Some(d) = cola.pop_front() {
            let vec: Vec<u32> = poincare_fs::proyeccion::vista(&c, &[d], p0).entradas.iter().map(|e| e.doc).chain(cob.hijos(Some(d)).iter().copied()).collect();
            for e in vec {
                if !bfs[e as usize] {
                    bfs[e as usize] = true;
                    cola.push_back(e);
                }
            }
        }
    }
    let punto = base.join("mnt");
    let mnt = Montaje::new(&anon, &punto)?;
    println!("montaje: {:.3} s (incluye construir índices y cobertura)", mnt.segundos);
    // verificación montada con γ = 0,5 (por defecto): se recorre el árbol con `ls` reales
    let mut ok_montado = 0;
    let mut acciones = Vec::new();
    for j in 0..n as u32 {
        let ruta = cob.ruta_hasta(j);
        let mut dir = punto.clone();
        for &r in &ruta {
            dir = dir.join(&c.docs[r as usize].nombre);
        }
        if ls_txt(&dir)?.contains(&c.docs[j as usize].nombre) && std::fs::metadata(dir.join(&c.docs[j as usize].nombre)).is_ok() {
            ok_montado += 1;
        }
        acciones.push(ruta.len() as f64 + 1.0);
    }
    println!(
        "alcance: árbol {}/{n} (colgados por cobertura: {}; profundidad máx {prof_max}); γ=0 por cualquier camino {}/{n}; \
         verificado MONTADO con γ=0,5 recorriendo el árbol con cd+ls: {ok_montado}/{n} (ls necesarios p50 {} máx {})",
        cob.alcanzables(),
        cob.agregados,
        bfs.iter().filter(|&&b| b).count(),
        percentil(&acciones, 50.0),
        percentil(&acciones, 100.0)
    );
    let mut extras_max = 0;
    for j in 0..n as u32 {
        let v = poincare_fs::proyeccion::vista(&c, &[j], p);
        extras_max = extras_max.max(cob.hijos(Some(j)).iter().filter(|h| !v.entradas.iter().any(|e| e.doc == **h)).count());
    }
    println!("entradas extra por cobertura en la vista /<doc> (γ=0,5): máx {extras_max}");

    // ---- T1 ---- (cada método en su propio bucle, como FAISS: rep 0 = frío, 1.. = caliente)
    let sim = punto.join("@similar");
    for (conj, sem) in &t1 {
        for met in ["fs-similar", "fs-similar-hibrido", "bola-proceso", "similar-hibrido-proceso", "tfidf-rust", "rg"] {
            for rep in 0..reps {
                for &s in sem {
                    let id = c.docs[s].nombre.clone();
                    let t = temas[s].clone();
                    let claves = if met == "rg" { claves_documento(&textos[s]) } else { Vec::new() };
                    let t0 = Instant::now();
                    let mut l: Vec<String> = match met {
                        "fs-similar" => ls_txt(&sim.join(&id))?,
                        "fs-similar-hibrido" => ls_txt(&punto.join("@similar-hibrido").join(&id))?,
                        "similar-hibrido-proceso" => nombres_de(&directo::buscar(&c, &ib, &lx, Modo::SimilarHibrido, &id, p.k).unwrap_or_default()),
                        "bola-proceso" => nombres_de(&ib.knn(&c, &c.docs[s].x, p.k, Some(s as u32))),
                        "tfidf-rust" => top_tfidf(&m, &m[s], Some(s), 10).iter().map(|&i| c.docs[i].nombre.clone()).collect(),
                        _ => rg(&anon, &claves)?,
                    };
                    let dt = t0.elapsed().as_secs_f64();
                    l.retain(|x| *x != id);
                    anotar(&mut tsv, conj, met, &id, rep, &t, &l, true, dt);
                    if met != "rg" {
                        molestar(&anon);
                    }
                }
            }
        }
    }
    // ---- T2 ----
    for (conj, qs) in &t2 {
        for met in ["fs-buscar-hibrido", "fs-buscar-bola", "hibrido-proceso", "bola-proceso", "lexico-disperso-proceso", "tfidf-rust", "rg"] {
            for rep in 0..reps {
                for (qi, (t, tipo, texto)) in qs.iter().enumerate() {
                    let id = format!("q{qi:02}{tipo}");
                    let claves = if met == "rg" { palabras_consulta(texto) } else { Vec::new() };
                    let t0 = Instant::now();
                    let l: Vec<String> = match met {
                        "fs-buscar-hibrido" => ls_txt(&punto.join("@buscar").join(texto))?,
                        "fs-buscar-bola" => ls_txt(&punto.join("@buscar-bola").join(texto))?,
                        "hibrido-proceso" => nombres_de(&directo::buscar(&c, &ib, &lx, Modo::Hibrido, texto, p.k).unwrap_or_default()),
                        "bola-proceso" => nombres_de(&directo::buscar(&c, &ib, &lx, Modo::Bola, texto, p.k).unwrap_or_default()),
                        "lexico-disperso-proceso" => lx.top(texto, 10).iter().map(|x| c.docs[x.0 as usize].nombre.clone()).collect(),
                        "tfidf-rust" => {
                            let qv = embed::vectorizar(texto, &idf);
                            top_tfidf(&m, &qv, None, 10).iter().map(|&i| c.docs[i].nombre.clone()).collect()
                        }
                        _ => rg(&anon, &claves)?,
                    };
                    let dt = t0.elapsed().as_secs_f64();
                    anotar(&mut tsv, conj, met, &id, rep, t, &l, false, dt);
                    if met != "rg" {
                        molestar(&anon);
                    }
                }
            }
        }
    }
    drop(mnt);
    println!("carga al terminar: {}", std::fs::read_to_string("/proc/loadavg").unwrap_or_default().trim());
    println!("\n| conjunto | método | n | recall@1 | P@10 | p50 caliente (ms) | p99 caliente (ms) | p50 frío (ms) |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    for ((conj, met), a) in &acs {
        println!("{}", a.fila(conj, met));
    }
    let _ = std::fs::remove_dir_all(&base);
    Ok(())
}
