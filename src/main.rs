use poincare_fs::ajuste::{self, ParamsAjuste, Rng};
use poincare_fs::es::{self, Modo};
use poincare_fs::fs::PoincareFs;
use poincare_fs::proyeccion::{self, Corpus, Opciones, Params};
use poincare_fs::{bola, embed, generar, hiper};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{Duration, Instant};

const USO: &str = "uso:
  poincare-fs montar <corpus> <punto> [opciones]   monta (termina con `fusermount3 -u <punto>`)
  poincare-fs vista <corpus> [doc1 doc2 ...]       la vista tras `cd doc1/doc2/...`, sin FUSE
  poincare-fs coords <corpus>                      norma en la bola y posición 2D de cada doc
  poincare-fs calidad <corpus>                     métrica de vecino más cercano del mismo tema
  poincare-fs indexar <corpus> <indice>            calcula y guarda el embedding
  poincare-fs es-info <corpus>                     lee el corpus e informa el modo de E/S usado
  poincare-fs generar <stdlib|sintetico> <n> <dir> [--python DIR] [--semilla S]
  poincare-fs bench <corpus> [--pasos N] [--cache N] [--indice F] [--semilla S]
opciones: --k N  --radio R  --memoria G  --dim D  --epocas E  --indice F  --cache N
E/S: POINCARE_IO=auto|uring|pread";

struct Args {
    pos: Vec<String>,
    flags: HashMap<String, String>,
}

impl Args {
    fn parse() -> Result<Args, String> {
        let mut pos = Vec::new();
        let mut flags = HashMap::new();
        let mut it = std::env::args().skip(1);
        while let Some(a) = it.next() {
            if let Some(f) = a.strip_prefix("--") {
                let v = it.next().ok_or(format!("falta valor para --{f}"))?;
                flags.insert(f.to_string(), v);
            } else {
                pos.push(a);
            }
        }
        Ok(Args { pos, flags })
    }
    fn num<T: std::str::FromStr>(&self, f: &str, def: T) -> Result<T, String> {
        match self.flags.get(f) {
            None => Ok(def),
            Some(v) => v.parse().map_err(|_| format!("--{f} inválido: {v}")),
        }
    }
    fn pos(&self, i: usize) -> Result<&str, String> {
        self.pos.get(i).map(|s| s.as_str()).ok_or_else(|| USO.to_string())
    }
    fn params(&self) -> Result<Params, String> {
        let d = Params::default();
        let p = Params {
            k: self.num("k", d.k)?,
            radio: self.num("radio", d.radio)?,
            memoria: self.num("memoria", d.memoria)?,
        };
        if !p.radio.is_finite() || p.radio < 0.0 || !(0.0..1.0).contains(&p.memoria) {
            return Err("radio debe ser ≥ 0 y memoria ∈ [0, 1)".into());
        }
        Ok(p)
    }
    fn opciones(&self) -> Result<Opciones, String> {
        let d = ParamsAjuste::default();
        Ok(Opciones {
            ajuste: ParamsAjuste {
                dim: self.num("dim", d.dim)?,
                epocas: self.num("epocas", d.epocas)?,
                semilla: self.num("semilla-ajuste", d.semilla)?,
                ..d
            },
            modo_io: Modo::desde_entorno().map_err(|e| e.to_string())?,
            indice: self.flags.get("indice").map(PathBuf::from),
        })
    }
    fn corpus(&self) -> Result<Corpus, String> {
        let dir = PathBuf::from(self.pos(1)?);
        Corpus::desde_directorio_con(&dir, &self.opciones()?).map_err(|e| format!("corpus: {e}"))
    }
}

fn informar_carga(c: &Corpus) {
    eprintln!(
        "poincare-fs: {} documentos, dim={}, E/S={:?}, lectura={:.3}s, embedding={:.3}s, índice={}",
        c.docs.len(),
        c.dim,
        c.info.modo_io,
        c.info.segundos_lectura,
        c.info.segundos_embedding,
        c.info.indice.as_deref().unwrap_or("no")
    );
}

fn tema(n: &str) -> &str {
    n.split('-').next().unwrap_or("")
}

