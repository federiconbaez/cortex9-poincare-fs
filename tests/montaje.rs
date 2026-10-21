//! Prueba de integración: monta el FS de verdad con FUSE, usa `cd`/`ls`/`cat`
//! desde un shell real, compara con el cálculo sin FUSE y desmonta.

use poincare_fs::hiper::distancia;
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
    hijo: Option<Child>,
}

impl Montaje {
    fn nuevo(etiqueta: &str) -> Montaje {
        let punto = std::env::temp_dir().join(format!("poincare-test-{}-{etiqueta}", std::process::id()));
        std::fs::create_dir_all(&punto).unwrap();
        let hijo = Command::new(BIN)
            .arg("montar")
            .arg(corpus_dir())
            .arg(&punto)
            .stderr(Stdio::inherit())
            .spawn()
            .expect("lanzar poincare-fs");
        let t = Instant::now();
        while !montado(&punto) {
            assert!(t.elapsed() < Duration::from_secs(10), "no se montó en 10 s");
            std::thread::sleep(Duration::from_millis(20));
        }
        Montaje { punto, hijo: Some(hijo) }
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
        let _ = std::fs::remove_dir(&self.punto);
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

fn nombres_esperados(c: &Corpus, ruta: &[&str]) -> Vec<String> {
    let idx: Vec<u32> = ruta.iter().map(|n| c.indice(n).unwrap()).collect();
    let v = proyeccion::vista(c, &idx, Params::default());
    let mut out = vec![".foco".to_string(), ".vista".to_string()];
    for e in &v.entradas {
        let n = &c.docs[e.doc as usize].nombre;
        out.push(n.clone());
        out.push(format!("{n}.txt"));
    }
    out
}

#[test]
fn montar_cd_ls_cat_y_desmontar() {
    let corpus = Corpus::desde_directorio(&corpus_dir()).unwrap();
    let mut m = Montaje::nuevo("principal");
    let raiz = m.punto.clone();
    let r = raiz.display();

    // 1. readdir de la raíz: mismo contenido y MISMO ORDEN que el cálculo sin FUSE.
    let (ok, out, err) = sh(&format!("cd {r} && ls -U -A"));
    assert!(ok, "{err}");
    let vistos: Vec<String> = out.lines().map(String::from).collect();
    assert_eq!(vistos, nombres_esperados(&corpus, &[]));

    // 2. Sesión real de shell: cd en dos pasos, ls, cat.
    let (ok, out, err) = sh(&format!(
        "cd {r} && cd musica-folklore && cd cocina-empanadas && pwd && ls -U && cat cocina-pan.txt && cat .foco"
    ));
    assert!(ok, "{err}");
    assert!(out.contains(&format!("{r}/musica-folklore/cocina-empanadas")));
    let esperado_pan = std::fs::read_to_string(corpus_dir().join("cocina-pan.txt")).unwrap();
    assert!(out.contains(esperado_pan.trim()));
    assert!(out.contains("ruta: /musica-folklore/cocina-empanadas"));
    assert!(out.contains("det: 1.000000000000000"));
    // El foco quedó en el origen: |imagen| < 1e-12.
    let linea = out.lines().find(|l| l.starts_with("foco_imagen:")).unwrap();
    let mag: f64 = linea.rsplit("= ").next().unwrap().trim().parse().unwrap();
    assert!(mag < 1e-12, "|T(foco)| = {mag}");

    // 3. El listado tras los cd coincide (orden incluido) con el cálculo sin FUSE.
    let (ok, out, _) = sh(&format!("cd {r}/musica-folklore/cocina-empanadas && ls -U -A"));
    assert!(ok);
    let vistos: Vec<String> = out.lines().map(String::from).collect();
    assert_eq!(vistos, nombres_esperados(&corpus, &["musica-folklore", "cocina-empanadas"]));

    // 4. `.vista` montada == texto calculado sin FUSE (byte a byte) y determinista.
    let ruta = ["musica-folklore", "cocina-empanadas"];
    let idx: Vec<u32> = ruta.iter().map(|n| corpus.indice(n).unwrap()).collect();
    let offline = proyeccion::texto_vista(&corpus, &proyeccion::vista(&corpus, &idx, Params::default()));
    let p_vista = raiz.join("musica-folklore/cocina-empanadas/.vista");
    let a = std::fs::read_to_string(&p_vista).unwrap();
    let b = std::fs::read_to_string(&p_vista).unwrap();
    assert_eq!(a, offline);
    assert_eq!(a, b);

    // 5. Las distancias de la vista son las hiperbólicas originales desde el foco
    //    (Möbius es isometría). Se imprimen con 9 decimales: tolerancia 1e-8.
    let foco = &corpus.docs[corpus.indice("cocina-empanadas").unwrap() as usize];
    for l in a.lines().filter(|l| !l.starts_with('#')) {
        let campos: Vec<&str> = l.split_whitespace().collect();
        let d: f64 = campos[1].parse().unwrap();
        let nombre = campos.last().unwrap();
        let doc = &corpus.docs[corpus.indice(nombre).unwrap() as usize];
        assert!((distancia(foco.z, doc.z) - d).abs() < 1e-8, "{nombre}");
    }

    // 6. Lo que no está en la proyección no existe: no se puede hacer cd.
    let no_visible = corpus
        .docs
        .iter()
        .map(|d| d.nombre.clone())
        .find(|n| !nombres_esperados(&corpus, &[]).contains(n))
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
    let original = std::fs::read(corpus_dir().join("rust-ownership.txt")).unwrap();
    let (ok, out, _) = sh(&format!("dd if={r}/rust-ownership.txt bs=7 status=none"));
    assert!(ok);
    assert_eq!(out.as_bytes(), &original[..]);

    // 9. Desmontar limpio: el proceso termina con éxito y el punto ya no está montado.
    let st = m.desmontar();
    assert!(st.success(), "salida del proceso: {st:?}");
    assert!(!montado(&raiz));
}

#[test]
fn rutas_largas_y_ciclos() {
    let mut m = Montaje::nuevo("ciclos");
    let r = m.punto.display().to_string();
    // ida y vuelta: a -> b -> a es una ruta válida y distinta de /a (otra transformación).
    let (ok, out, err) = sh(&format!("cd {r}/musica-tango/musica-jazz/musica-tango && cat .foco && ls -U -A"));
    assert!(ok, "{err}");
    assert!(out.contains("ruta: /musica-tango/musica-jazz/musica-tango"));
    // El conjunto y orden visible desde /a/b/a es el mismo que desde /a (isometría),
    // aunque la transformación acumulada (rotación) puede diferir.
    let desde_a = sh(&format!("cd {r}/musica-tango && ls -U -A")).1;
    let listado: Vec<&str> = out.lines().skip_while(|l| !l.starts_with(".foco")).collect();
    assert_eq!(listado, desde_a.lines().collect::<Vec<_>>());
    assert!(listado.len() > 2);

    // Límite de profundidad (MAX_PROFUNDIDAD = 64): 64 cd funcionan, el 65 no.
    let mut ruta = String::new();
    for i in 0..64 {
        ruta.push_str(if i % 2 == 0 { "/musica-tango" } else { "/musica-jazz" });
    }
    let (ok, out, err) = sh(&format!("cd {r}{ruta} && cat .foco"));
    assert!(ok, "{err}");
    assert!(out.contains("profundidad: 64"));
    let (ok, _, _) = sh(&format!("cd {r}{ruta}/musica-tango"));
    assert!(!ok, "el cd 65 debe fallar");
    assert!(m.desmontar().success());
}
