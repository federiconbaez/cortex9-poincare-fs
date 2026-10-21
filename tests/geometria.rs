//! El arnés de geometría y el FS montado dan el mismo `@similar`.
use poincare_fs::ajuste::{self, ParamsAjuste};
use poincare_fs::bola;
use poincare_fs::directo::{IndiceBola, IndiceLexico};
use poincare_fs::embed;
use poincare_fs::proyeccion::{Corpus, Opciones, Params};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_poincare-fs");

fn montado(p: &Path) -> bool {
    let m = std::fs::read_to_string("/proc/self/mounts").unwrap();
    m.lines().any(|l| l.split(' ').nth(1) == Some(p.to_str().unwrap()))
}

struct Montaje {
    punto: PathBuf,
    hijo: Option<Child>,
}
impl Montaje {
    fn nuevo(corpus: &Path, extra: &[&str]) -> Montaje {
        let base = std::env::temp_dir().join(format!("poincare-geo-{}-{}", std::process::id(), extra.join("-")));
        let punto = base.join("mnt");
        std::fs::create_dir_all(&punto).unwrap();
        let hijo = Command::new(BIN)
            .arg("montar")
            .arg(corpus)
            .arg(&punto)
            .args(extra)
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let t = Instant::now();
        while !montado(&punto) {
            assert!(t.elapsed() < Duration::from_secs(120), "no se montó");
            std::thread::sleep(Duration::from_millis(50));
        }
        Montaje { punto, hijo: Some(hijo) }
    }
}
impl Drop for Montaje {
    fn drop(&mut self) {
        let _ = Command::new("fusermount3").arg("-u").arg(&self.punto).status();
        if let Some(mut h) = self.hijo.take() {
            let _ = h.kill();
            let _ = h.wait();
        }
        let _ = std::fs::remove_dir_all(self.punto.parent().unwrap());
    }
}

#[test]
fn euclideo_y_hiperbolico_distintos_mismos_pares() {
    // Mismo grafo, mismas semillas → dos embeddings; el euclídeo no es proyección.
    let textos = [
        "alpha beta gamma",
        "alpha beta delta",
        "gamma delta epsilon",
        "zeta eta theta",
        "zeta eta iota",
        "theta iota kappa",
    ];
    let tf = embed::tfidf(&textos);
    let v = ajuste::vecinos_tfidf(&tf, 2);
    let ph = ParamsAjuste { dim: 4, vecinos: 2, negativos: 3, epocas: 40, lr: 0.3, semilla: 7, radio: 0.95 };
    let pe = ParamsAjuste { dim: 4, vecinos: 2, negativos: 3, epocas: 40, lr: 0.3, semilla: 7, radio: 2.0 * 0.95_f64.atanh() };
    let h = ajuste::ajustar(&v, ph);
    let e = ajuste::ajustar_euclideo(&v, pe);
    assert_ne!(h, e);
    // Ambos ponen pares vecinos más cerca que no-vecinos (distancia propia).
    assert!(bola::distancia(&h[0], &h[1]) < bola::distancia(&h[0], &h[3]));
    let de = |a: &[f64], b: &[f64]| a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum::<f64>();
    assert!(de(&e[0], &e[1]) < de(&e[0], &e[3]));
}

#[test]
fn similar_montado_coincide_con_indice_bola_dim2() {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let o = Opciones {
        ajuste: ParamsAjuste { dim: 2, epocas: 30, ..Default::default() },
        ..Default::default()
    };
    let c = Corpus::desde_directorio_con(&corpus, &o).unwrap();
    let ib = IndiceBola::new(&c);
    let _lx = IndiceLexico::new(&c);
    let m = Montaje::nuevo(
        &corpus,
        &["--dim", "2", "--epocas", "30", "--k", "10"],
    );
    for (j, d) in c.docs.iter().enumerate().take(5) {
        let mut ents: Vec<String> = std::fs::read_dir(m.punto.join("@similar").join(&d.nombre))
            .unwrap()
            .filter_map(|e| {
                let n = e.ok()?.file_name().to_string_lossy().into_owned();
                n.strip_suffix(".txt").map(String::from)
            })
            .collect();
        // ls no garantiza orden; el FS con ls -U sí. Leemos .resultados.
        let res = std::fs::read_to_string(m.punto.join("@similar").join(&d.nombre).join(".resultados")).unwrap();
        let orden: Vec<String> = res
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .filter_map(|l| l.split_whitespace().nth(2).map(String::from))
            .collect();
        let esp: Vec<String> = ib.knn(&c, &d.x, 10, Some(j as u32)).into_iter().map(|(i, _)| c.docs[i as usize].nombre.clone()).collect();
        assert_eq!(orden, esp, "{}", d.nombre);
        ents.sort();
        let mut esp2 = esp.clone();
        esp2.sort();
        assert_eq!(ents, esp2);
    }
    let _ = Params::default();
}
