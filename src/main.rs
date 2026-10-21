use poincare_fs::fs::PoincareFs;
use poincare_fs::proyeccion::{self, Corpus, Params};
use std::path::PathBuf;
use std::process::ExitCode;

const USO: &str = "uso:
  poincare-fs montar <corpus_dir> <punto_de_montaje> [--k N] [--radio R]
      Monta (bloquea hasta `fusermount3 -u <punto>`).
  poincare-fs coords <corpus_dir>
      Imprime la coordenada en el disco de cada documento.
  poincare-fs calidad <corpus_dir>
      Mide (tema = prefijo antes del primer '-') cuánto conserva el disco la semántica.
  poincare-fs vista <corpus_dir> [doc1 doc2 ...] [--k N] [--radio R]
      Calcula sin FUSE la vista tras `cd doc1/doc2/...`.";

fn params(args: &mut Vec<String>) -> Result<Params, String> {
    let mut p = Params::default();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--k" | "--radio" => {
                let v = args.get(i + 1).ok_or("falta valor")?.clone();
                if args[i] == "--k" {
                    p.k = v.parse().map_err(|_| "k inválido")?;
                } else {
                    p.radio = v.parse().map_err(|_| "radio inválido")?;
                    if !p.radio.is_finite() || p.radio < 0.0 {
                        return Err("radio inválido".into());
                    }
                }
                args.drain(i..i + 2);
            }
            _ => i += 1,
        }
    }
    Ok(p)
}

fn run() -> Result<(), String> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let p = params(&mut args)?;
    let cmd = args.first().cloned().ok_or(USO)?;
    let corpus_dir = PathBuf::from(args.get(1).ok_or(USO)?);
    let corpus = Corpus::desde_directorio(&corpus_dir).map_err(|e| format!("corpus: {e}"))?;
    match cmd.as_str() {
        "montar" => {
            let punto = PathBuf::from(args.get(2).ok_or(USO)?);
            eprintln!(
                "poincare-fs: {} documentos, k={}, radio={}, montando en {}",
                corpus.docs.len(),
                p.k,
                p.radio,
                punto.display()
            );
            let opts = [
                fuser::MountOption::RO,
                fuser::MountOption::FSName("poincarefs".into()),
                fuser::MountOption::Subtype("poincare".into()),
                fuser::MountOption::DefaultPermissions,
            ];
            fuser::mount2(PoincareFs::new(corpus, p), &punto, &opts).map_err(|e| format!("montaje: {e}"))?;
            eprintln!("poincare-fs: desmontado");
        }
        "coords" => {
            for d in &corpus.docs {
                println!("{:+.9} {:+.9} {}", d.z.re, d.z.im, d.nombre);
            }
        }
        "vista" => {
            let mut ruta = Vec::new();
            for n in &args[2..] {
                let j = corpus.indice(n).ok_or(format!("no existe: {n}"))?;
                let v = proyeccion::vista(&corpus, &ruta, p);
                if !v.entradas.iter().any(|e| e.doc == j) {
                    return Err(format!("{n} no es visible desde {}", proyeccion::ruta_texto(&corpus, &ruta)));
                }
                ruta.push(j);
            }
            print!("{}", proyeccion::texto_vista(&corpus, &proyeccion::vista(&corpus, &ruta, p)));
        }
        "calidad" => calidad(&corpus, &corpus_dir)?,
        _ => return Err(USO.into()),
    }
    Ok(())
}

fn calidad(corpus: &Corpus, dir: &std::path::Path) -> Result<(), String> {
    use poincare_fs::{embed, hiper::distancia};
    let n = corpus.docs.len();
    if n < 2 {
        return Err("hacen falta al menos 2 documentos".into());
    }
    let tema = |i: usize| corpus.docs[i].nombre.split('-').next().unwrap_or("").to_string();
    let textos: Vec<String> = corpus.docs.iter().map(|d| String::from_utf8_lossy(&d.contenido).into_owned()).collect();
    let refs: Vec<&str> = textos.iter().map(|s| s.as_str()).collect();
    let alta = embed::tfidf(&refs);
    let (mut intra, mut ni, mut inter, mut ne) = (0.0, 0, 0.0, 0);
    let (mut acierto_disco, mut acierto_tfidf) = (0, 0);
    for i in 0..n {
        let mut mejor_d = (f64::INFINITY, 0);
        let mut mejor_c = (f64::NEG_INFINITY, 0);
        for j in 0..n {
            if i == j {
                continue;
            }
            let d = distancia(corpus.docs[i].z, corpus.docs[j].z);
            if tema(i) == tema(j) {
                intra += d;
                ni += 1;
            } else {
                inter += d;
                ne += 1;
            }
            if d < mejor_d.0 {
                mejor_d = (d, j);
            }
            let c = embed::coseno(&alta[i], &alta[j]);
            if c > mejor_c.0 {
                mejor_c = (c, j);
            }
        }
        acierto_disco += (tema(mejor_d.1) == tema(i)) as usize;
        acierto_tfidf += (tema(mejor_c.1) == tema(i)) as usize;
    }
    println!("corpus: {} ({} documentos)", dir.display(), n);
    println!("distancia_hiperbolica_media_mismo_tema: {:.6}", intra / ni.max(1) as f64);
    println!("distancia_hiperbolica_media_distinto_tema: {:.6}", inter / ne.max(1) as f64);
    println!("vecino_mas_cercano_mismo_tema_en_disco_2d: {acierto_disco}/{n}");
    println!("vecino_mas_cercano_mismo_tema_en_tfidf_{}d: {acierto_tfidf}/{n}", embed::DIM);
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