/// Cuántos documentos tienen como vecino más cercano (según `dist`) uno del mismo tema.
fn acierto_nn(nombres: &[&str], dist: impl Fn(usize, usize) -> f64) -> usize {
    let n = nombres.len();
    (0..n)
        .filter(|&i| {
            let j = (0..n)
                .filter(|&j| j != i)
                .min_by(|&a, &b| dist(i, a).total_cmp(&dist(i, b)).then(a.cmp(&b)))
                .unwrap();
            tema(nombres[i]) == tema(nombres[j])
        })
        .count()
}

fn calidad(a: &Args) -> Result<(), String> {
    let c = a.corpus()?;
    informar_carga(&c);
    let n = c.docs.len();
    if n < 2 {
        return Err("hacen falta al menos 2 documentos".into());
    }
    let nombres: Vec<&str> = c.docs.iter().map(|d| d.nombre.as_str()).collect();
    let textos: Vec<String> = c.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
    let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
    let tf = embed::tfidf(&refs);
    let disco_viejo = embed::puntos(&refs);
    let disco_nuevo: Vec<(f64, f64)> = c.docs.iter().map(|d| ajuste::a_disco(&d.x, &c.base)).collect();
    let r_tfidf = acierto_nn(&nombres, |i, j| -embed::coseno(&tf[i], &tf[j]));
    let r_viejo = acierto_nn(&nombres, |i, j| hiper::distancia(disco_viejo[i], disco_viejo[j]));
    let r_bola = acierto_nn(&nombres, |i, j| bola::distancia(&c.docs[i].x, &c.docs[j].x));
    let r_2d = acierto_nn(&nombres, |i, j| {
        let (p, q) = (disco_nuevo[i], disco_nuevo[j]);
        hiper::distancia(hiper::C::new(p.0, p.1), hiper::C::new(q.0, q.1))
    });
    // recall@5 de los vecinos TF-IDF dentro de la bola
    let m = 5.min(n - 1);
    let vt = ajuste::vecinos_tfidf(&tf, m);
    let mut comunes = 0usize;
    for (i, vi) in vt.iter().enumerate() {
        let mut orden: Vec<usize> = (0..n).filter(|&j| j != i).collect();
        orden.sort_by(|&x, &y| {
            bola::distancia(&c.docs[i].x, &c.docs[x].x).total_cmp(&bola::distancia(&c.docs[i].x, &c.docs[y].x))
        });
        comunes += orden[..m].iter().filter(|j| vi.contains(&(**j as u32))).count();
    }
    let temas: std::collections::BTreeSet<&str> = nombres.iter().map(|x| tema(x)).collect();
    println!("documentos: {n}  temas: {}", temas.len());
    println!("vecino_mismo_tema tfidf_{}d:            {r_tfidf}/{n}", embed::DIM);
    println!("vecino_mismo_tema disco_2d_pca (v0.1):   {r_viejo}/{n}");
    println!("vecino_mismo_tema bola_{}d (RSGD):       {r_bola}/{n}", c.dim);
    println!("vecino_mismo_tema bola->disco_2d (vista): {r_2d}/{n}");
    println!("recall@{m} vecinos_tfidf en la bola:      {:.4}", comunes as f64 / (n * m) as f64);
    let radios: Vec<f64> = c.docs.iter().map(|d| bola::norma(&d.x)).collect();
    println!(
        "norma en la bola: min {:.4} max {:.4}",
        radios.iter().cloned().fold(f64::INFINITY, f64::min),
        radios.iter().cloned().fold(0.0, f64::max)
    );
    Ok(())
}

fn montado(p: &Path) -> bool {
    let m = std::fs::read_to_string("/proc/self/mounts").unwrap_or_default();
    let p = p.to_string_lossy();
    m.lines().any(|l| l.split(' ').nth(1) == Some(&p))
}

fn percentil(v: &mut [f64], p: f64) -> f64 {
    v.sort_by(f64::total_cmp);
    if v.is_empty() {
        return f64::NAN;
    }
    let i = ((p / 100.0) * (v.len() - 1) as f64).round() as usize;
    v[i]
}

