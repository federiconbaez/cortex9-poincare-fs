//! Prueba de integración: monta el FS de verdad con FUSE, usa `cd`/`ls`/`cat`
//! desde un shell real, compara con el cálculo sin FUSE y desmonta.

use poincare_fs::bola;
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
    let p = p.to_str().unwrap();
    m.lines().any(|l| l.split(' ').nth(1) == Some(p))
}

struct Montaje {
    punto: PathBuf,
    log: PathBuf,
    hijo: Option<Child>,
}

impl Montaje {
    fn nuevo(etiqueta: &str, modo_io: &str) -> Montaje {
        let base = std::env::temp_dir().join(format!("poincare-test-{}-{etiqueta}", std::process::id()));
        let punto = base.join("mnt");
        std::fs::create_dir_all(&punto).unwrap();
        let log = base.join("stderr.log");
        let hijo = Command::new(BIN)
            .arg("montar")
            .arg(corpus_dir())
            .arg(&punto)
            .env("POINCARE_IO", modo_io)
            .stderr(std::fs::File::create(&log).unwrap())
            .stdout(Stdio::null())
            .spawn()
            .expect("lanzar poincare-fs");
        let t = Instant::now();
        while !montado(&punto) {
            assert!(t.elapsed() < Duration::from_secs(30), "no se montó en 30 s");
            std::thread::sleep(Duration::from_millis(20));
        }
        Montaje { punto, log, hijo: Some(hijo) }
    }

