//! Experimento de VALOR: ¿encontrar un documento relevante es más rápido o más
//! certero con poincare-fs que con una línea base simple?
//!
//! * Corpus con temas conocidos; los nombres se **anonimizan** (`dNNNN`, orden
//!   barajado con semilla fija) para que ningún método haga trampa con el nombre.
//! * Relevante = mismo tema (prefijo del nombre original).
//! * T1 «más como este»: dado un documento semilla, encontrar otro del mismo tema.
//! * T2 «consulta»: dada una consulta escrita a mano (`bench/consultas-*.tsv`),
//!   encontrar un documento del tema.
//! * Métodos: poincare-fs montado (FUSE real), bola sin FUSE (γ = 0),
//!   TF-IDF plano (coseno, mismo espacio de 1024 dim que usa el FS antes de la
//!   bola), ripgrep (proceso `rg` por consulta) y, en T2, un agente que navega
//!   el FS con `cd`/`ls`/`cat` y un híbrido rg → `cd`.

use poincare_fs::ajuste::Rng;
use poincare_fs::{bola, embed};
use poincare_fs::proyeccion::{self, Corpus, Params};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

pub(crate) const STOP: &[&str] = &[
    "the", "a", "an", "and", "or", "of", "to", "in", "on", "for", "with", "by", "is", "are", "be", "it", "this",
    "that", "from", "as", "at", "into", "its", "their", "them", "then", "than", "which", "when", "where", "how",
    "what", "who", "was", "were", "has", "have", "had", "not", "no", "but", "if", "else", "can", "will", "would",
    "should", "could", "may", "might", "do", "does", "did", "so", "such", "these", "those", "there", "here", "also",
    "using", "use", "used", "about", "over", "under", "between", "all", "any", "each", "other", "some", "more",
    "most", "only", "own", "same", "very", "just", "one", "two", "new", "get", "set", "we", "you", "your", "our",
    "they", "he", "she", "his", "her", "i", "me", "my", "up", "out", "via", "per", "like", "way", "back", "later",
    "none", "true", "false", "self", "return", "returns", "default", "value", "values", "must", "need", "needs",
    "into", "because", "being", "been", "after", "before", "while", "until", "since", "whether", "otherwise",
];

pub(crate) fn tema(n: &str) -> &str {
    n.split('-').next().unwrap_or("")
}

pub(crate) fn palabras(t: &str) -> Vec<String> {
    t.split(|c: char| !(c.is_alphanumeric() || c == '_'))
        .filter(|p| !p.is_empty())
        .map(|p| p.to_lowercase())
        .collect()
}

pub(crate) fn palabras_consulta(q: &str) -> Vec<String> {
    let mut v: Vec<String> = palabras(q).into_iter().filter(|w| !STOP.contains(&w.as_str())).collect();
    let mut vistos = HashSet::new();
    v.retain(|w| vistos.insert(w.clone()));
    v
}

/// Las 3 palabras más frecuentes (≥ 4 letras, sin vacías) de un documento: lo que
/// un usuario le daría a `rg` para buscar «más como este».
pub(crate) fn claves_documento(t: &str) -> Vec<String> {
    let mut cuenta: HashMap<String, (usize, usize)> = HashMap::new();
    for (pos, w) in palabras(t).into_iter().enumerate() {
        if w.chars().count() < 4 || STOP.contains(&w.as_str()) || w.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        cuenta.entry(w).or_insert((0, pos)).0 += 1;
    }
    let mut v: Vec<(String, (usize, usize))> = cuenta.into_iter().collect();
    v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0).then(a.1 .1.cmp(&b.1 .1)));
    v.into_iter().take(3).map(|x| x.0).collect()
}

pub(crate) fn percentil(v: &[f64], p: f64) -> f64 {
    let mut v = v.to_vec();
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        return f64::NAN;
    }
    v[((p / 100.0) * (v.len() - 1) as f64).round() as usize]
}