fn bench(a: &Args) -> Result<(), String> {
    let corpus = PathBuf::from(a.pos(1)?);
    let pasos: usize = a.num("pasos", 2000)?;
    let cache: usize = a.num("cache", 4096)?;
    let mut rng = Rng::new(a.num("semilla", 12345u64)?);
    let punto = std::env::temp_dir().join(format!("poincare-bench-{}", std::process::id()));
    std::fs::create_dir_all(&punto).map_err(|e| e.to_string())?;
    let mut cmd = std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
    cmd.arg("montar").arg(&corpus).arg(&punto).arg("--cache").arg(cache.to_string());
    if let Some(i) = a.flags.get("indice") {
        cmd.arg("--indice").arg(i);
    }
    let t0 = Instant::now();
    let mut hijo = cmd.spawn().map_err(|e| e.to_string())?;
    while !montado(&punto) {
        if t0.elapsed() > Duration::from_secs(1800) || hijo.try_wait().ok().flatten().is_some() {
            let _ = hijo.kill();
            return Err("no se montó".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let t_montaje = t0.elapsed().as_secs_f64();
    let (mut ls_frio, mut ls_caliente, mut cd, mut cat) = (vec![], vec![], vec![], vec![]);
    let mut cur = punto.clone();
    let mut prof = 0;
    let mut error = None;
    let t_total = Instant::now();
    for _ in 0..pasos {
        let t = Instant::now();
        let nombres: Vec<String> = match std::fs::read_dir(&cur) {
            Ok(rd) => rd.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect(),
            Err(e) => {
                error = Some(format!("ls {}: {e}", cur.display()));
                break;
            }
        };
        ls_frio.push(t.elapsed().as_secs_f64());
        let t = Instant::now();
        let n2 = std::fs::read_dir(&cur).map(|rd| rd.count()).unwrap_or(0);
        ls_caliente.push(t.elapsed().as_secs_f64());
        assert_eq!(n2, nombres.len());
        let txt: Vec<&String> = nombres.iter().filter(|n| n.ends_with(".txt")).collect();
        let dirs: Vec<&String> = nombres.iter().filter(|n| !n.ends_with(".txt") && !n.starts_with('.')).collect();
        if !txt.is_empty() {
            let f = cur.join(txt[rng.indice(txt.len())]);
            let t = Instant::now();
            if let Err(e) = std::fs::read(&f) {
                error = Some(format!("cat {}: {e}", f.display()));
                break;
            }
            cat.push(t.elapsed().as_secs_f64());
        }
        if dirs.is_empty() || prof >= 32 {
            cur = punto.clone();
            prof = 0;
            continue;
        }
        let siguiente = cur.join(dirs[rng.indice(dirs.len())]);
        let t = Instant::now();
        if let Err(e) = std::env::set_current_dir(&siguiente) {
            error = Some(format!("cd {}: {e}", siguiente.display()));
            break;
        }
        cd.push(t.elapsed().as_secs_f64());
        cur = siguiente;
        prof += 1;
    }
    let total = t_total.elapsed().as_secs_f64();
    let _ = std::env::set_current_dir("/");
    let _ = std::process::Command::new("fusermount3").arg("-u").arg(&punto).status();
    let _ = hijo.wait();
    let _ = std::fs::remove_dir(&punto);
    if let Some(e) = error {
        return Err(e);
    }
    println!("corpus={} pasos={pasos} cache={cache} montaje={t_montaje:.3}s caminata={total:.3}s", corpus.display());
    println!("| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |");
    println!("|---|---:|---:|---:|---:|---:|");
    for (nombre, v) in [("ls (vista nueva)", &mut ls_frio), ("ls (repetido)", &mut ls_caliente), ("cd", &mut cd), ("cat", &mut cat)] {
        let suma: f64 = v.iter().sum();
        let n = v.len();
        let p50 = percentil(v, 50.0) * 1e6;
        let p99 = percentil(v, 99.0) * 1e6;
        println!(
            "| {nombre} | {n} | {p50:.1} | {p99:.1} | {:.1} | {:.0} |",
            suma / n as f64 * 1e6,
            n as f64 / suma
        );
    }
    Ok(())
}

fn run() -> Result<(), String> {
    let a = Args::parse()?;
    match a.pos(0)? {
        "montar" => {
            let p = a.params()?;
            let punto = PathBuf::from(a.pos(2)?);
            let c = a.corpus()?;
            informar_carga(&c);
            let cache: usize = a.num("cache", 4096)?;
            eprintln!("poincare-fs: k={} radio={} memoria={} cache={cache}; montando en {}", p.k, p.radio, p.memoria, punto.display());
            let opts = [
                fuser::MountOption::RO,
                fuser::MountOption::FSName("poincarefs".into()),
                fuser::MountOption::Subtype("poincare".into()),
                fuser::MountOption::DefaultPermissions,
            ];
            let fs = PoincareFs::con_cache(c, p, cache);
            fuser::mount2(fs, &punto, &opts).map_err(|e| format!("montaje: {e}"))?;
            eprintln!("poincare-fs: desmontado");
        }
        "coords" => {
            let c = a.corpus()?;
            for d in &c.docs {
                let (u, v) = ajuste::a_disco(&d.x, &c.base);
                println!("{:.9} {:+.6} {:+.6} {}", bola::norma(&d.x), u, v, d.nombre);
            }
        }
        "vista" => {
            let p = a.params()?;
            let c = a.corpus()?;
            let mut ruta = Vec::new();
            for n in &a.pos[2..] {
                let j = c.indice(n).ok_or(format!("no existe: {n}"))?;
                let v = proyeccion::vista(&c, &ruta, p);
                if !v.entradas.iter().any(|e| e.doc == j) {
                    return Err(format!("{n} no es visible desde {}", proyeccion::ruta_texto(&c, &ruta)));
                }
                ruta.push(j);
            }
            print!("{}", proyeccion::texto_vista(&c, &proyeccion::vista(&c, &ruta, p)));
        }
        "calidad" => calidad(&a)?,
        "indexar" => {
            let mut o = a.opciones()?;
            o.indice = Some(PathBuf::from(a.pos(2)?));
            let c = Corpus::desde_directorio_con(Path::new(a.pos(1)?), &o).map_err(|e| e.to_string())?;
            informar_carga(&c);
        }
        "es-info" => {
            let modo = Modo::desde_entorno().map_err(|e| e.to_string())?;
            let (m, pares, s) = proyeccion::leer_directorio(Path::new(a.pos(1)?), modo).map_err(|e| e.to_string())?;
            let bytes: usize = pares.iter().map(|p| p.1.len()).sum();
            let mut h: u64 = 0xcbf2_9ce4_8422_2325;
            let mut pares = pares;
            pares.sort();
            for (n, c) in &pares {
                for &b in n.as_bytes().iter().chain(c) {
                    h ^= b as u64;
                    h = h.wrapping_mul(0x0000_0100_0000_01b3);
                }
            }
            let sonda = es::sondear_uring().map(|_| "ok".to_string()).unwrap_or_else(|e| e);
            println!("modo_pedido={modo:?} modo_usado={m:?} sonda_uring={sonda} archivos={} bytes={bytes} fnv={h:016x} segundos={s:.4}", pares.len());
        }
        "generar" => {
            let n: usize = a.pos(2)?.parse().map_err(|_| "n inválido")?;
            let dir = PathBuf::from(a.pos(3)?);
            let py = match a.flags.get("python") {
                Some(p) => PathBuf::from(p),
                None => generar::python_por_defecto().ok_or("no encontré /usr/lib/python3.*")?,
            };
            let docs = match a.pos(1)? {
                "stdlib" => generar::generar_stdlib(&py, n),
                "sintetico" => generar::generar_sintetico(&py, n, a.num("semilla", 7u64)?),
                o => return Err(format!("fuente desconocida: {o}")),
            }
            .map_err(|e| e.to_string())?;
            generar::escribir(&dir, &docs).map_err(|e| e.to_string())?;
            let temas: std::collections::BTreeSet<&str> = docs.iter().map(|d| tema(&d.0)).collect();
            println!("{} documentos, {} temas, fuente {} -> {}", docs.len(), temas.len(), py.display(), dir.display());
        }
        "bench" => bench(&a)?,
        _ => return Err(USO.into()),
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