    fn log(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    fn desmontar(&mut self) -> std::process::ExitStatus {
        let st = Command::new("fusermount3").arg("-u").arg(&self.punto).status().unwrap();
        assert!(st.success(), "fusermount3 -u falló");
        let mut h = self.hijo.take().unwrap();
        let t = Instant::now();
        loop {
            if let Some(st) = h.try_wait().unwrap() {
                return st;
            }
            assert!(t.elapsed() < Duration::from_secs(10), "el proceso no terminó tras desmontar");
            std::thread::sleep(Duration::from_millis(20));
        }
    }
}

impl Drop for Montaje {
    fn drop(&mut self) {
        if let Some(mut h) = self.hijo.take() {
            let _ = Command::new("fusermount3").arg("-u").arg("-z").arg(&self.punto).status();
            let _ = h.kill();
            let _ = h.wait();
        }
        let _ = std::fs::remove_dir_all(self.punto.parent().unwrap());
    }
}

fn sh(script: &str) -> (bool, String, String) {
    let o = Command::new("sh").arg("-c").arg(script).output().unwrap();
    (
        o.status.success(),
        String::from_utf8_lossy(&o.stdout).into_owned(),
        String::from_utf8_lossy(&o.stderr).into_owned(),
    )
}

fn idx(c: &Corpus, ruta: &[&str]) -> Vec<u32> {
    ruta.iter().map(|n| c.indice(n).unwrap()).collect()
}

fn visibles(c: &Corpus, ruta: &[&str]) -> Vec<String> {
    proyeccion::vista(c, &idx(c, ruta), Params::default())
        .entradas
        .iter()
        .map(|e| c.docs[e.doc as usize].nombre.clone())
        .collect()
}

fn nombres_esperados(c: &Corpus, ruta: &[&str]) -> Vec<String> {
    let mut out = vec![".foco".to_string(), ".vista".to_string()];
    for n in visibles(c, ruta) {
        out.push(n.clone());
        out.push(format!("{n}.txt"));
    }
    out
}

#[test]
fn montar_cd_ls_cat_y_desmontar() {
    let corpus = Corpus::desde_directorio(&corpus_dir()).unwrap();
    let mut m = Montaje::nuevo("principal", "auto");
    let raiz = m.punto.clone();
    let r = raiz.display();
    assert!(m.log().contains("E/S=Some(Uring)"), "se esperaba io_uring en esta máquina: {}", m.log());

    // Camino elegido con el cálculo sin FUSE: primer visible de la raíz, luego
    // el primer visible desde ahí, y un archivo visible desde el final.
    let a = visibles(&corpus, &[])[0].clone();
    let b = visibles(&corpus, &[&a])[0].clone();
    let archivo = visibles(&corpus, &[&a, &b])[0].clone();

    // 1. readdir de la raíz: mismo contenido y MISMO ORDEN que el cálculo sin FUSE.
    let (ok, out, err) = sh(&format!("cd {r} && ls -U -A"));
    assert!(ok, "{err}");
    let vistos: Vec<String> = out.lines().map(String::from).collect();
    assert_eq!(vistos, nombres_esperados(&corpus, &[]));

    // 2. Sesión real de shell: cd en dos pasos, ls, cat.
    let (ok, out, err) = sh(&format!("cd {r} && cd {a} && cd {b} && pwd && ls -U && cat {archivo}.txt && cat .foco"));
    assert!(ok, "{err}");
    assert!(out.contains(&format!("{r}/{a}/{b}")));
    let esperado = std::fs::read_to_string(corpus_dir().join(format!("{archivo}.txt"))).unwrap();
    assert!(out.contains(esperado.trim()));
    assert!(out.contains(&format!("ruta: /{a}/{b}")));
    assert!(out.contains("dimension_bola: 16"));
    // El foco quedó en el origen y R sigue ortogonal.
    let campo = |clave: &str| -> f64 {
        out.lines().find(|l| l.starts_with(clave)).unwrap().rsplit(' ').next().unwrap().parse().unwrap()
    };
    assert!(campo("foco_imagen_norma:") < 1e-12);
    assert!(campo("R_error_ortogonal:") < 1e-12);

    // 3. El listado tras los cd coincide (orden incluido) con el cálculo sin FUSE.
    let (ok, out, _) = sh(&format!("cd {r}/{a}/{b} && ls -U -A"));
    assert!(ok);
    let vistos: Vec<String> = out.lines().map(String::from).collect();
    assert_eq!(vistos, nombres_esperados(&corpus, &[&a, &b]));

    // 4. `.vista` montada == texto calculado sin FUSE (byte a byte) y determinista.
    let ruta = idx(&corpus, &[&a, &b]);
    let offline = proyeccion::texto_vista(&corpus, &proyeccion::vista(&corpus, &ruta, Params::default()));
    let p_vista = raiz.join(format!("{a}/{b}/.vista"));
    let v1 = std::fs::read_to_string(&p_vista).unwrap();
    let v2 = std::fs::read_to_string(&p_vista).unwrap();
    assert_eq!(v1, offline);
    assert_eq!(v1, v2);

    // 5. La columna distancia_al_foco es la distancia hiperbólica original en la
    //    bola (Möbius es isometría). Se imprime con 9 decimales: tolerancia 1e-8.
    let foco = &corpus.docs[corpus.indice(&b).unwrap() as usize];
    for l in v1.lines().filter(|l| !l.starts_with('#')) {
        let campos: Vec<&str> = l.split_whitespace().collect();
        let d: f64 = campos[2].parse().unwrap();
        let nombre = campos.last().unwrap();
        let doc = &corpus.docs[corpus.indice(nombre).unwrap() as usize];
        assert!((bola::distancia(&foco.x, &doc.x) - d).abs() < 1e-8, "{nombre}");
    }

    // 6. Lo que no está en la proyección no existe: no se puede hacer cd.
    let no_visible = corpus
        .docs
        .iter()
        .map(|d| d.nombre.clone())
        .find(|n| !visibles(&corpus, &[]).contains(n))
        .expect("con k=10 y 18 docs hay alguno invisible desde la raíz");
    let (ok, _, _) = sh(&format!("cd {r}/{no_visible}"));
    assert!(!ok, "cd a {no_visible} debería fallar desde la raíz");
    assert_eq!(
        std::fs::metadata(raiz.join(&no_visible)).unwrap_err().raw_os_error(),
        Some(libc::ENOENT)
    );

    // 7. Solo lectura.
    let e = std::fs::write(raiz.join("nuevo.txt"), b"x").unwrap_err();
    assert_eq!(e.raw_os_error(), Some(libc::EROFS));

    // 8. Contenido leído por partes (offsets) igual al original.
    let original = std::fs::read(corpus_dir().join(format!("{a}.txt"))).unwrap();
    let (ok, out, _) = sh(&format!("dd if={r}/{a}.txt bs=7 status=none"));
    assert!(ok);
    assert_eq!(out.as_bytes(), &original[..]);

    // 9. Desmontar limpio: el proceso termina con éxito y el punto ya no está montado.
    let st = m.desmontar();
    assert!(st.success(), "salida del proceso: {st:?}");
    assert!(!montado(&raiz));
    assert!(m.log().contains("desmontado"));
}

/// El camino completo importa: mismo foco final por dos caminos distintos.
#[test]
fn el_camino_importa_montado() {
    let corpus = Corpus::desde_directorio(&corpus_dir()).unwrap();
    // buscar b alcanzable desde dos primeros pasos distintos a1 != a2
    let raiz = visibles(&corpus, &[]);
    let mut caso = None;
    'buscar: for a1 in &raiz {
        for a2 in &raiz {
            if a1 == a2 {
                continue;
            }
            for b in visibles(&corpus, &[a1]) {
                if &b != a2 && visibles(&corpus, &[a2]).contains(&b) {
                    caso = Some((a1.clone(), a2.clone(), b));
                    break 'buscar;
                }
            }
        }
    }
    let (a1, a2, b) = caso.expect("hay un foco alcanzable por dos caminos");
    let mut m = Montaje::nuevo("camino", "auto");
    let r = m.punto.display().to_string();
    let v1 = std::fs::read_to_string(format!("{r}/{a1}/{b}/.vista")).unwrap();
    let v2 = std::fs::read_to_string(format!("{r}/{a2}/{b}/.vista")).unwrap();
    assert_ne!(v1, v2, "con memoria > 0 el camino debe cambiar la vista");
    // ...pero la distancia al foco de cada documento común es la misma (isometría).
    let tabla = |v: &str| -> std::collections::HashMap<String, (f64, f64)> {
        v.lines()
            .filter(|l| !l.starts_with('#'))
            .map(|l| {
                let c: Vec<&str> = l.split_whitespace().collect();
                (c.last().unwrap().to_string(), (c[1].parse().unwrap(), c[2].parse().unwrap()))
            })
            .collect()
    };
    let (t1, t2) = (tabla(&v1), tabla(&v2));
    let mut comunes = 0;
    let mut puntajes_distintos = 0;
    for (n, (p1, d1)) in &t1 {
        if let Some((p2, d2)) = t2.get(n) {
            comunes += 1;
            assert!((d1 - d2).abs() < 1e-8, "{n}: {d1} vs {d2}");
            if (p1 - p2).abs() > 1e-6 {
                puntajes_distintos += 1;
            }
        }
    }
    assert!(comunes > 0 && puntajes_distintos > 0);
    let f1 = std::fs::read_to_string(format!("{r}/{a1}/{b}/.foco")).unwrap();
    assert!(f1.contains(&format!("ruta: /{a1}/{b}")) && f1.contains("trayectoria"));

