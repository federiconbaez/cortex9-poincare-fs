//! Integración v0.3: acceso directo (@doc, @similar, @similar-hibrido, @buscar,
//! @buscar-bola) y alcance completo, con el FS montado de verdad.

use poincare_fs::directo::{self, Cobertura, IndiceBola, IndiceLexico, Modo};
use poincare_fs::proyeccion::{self, Corpus, Params};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

const BIN: &str = env!("CARGO_BIN_EXE_poincare-fs");

fn corpus_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus")
}

fn montado(p: &Path) -> bool {
    let m = std::fs::read_to_string("/proc/self/mounts").unwrap();
    m.lines().any(|l| l.split(' ').nth(1) == Some(p.to_str().unwrap()))
}

struct Montaje {
    punto: PathBuf,
    hijo: Option<Child>,
}

impl Montaje {
    fn nuevo(etiqueta: &str, extra: &[&str]) -> Montaje {
        let base = std::env::temp_dir().join(format!("poincare-directo-{}-{etiqueta}", std::process::id()));
        let punto = base.join("mnt");
        std::fs::create_dir_all(&punto).unwrap();
        let hijo = Command::new(BIN)
            .arg("montar")
            .arg(corpus_dir())
            .arg(&punto)
            .args(extra)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let t = Instant::now();
        while !montado(&punto) {
            assert!(t.elapsed() < Duration::from_secs(30), "no se montó");
            std::thread::sleep(Duration::from_millis(20));
        }
        Montaje { punto, hijo: Some(hijo) }
    }
}

impl Drop for Montaje {
    fn drop(&mut self) {
        let _ = Command::new("fusermount3").arg("-u").arg(&self.punto).status();
        if let Some(mut h) = self.hijo.take() {
            let t = Instant::now();
            while h.try_wait().ok().flatten().is_none() && t.elapsed() < Duration::from_secs(10) {
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = h.kill();
            let _ = h.wait();
        }
        let _ = std::fs::remove_dir_all(self.punto.parent().unwrap());
    }
}

fn ls(p: &Path) -> Vec<String> {
    std::fs::read_dir(p).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect()
}

fn txt(v: &[String]) -> Vec<String> {
    v.iter().filter_map(|n| n.strip_suffix(".txt").map(String::from)).collect()
}

#[test]
fn acceso_directo_montado() {
    let c = Corpus::desde_directorio(&corpus_dir()).unwrap();
    let ib = IndiceBola::new(&c);
    let lx = IndiceLexico::new(&c);
    let m = Montaje::nuevo("directo", &[]);
    let r = &m.punto;
    let nombre = |v: &[(u32, f64)]| -> Vec<String> { v.iter().map(|x| c.docs[x.0 as usize].nombre.clone()).collect() };

    // Los mágicos no se listan (el listado de la raíz no cambia).
    assert!(ls(r).iter().all(|n| !n.starts_with('@')));

    // @similar: mismo orden que la bola sin FUSE, sin la semilla, para todos los docs.
    for (j, d) in c.docs.iter().enumerate() {
        let l = txt(&ls(&r.join("@similar").join(&d.nombre)));
        assert_eq!(l, nombre(&ib.knn(&c, &d.x, 10, Some(j as u32))), "{}", d.nombre);
        assert!(!l.contains(&d.nombre));
        let h = txt(&ls(&r.join("@similar-hibrido").join(&d.nombre)));
        assert_eq!(h, nombre(&directo::buscar(&c, &ib, &lx, Modo::SimilarHibrido, &d.nombre, 10).unwrap()));
    }
    // @buscar / @buscar-bola con espacios en la consulta.
    for q in ["kernel ebpf", "receta de pan con harina", "gato"] {
        let h = txt(&ls(&r.join("@buscar").join(q)));
        assert_eq!(h, nombre(&directo::buscar(&c, &ib, &lx, Modo::Hibrido, q, 10).unwrap()));
        let b = txt(&ls(&r.join("@buscar-bola").join(q)));
        assert_eq!(b, nombre(&directo::buscar(&c, &ib, &lx, Modo::Bola, q, 10).unwrap()));
        let res = std::fs::read_to_string(r.join("@buscar").join(q).join(".resultados")).unwrap();
        assert!(res.contains(&h[0]));
        // cat de un resultado = contenido original
        let orig = std::fs::read(corpus_dir().join(format!("{}.txt", h[0]))).unwrap();
        assert_eq!(std::fs::read(r.join("@buscar").join(q).join(format!("{}.txt", h[0]))).unwrap(), orig);
    }
    // Nombres inexistentes: ENOENT.
    assert_eq!(std::fs::metadata(r.join("@similar/no-existe")).unwrap_err().raw_os_error(), Some(libc::ENOENT));
    assert_eq!(std::fs::metadata(r.join("@doc/no-existe")).unwrap_err().raw_os_error(), Some(libc::ENOENT));

    // cd @doc/<x> salta a la vista de x aunque x no sea visible desde la raíz.
    let raiz: Vec<String> = proyeccion::vista(&c, &[], Params::default()).entradas.iter().map(|e| c.docs[e.doc as usize].nombre.clone()).collect();
    let oculto = c.docs.iter().find(|d| !raiz.contains(&d.nombre)).unwrap();
    assert!(std::fs::metadata(r.join(&oculto.nombre)).is_err());
    let dentro = txt(&ls(&r.join("@doc").join(&oculto.nombre)));
    let j = c.indice(&oculto.nombre).unwrap();
    let esperado: Vec<String> =
        proyeccion::vista(&c, &[j], Params::default()).entradas.iter().map(|e| c.docs[e.doc as usize].nombre.clone()).collect();
    assert_eq!(dentro[..esperado.len()], esperado[..]);
    let foco = std::fs::read_to_string(r.join("@doc").join(&oculto.nombre).join(".foco")).unwrap();
    assert!(foco.contains(&format!("ruta: /{}", oculto.nombre)));
    // Escribir sigue prohibido.
    assert_eq!(std::fs::write(r.join("@buscar/x/nuevo.txt"), b"x").unwrap_err().raw_os_error(), Some(libc::EROFS));
}

/// Con k = 1 el grafo de vecinos deja documentos inalcanzables: la cobertura los
/// cuelga y se alcanzan TODOS con `ls`/`cd` reales recorriendo el árbol.
#[test]
fn alcance_completo_montado_con_k_chico() {
    let c = Corpus::desde_directorio(&corpus_dir()).unwrap();
    let p = Params { k: 1, ..Params::default() };
    let ib = IndiceBola::new(&c);
    let cob = Cobertura::new(&c, p, &ib);
    assert!(cob.agregados > 0, "con k=1 tiene que haber inalcanzables sin cobertura");
    assert_eq!(cob.alcanzables(), c.docs.len());
    let m = Montaje::nuevo("alcance", &["--k", "1"]);
    for (j, d) in c.docs.iter().enumerate() {
        let mut dir = m.punto.clone();
        for x in cob.ruta_hasta(j as u32) {
            dir = dir.join(&c.docs[x as usize].nombre);
        }
        let l = ls(&dir);
        assert!(l.contains(&d.nombre) && l.contains(&format!("{}.txt", d.nombre)), "{} no aparece en {}", d.nombre, dir.display());
        assert!(std::fs::metadata(dir.join(&d.nombre)).unwrap().is_dir());
    }
}