fn montado(p: &Path) -> bool {
    let m = std::fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let p = p.to_string_lossy();
    m.lines().any(|l| l.split(' ').nth(1) == Some(&p))
}

pub(crate) struct Montaje {
    punto: PathBuf,
    hijo: Option<Child>,
    pub(crate) segundos: f64,
}

impl Montaje {
    pub(crate) fn new(corpus: &Path, punto: &Path) -> Result<Montaje, String> {
        std::fs::create_dir_all(punto).map_err(|e| e.to_string())?;
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        let t0 = Instant::now();
        let mut hijo = Command::new(exe)
            .arg("montar")
            .arg(corpus)
            .arg(punto)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        while !montado(punto) {
            if t0.elapsed() > Duration::from_secs(600) || hijo.try_wait().ok().flatten().is_some() {
                let _ = hijo.kill();
                return Err("no se montó".into());
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        Ok(Montaje { punto: punto.to_path_buf(), hijo: Some(hijo), segundos: t0.elapsed().as_secs_f64() })
    }
}

impl Drop for Montaje {
    fn drop(&mut self) {
        let _ = Command::new("fusermount3").arg("-u").arg(&self.punto).status();
        if let Some(mut h) = self.hijo.take() {
            let _ = h.wait();
        }
    }
}

/// `ls -U` de un directorio montado: los `.txt` en el orden de la vista.
pub(crate) fn ls_txt(dir: &Path) -> Result<Vec<String>, String> {
    let mut v = Vec::new();
    for e in std::fs::read_dir(dir).map_err(|e| format!("ls {}: {e}", dir.display()))? {
        let n = e.map_err(|e| e.to_string())?.file_name().to_string_lossy().into_owned();
        if let Some(s) = n.strip_suffix(".txt") {
            v.push(s.to_string());
        }
    }
    Ok(v)
}

/// `rg` sobre el directorio en disco; devuelve los nombres ordenados por cantidad de coincidencias.
pub(crate) fn rg(dir: &Path, claves: &[String]) -> Result<Vec<String>, String> {
    if claves.is_empty() {
        return Ok(Vec::new());
    }
    let mut c = Command::new("rg");
    c.args(["--no-config", "--count-matches", "--with-filename", "-i", "-w", "-F"]);
    for k in claves {
        c.arg("-e").arg(k);
    }
    c.arg(dir);
    let out = c.output().map_err(|e| format!("rg: {e}"))?;
    let s = String::from_utf8_lossy(&out.stdout);
    let mut v: Vec<(String, u64)> = s
        .lines()
        .filter_map(|l| {
            let (p, n) = l.rsplit_once(':')?;
            let nombre = Path::new(p).file_name()?.to_str()?.strip_suffix(".txt")?.to_string();
            Some((nombre, n.parse().ok()?))
        })
        .collect();
    v.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    Ok(v.into_iter().map(|x| x.0).collect())
}

pub(crate) fn top_tfidf(m: &[Vec<f64>], q: &[f64], excluir: Option<usize>, k: usize) -> Vec<usize> {
    let mut v: Vec<(usize, f64)> =
        m.iter().enumerate().filter(|(i, _)| Some(*i) != excluir).map(|(i, x)| (i, embed::coseno(x, q))).collect();
    let cmp = |a: &(usize, f64), b: &(usize, f64)| b.1.total_cmp(&a.1).then(a.0.cmp(&b.0));
    if v.len() > k {
        v.select_nth_unstable_by(k - 1, cmp);
        v.truncate(k);
    }
    v.sort_by(cmp);
    v.into_iter().map(|x| x.0).collect()
}

#[derive(Default)]
struct Acum {
    n: usize,
    ok: usize,
    p10: f64,
    acciones: f64,
    tiempos: Vec<f64>,
    fallas: Vec<String>,
}

struct Ctx {
    temas: Vec<String>,
    tam_tema: HashMap<String, usize>,
    idx: HashMap<String, usize>,
    salida: Option<std::fs::File>,
}

impl Ctx {
    /// Éxito@1 y precisión@10 (sobre min(10, |tema| − excluidos)).
    fn evaluar(&self, t: &str, lista: &[String], excluir: bool) -> (bool, f64) {
        let rel = |n: &String| self.idx.get(n).is_some_and(|&i| self.temas[i] == t);
        let ok = lista.first().is_some_and(rel);
        let tope = (self.tam_tema[t] - usize::from(excluir)).clamp(1, 10);
        let hits = lista.iter().take(10).filter(|n| rel(n)).count();
        (ok, hits as f64 / tope as f64)
    }
    #[allow(clippy::too_many_arguments)]
    fn anotar(&mut self, a: &mut Acum, tarea: &str, met: &str, id: &str, rep: usize, t: &str, lista: &[String], excluir: bool, s: f64, acc: f64, medir: bool) {
        let (ok, p) = self.evaluar(t, lista, excluir);
        if let Some(f) = self.salida.as_mut() {
            let _ = writeln!(f, "{tarea}\t{met}\t{id}\t{rep}\t{t}\t{}\t{p:.3}\t{s:.9}\t{acc}\t{}", ok as u8, lista.first().map(|x| x.as_str()).unwrap_or("-"));
        }
        if medir {
            a.tiempos.push(s);
        }
        if rep == 0 || !medir {
            a.n += 1;
            a.ok += ok as usize;
            a.p10 += p;
            a.acciones += acc;
            if !ok {
                a.fallas.push(id.to_string());
            }
        }
    }
}

fn fila(tarea: &str, met: &str, a: &Acum) {
    println!(
        "| {tarea} | {met} | {} | {:.1} % ({}/{}) | {:.3} | {:.3} | {:.3} | {:.1} |",
        a.n,
        100.0 * a.ok as f64 / a.n.max(1) as f64,
        a.ok,
        a.n,
        a.p10 / a.n.max(1) as f64,
        1e3 * percentil(&a.tiempos, 50.0),
        1e3 * percentil(&a.tiempos, 99.0),
        a.acciones / a.n.max(1) as f64
    );
}

/// Agente que solo usa el FS: lee los `.txt` visibles, puntúa por coincidencias
/// con la consulta y hace `cd` al mejor no visitado. Devuelve (respuesta, cds, cats).
fn agente(punto: &Path, claves: &[String], max_cd: usize, paciencia: usize) -> Result<(Vec<String>, usize, usize), String> {
    let puntuar = |t: &str| -> usize {
        let ws = palabras(t);
        ws.iter().filter(|w| claves.contains(w)).count()
    };
    let mut cur = punto.to_path_buf();
    let mut vistos: HashMap<String, usize> = HashMap::new();
    let mut orden_visto: Vec<String> = Vec::new();
    let mut visitados: HashSet<String> = HashSet::new();
    let (mut cds, mut cats, mut sin_mejora, mut mejor) = (0, 0, 0, 0usize);
    loop {
        let lista = ls_txt(&cur)?;
        let mut mejoro = false;
        for n in &lista {
            if !vistos.contains_key(n) {
                let txt = std::fs::read(cur.join(format!("{n}.txt"))).map_err(|e| e.to_string())?;
                cats += 1;
                let s = puntuar(&String::from_utf8_lossy(&txt));
                if s > mejor {
                    mejor = s;
                    mejoro = true;
                }
                vistos.insert(n.clone(), s);
                orden_visto.push(n.clone());
            }
        }
        sin_mejora = if mejoro { 0 } else { sin_mejora + 1 };
        if cds >= max_cd || sin_mejora >= paciencia {
            break;
        }
        // siguiente: el visible de mayor puntaje no visitado (empate: orden de la vista)
        let sig = lista
            .iter()
            .filter(|n| !visitados.contains(*n))
            .max_by(|a, b| vistos[*a].cmp(&vistos[*b]).then(std::cmp::Ordering::Greater));
        let Some(sig) = sig.cloned() else { break };
        visitados.insert(sig.clone());
        if cur.strip_prefix(punto).map(|r| r.components().count()).unwrap_or(0) >= 60 {
            cur = punto.to_path_buf();
        }
        cur = cur.join(&sig);
        cds += 1;
    }
    let mut resp: Vec<String> = orden_visto.into_iter().filter(|n| vistos[n] > 0).collect();
    resp.sort_by_key(|n| std::cmp::Reverse(vistos[n])); // estable: empate = primero visto
    Ok((resp, cds, cats))
}

/// Llega por `cd` al directorio de `destino` en el FS montado y hace `ls -U` ahí.
/// poincare-fs no permite `cd` a un documento que no esté en la vista actual, así
/// que hay que navegar: en cada paso se entra al documento visible más cercano
/// (distancia hiperbólica en la bola) al destino. Es una navegación con
/// **oráculo** (conoce las coordenadas): el mejor caso para el FS.
/// Devuelve (listado en el destino o None si no llegó, cantidad de `ls` + `cd`).
fn navegar(punto: &Path, c: &Corpus, destino: usize) -> Result<(Option<Vec<String>>, usize), String> {
    let objetivo = &c.docs[destino].nombre;
    let xd = &c.docs[destino].x;
    let mut cur = punto.to_path_buf();
    let mut visitados: HashSet<String> = HashSet::new();
    let mut acciones = 0;
    for _ in 0..60 {
        let l = ls_txt(&cur)?;
        acciones += 1;
        if l.contains(objetivo) {
            let dir = cur.join(objetivo);
            let l2 = ls_txt(&dir)?;
            return Ok((Some(l2), acciones + 2));
        }
        let sig = l
            .iter()
            .filter(|n| !visitados.contains(*n))
            .min_by(|a, b| {
                let da = bola::distancia(&c.docs[c.indice(a).unwrap() as usize].x, xd);
                let db = bola::distancia(&c.docs[c.indice(b).unwrap() as usize].x, xd);
                da.total_cmp(&db)
            });
        let Some(sig) = sig.cloned() else { return Ok((None, acciones)) };
        visitados.insert(sig.clone());
        cur = cur.join(sig);
        acciones += 1;
    }
    Ok((None, acciones))
}

/// Navegación con oráculo en proceso (sin FUSE), misma regla que [`navegar`], con parámetros dados.
fn alcanza_en_proceso(c: &Corpus, destino: usize, p: Params) -> Option<usize> {
    let xd = &c.docs[destino].x;
    let mut ruta: Vec<u32> = Vec::new();
    let mut visitados: HashSet<u32> = HashSet::new();
    for paso in 0..60 {
        let v = proyeccion::vista(c, &ruta, p);
        if v.entradas.iter().any(|e| e.doc as usize == destino) {
            return Some(paso);
        }
        let sig = v
            .entradas
            .iter()
            .filter(|e| !visitados.contains(&e.doc))
            .min_by(|a, b| bola::distancia(&c.docs[a.doc as usize].x, xd).total_cmp(&bola::distancia(&c.docs[b.doc as usize].x, xd)))?;
        visitados.insert(sig.doc);
        ruta.push(sig.doc);
    }
    None
}

/// Con γ = 0 la vista depende solo del foco: el FS es un grafo dirigido de k vecinos.
/// Devuelve cuántos documentos se pueden alcanzar desde la raíz por CUALQUIER camino (BFS).
fn alcanzables_bfs_g0(c: &Corpus) -> usize {
    let p = Params { memoria: 0.0, ..Params::default() };
    let n = c.docs.len();
    let mut visto = vec![false; n];
    let mut cola: std::collections::VecDeque<u32> = proyeccion::vista(c, &[], p).entradas.iter().map(|e| e.doc).collect();
    for &d in &cola {
        visto[d as usize] = true;
    }
    while let Some(d) = cola.pop_front() {
        for e in proyeccion::vista(c, &[d], p).entradas {
            if !visto[e.doc as usize] {
                visto[e.doc as usize] = true;
                cola.push_back(e.doc);
            }
        }
    }
    visto.iter().filter(|&&v| v).count()
}

/// Escribe vectores como f32 little-endian contiguos (para la línea base FAISS en Python).
pub(crate) fn escribir_f32(p: &Path, vs: &[Vec<f64>]) -> Result<(), String> {
    let mut b = Vec::with_capacity(vs.len() * embed::DIM * 4);
    for v in vs {
        for x in v {
            b.extend_from_slice(&(*x as f32).to_le_bytes());
        }
    }
    std::fs::write(p, b).map_err(|e| e.to_string())
}

pub fn correr(original: &Path, consultas: Option<&Path>, reps: usize, max_semillas: usize, salida: Option<&Path>, exportar: Option<&Path>) -> Result<(), String> {
    let reps = reps.max(2);
    // 1. corpus anonimizado
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
    let base = std::env::temp_dir().join(format!("pfs-valor-{}", std::process::id()));
    let anon = base.join("anon");
    std::fs::create_dir_all(&anon).map_err(|e| e.to_string())?;
    let mut tema_de: HashMap<String, String> = HashMap::new();
    for (i, n) in nombres.iter().enumerate() {
        let a = format!("d{i:05}");
        std::fs::copy(original.join(format!("{n}.txt")), anon.join(format!("{a}.txt"))).map_err(|e| e.to_string())?;
        tema_de.insert(a, tema(n).to_string());
    }
    let t0 = Instant::now();
    let c = Corpus::desde_directorio(&anon).map_err(|e| e.to_string())?;
    let t_embed = t0.elapsed().as_secs_f64();
    let n = c.docs.len();
    let temas: Vec<String> = c.docs.iter().map(|d| tema_de[&d.nombre].clone()).collect();
    let mut tam_tema: HashMap<String, usize> = HashMap::new();
    for t in &temas {
        *tam_tema.entry(t.clone()).or_default() += 1;
    }
    let idx: HashMap<String, usize> = c.docs.iter().enumerate().map(|(i, d)| (d.nombre.clone(), i)).collect();
    let textos: Vec<String> = c.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
    let t0 = Instant::now();
    let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
    let idf = embed::idf(&refs);
    let m: Vec<Vec<f64>> = refs.iter().map(|t| embed::vectorizar(t, &idf)).collect();
    let t_tfidf = t0.elapsed().as_secs_f64();
    let salida = match salida {
        Some(p) => {
            let mut f = std::fs::File::create(p).map_err(|e| e.to_string())?;
            let _ = writeln!(f, "tarea\tmetodo\tid\trep\ttema\tok1\tp10\tsegundos\tacciones\ttop1");
            Some(f)
        }
        None => None,
    };
    let mut cx = Ctx { temas: temas.clone(), tam_tema, idx, salida };
    let punto = base.join("mnt");

    println!("corpus: {} ({} docs, {} temas), anonimizado en {}", original.display(), n, cx.tam_tema.len(), anon.display());
    println!("carga: {}", std::fs::read_to_string("/proc/loadavg").unwrap_or_default().trim());
    println!("costo de preparación: embedding en proceso {t_embed:.3} s (lectura + TF-IDF + vecinos + RSGD); índice TF-IDF plano {t_tfidf:.3} s; rg: sin índice");

    // ---------- T1: más como este ----------
    let mut semillas: Vec<usize> = (0..n).collect();
    if n > max_semillas {
        let mut r = Rng::new(7);
        for i in (1..n).rev() {
            let j = r.indice(i + 1);
            semillas.swap(i, j);
        }
        semillas.truncate(max_semillas);
        semillas.sort();
    }
    if let Some(dir) = exportar {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        escribir_f32(&dir.join("docs.f32"), &m)?;
        std::fs::write(dir.join("temas.txt"), temas.join("\n") + "\n").map_err(|e| e.to_string())?;
        let sem: Vec<String> = semillas.iter().map(|s| s.to_string()).collect();
        std::fs::write(dir.join("semillas.txt"), sem.join("\n") + "\n").map_err(|e| e.to_string())?;
        if let Some(q) = consultas {
            let qs: Vec<(String, String, String)> = std::fs::read_to_string(q)
                .map_err(|e| e.to_string())?
                .lines()
                .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
                .filter_map(|l| {
                    let mut p = l.splitn(3, '\t');
                    Some((p.next()?.to_string(), p.next()?.to_string(), p.next()?.to_string()))
                })
                .collect();
            let vs: Vec<Vec<f64>> = qs.iter().map(|q| embed::vectorizar(&q.2, &idf)).collect();
            escribir_f32(&dir.join("consultas.f32"), &vs)?;
            let meta: Vec<String> = qs.iter().map(|q| format!("{}\t{}", q.0, q.1)).collect();
            std::fs::write(dir.join("consultas.txt"), meta.join("\n") + "\n").map_err(|e| e.to_string())?;
        }
        println!("exportado para FAISS en {}", dir.display());
    }
    let mnt = Montaje::new(&anon, &punto)?;
    println!("montaje T1: {:.3} s", mnt.segundos);
    // control: la raíz montada coincide con el cálculo sin FUSE
    let raiz: Vec<String> = proyeccion::vista(&c, &[], Params::default())
        .entradas
        .iter()
        .map(|e| c.docs[e.doc as usize].nombre.clone())
        .collect();
    if ls_txt(&punto)? != raiz {
        return Err("la raíz montada no coincide con la vista calculada".into());
    }
    let mut a_pfs_frio = Acum::default();
    let mut a_pfs = Acum::default();
    let mut a_bola = Acum::default();
    let mut a_tf = Acum::default();
    let mut a_rg = Acum::default();
    let p0 = Params { memoria: 0.0, ..Params::default() };
    let mut no_llego = 0usize;
    let mut acc_pfs: Vec<f64> = Vec::new();
    for rep in 0..reps {
        for &s in &semillas {
            let id = c.docs[s].nombre.clone();
            let t = temas[s].clone();
            // poincare-fs montado: cd <semilla>; ls -U
            let t0 = Instant::now();
            let (l, acc) = navegar(&punto, &c, s)?;
            let dt = t0.elapsed().as_secs_f64();
            if l.is_none() && rep == 0 {
                no_llego += 1;
            }
            if rep == 0 {
                acc_pfs.push(acc as f64);
            }
            let l = l.unwrap_or_default();
            if rep == 0 {
                cx.anotar(&mut a_pfs_frio, "T1", "pfs-frio", &id, 0, &t, &l, true, dt, acc as f64, false);
                a_pfs_frio.tiempos.push(dt);
            } else {
                cx.anotar(&mut a_pfs, "T1", "pfs-caliente", &id, rep - 1, &t, &l, true, dt, acc as f64, true);
            }
            // bola sin FUSE, γ = 0 (solo distancia hiperbólica a la semilla)
            let t0 = Instant::now();
            let v = proyeccion::vista(&c, &[s as u32], p0);
            let dt = t0.elapsed().as_secs_f64();
            let l: Vec<String> = v.entradas.iter().map(|e| c.docs[e.doc as usize].nombre.clone()).collect();
            cx.anotar(&mut a_bola, "T1", "bola-g0", &id, rep, &t, &l, true, dt, 0.0, true);
            // TF-IDF plano
            let t0 = Instant::now();
            let top = top_tfidf(&m, &m[s], Some(s), 10);
            let dt = t0.elapsed().as_secs_f64();
            let l: Vec<String> = top.iter().map(|&i| c.docs[i].nombre.clone()).collect();
            cx.anotar(&mut a_tf, "T1", "tfidf-plano", &id, rep, &t, &l, true, dt, 0.0, true);
            // ripgrep con 3 palabras clave de la semilla
            let claves = claves_documento(&textos[s]);
            let t0 = Instant::now();
            let mut l = rg(&anon, &claves)?;
            let dt = t0.elapsed().as_secs_f64();
            l.retain(|x| *x != id);
            cx.anotar(&mut a_rg, "T1", "rg-3claves", &id, rep, &t, &l, true, dt, 1.0, true);
        }
    }
    drop(mnt);
    println!(
        "navegación hasta la semilla (oráculo): no llegó en {no_llego}/{}; acciones ls+cd p50 {} p99 {} máx {}",
        semillas.len(),
        percentil(&acc_pfs, 50.0),
        percentil(&acc_pfs, 99.0),
        percentil(&acc_pfs, 100.0)
    );
    let p05 = Params::default();
    let g05 = semillas.iter().filter(|&&s| alcanza_en_proceso(&c, s, p05).is_some()).count();
    let g0 = semillas.iter().filter(|&&s| alcanza_en_proceso(&c, s, p0).is_some()).count();
    println!(
        "alcance (en proceso, misma navegación con oráculo): γ=0.5 (por defecto) {g05}/{} semillas; γ=0 {g0}/{}",
        semillas.len(),
        semillas.len()
    );
    println!("alcance γ=0 por CUALQUIER camino (BFS en el grafo de k={} vecinos): {}/{n} documentos", p0.k, alcanzables_bfs_g0(&c));
    println!("\n### T1 «más como este» ({} semillas, {} repeticiones; éxito y P@10 de la primera repetición)\n", semillas.len(), reps);
    println!("| tarea | método | n | éxito@1 | P@10 | p50 (ms) | p99 (ms) | acciones |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    fila("T1", "poincare-fs montado: navegar hasta la semilla + ls, frío", &a_pfs_frio);
    fila("T1", "poincare-fs montado: navegar hasta la semilla + ls, caliente (LRU)", &a_pfs);
    fila("T1", "bola 16d sin FUSE, γ=0, acceso directo (no existe en el FS)", &a_bola);
    fila("T1", "TF-IDF plano 1024d (coseno)", &a_tf);
    fila("T1", "ripgrep, 3 palabras clave de la semilla", &a_rg);

    // ---------- T2: consultas ----------
    let Some(q) = consultas else {
        let _ = std::fs::remove_dir_all(&base);
        return Ok(());
    };
    let qs: Vec<(String, String, String)> = std::fs::read_to_string(q)
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|l| !l.starts_with('#') && !l.trim().is_empty())
        .filter_map(|l| {
            let mut p = l.splitn(3, '\t');
            Some((p.next()?.to_string(), p.next()?.to_string(), p.next()?.to_string()))
        })
        .collect();
    for (t, _, _) in &qs {
        if !cx.tam_tema.contains_key(t) {
            return Err(format!("tema {t} de las consultas no está en el corpus"));
        }
    }
    let mnt = Montaje::new(&anon, &punto)?;
    println!("\nmontaje T2: {:.3} s", mnt.segundos);
    let mut acs: BTreeMap<(String, String), Acum> = BTreeMap::new();
    for rep in 0..reps {
        for (qi, (t, tipo, texto)) in qs.iter().enumerate() {
            let id = format!("q{qi:02}{tipo}");
            let claves = palabras_consulta(texto);
            let mut put = |cx: &mut Ctx, met: &str, l: &[String], dt: f64, acc: f64, r: usize| {
                for k in [tipo.clone(), "todas".to_string()] {
                    let a = acs.entry((met.to_string(), k.clone())).or_default();
                    // la fila «todas» no vuelve a escribir el TSV
                    if k == "todas" {
                        let (ok, p) = cx.evaluar(t, l, false);
                        a.tiempos.push(dt);
                        if r == 0 {
                            a.n += 1;
                            a.ok += ok as usize;
                            a.p10 += p;
                            a.acciones += acc;
                            if !ok {
                                a.fallas.push(id.clone());
                            }
                        }
                    } else {
                        cx.anotar(a, "T2", met, &id, r, t, l, false, dt, acc, true);
                    }
                }
            };
            // rg
            let t0 = Instant::now();
            let l_rg = rg(&anon, &claves)?;
            let dt_rg = t0.elapsed().as_secs_f64();
            put(&mut cx, "rg", &l_rg, dt_rg, 1.0, rep);
            // TF-IDF plano
            let t0 = Instant::now();
            let qv = embed::vectorizar(texto, &idf);
            let top = top_tfidf(&m, &qv, None, 10);
            let dt = t0.elapsed().as_secs_f64();
            let l: Vec<String> = top.iter().map(|&i| c.docs[i].nombre.clone()).collect();
            put(&mut cx, "tfidf", &l, dt, 0.0, rep);
            // híbrido: rg da la entrada, poincare-fs completa con cd + ls
            let t0 = Instant::now();
            let l_rg2 = rg(&anon, &claves)?;
            let mut l = Vec::new();
            let mut acc = 1.0;
            if let Some(e) = l_rg2.first() {
                l.push(e.clone());
                let (vecinos, a) = navegar(&punto, &c, cx.idx[e])?;
                acc += a as f64;
                l.extend(vecinos.unwrap_or_default().into_iter().take(9));
            }
            let dt = t0.elapsed().as_secs_f64();
            let met = if rep == 0 { "hibrido-frio" } else { "hibrido" };
            put(&mut cx, met, &l, dt, acc, if rep == 0 { 0 } else { rep - 1 });
            // agente que solo navega el FS
            let t0 = Instant::now();
            let (l, cds, cats) = agente(&punto, &claves, 20, 3)?;
            let dt = t0.elapsed().as_secs_f64();
            let met = if rep == 0 { "agente-frio" } else { "agente" };
            put(&mut cx, met, &l, dt, (cds + cats) as f64, if rep == 0 { 0 } else { rep - 1 });
        }
    }
    drop(mnt);
    println!("\n### T2 consultas escritas a mano ({} consultas, {} repeticiones; «frío» = primera pasada tras montar)\n", qs.len(), reps);
    println!("| tarea | método | n | éxito@1 | P@10 | p50 (ms) | p99 (ms) | acciones |");
    println!("|---|---|---:|---:|---:|---:|---:|---:|");
    let etiqueta = |m: &str| match m {
        "rg" => "ripgrep (palabras de la consulta, sin vacías)",
        "tfidf" => "TF-IDF plano 1024d (coseno)",
        "hibrido-frio" => "híbrido rg → cd en poincare-fs, frío",
        "hibrido" => "híbrido rg → cd en poincare-fs, caliente",
        "agente-frio" => "agente que navega poincare-fs, frío",
        "agente" => "agente que navega poincare-fs, caliente",
        _ => "?",
    };
    for k in ["K", "D", "todas"] {
        for met in ["rg", "tfidf", "hibrido-frio", "hibrido", "agente-frio", "agente"] {
            if let Some(a) = acs.get(&(met.to_string(), k.to_string())) {
                fila(&format!("T2-{k}"), etiqueta(met), a);
            }
        }
    }
    println!("\nfallas T2 (primera pasada):");
    for ((met, k), a) in &acs {
        if k == "todas" {
            println!("  {met}: {:?}", a.fallas);
        }
    }
    let _ = std::fs::remove_dir_all(&base);
    Ok(())
}