    // Límite de profundidad (MAX_PROFUNDIDAD = 64): 64 cd funcionan, el 65 no.
    // Par (x, y) que se ven mutuamente en la caminata alternada (calculado sin FUSE).
    let mut par = None;
    'par: for x in &raiz {
        for y in visibles(&corpus, &[x]) {
            let mut ruta: Vec<String> = Vec::new();
            let mut ok = true;
            for i in 0..65 {
                let sig = if i % 2 == 0 { x.clone() } else { y.clone() };
                let rr: Vec<&str> = ruta.iter().map(|s| s.as_str()).collect();
                if !visibles(&corpus, &rr).contains(&sig) {
                    ok = false;
                    break;
                }
                ruta.push(sig);
                if ruta.len() == 64 {
                    break;
                }
            }
            if ok {
                par = Some((x.clone(), y));
                break 'par;
            }
        }
    }
    let (x, y) = par.expect("hay un par que se ve mutuamente");
    let mut ruta = String::new();
    for i in 0..64 {
        ruta.push('/');
        ruta.push_str(if i % 2 == 0 { &x } else { &y });
    }
    let (ok, out, err) = sh(&format!("cd {r}{ruta} && cat .foco"));
    assert!(ok, "{err}");
    assert!(out.contains("profundidad: 64"));
    let (ok, _, _) = sh(&format!("cd {r}{ruta}/{x}"));
    assert!(!ok, "el cd 65 debe fallar");
    assert!(m.desmontar().success());
}

/// El mismo FS montado leyendo el corpus por pread da exactamente lo mismo.
#[test]
fn montaje_con_pread_es_identico() {
    let mut m = Montaje::nuevo("pread", "pread");
    assert!(m.log().contains("E/S=Some(Pread)"), "{}", m.log());
    let r = m.punto.display().to_string();
    let (ok, a, _) = sh(&format!("cd {r} && ls -U -A && cat .vista && cat *.txt"));
    assert!(ok);
    let corpus = Corpus::desde_directorio(&corpus_dir()).unwrap();
    let mut esperado = nombres_esperados(&corpus, &[]).join("\n");
    esperado.push('\n');
    assert!(a.starts_with(&esperado));
    assert!(a.contains(&proyeccion::texto_vista(&corpus, &proyeccion::vista(&corpus, &[], Params::default()))));
    assert!(m.desmontar().success());
}
