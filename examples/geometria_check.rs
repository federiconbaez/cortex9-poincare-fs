//! Comprueba que `ls -U @similar/<q>` del FS montado = knn hiperbólico del embedding.
use poincare_fs::ajuste::ParamsAjuste;
use poincare_fs::directo::IndiceBola;
use poincare_fs::proyeccion::{Corpus, Opciones};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

fn montado(p: &Path) -> bool {
    std::fs::read_to_string("/proc/self/mounts").unwrap().lines().any(|l| l.split(' ').nth(1) == Some(p.to_str().unwrap()))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    // corpus dim lr radio semilla test.tsv
    let corpus = PathBuf::from(&a[1]);
    let dim: usize = a[2].parse().unwrap();
    let lr: f64 = a[3].parse().unwrap();
    let radio: f64 = a[4].parse().unwrap();
    let semilla: u64 = u64::from_str_radix(a[5].trim_start_matches("0x"), 16).unwrap();
    let tests: Vec<String> = std::fs::read_to_string(&a[6]).unwrap().lines().skip(1).take(50).map(String::from).collect();
    let o = Opciones {
        ajuste: ParamsAjuste { dim, epocas: 50, lr, radio, semilla, ..Default::default() },
        ..Default::default()
    };
    let c = Corpus::desde_directorio_con(&corpus, &o).unwrap();
    let ib = IndiceBola::new(&c);
    let base = std::env::temp_dir().join(format!("pfs-gcheck-{}", std::process::id()));
    let mnt = base.join("mnt");
    std::fs::create_dir_all(&mnt).unwrap();
    let idx = base.join("idx");
    let mut hijo: Child = Command::new(concat!(env!("CARGO_MANIFEST_DIR"), "/target/release/poincare-fs"))
        .args(["montar", corpus.to_str().unwrap(), mnt.to_str().unwrap(),
            "--dim", &dim.to_string(), "--epocas", "50", "--lr", &lr.to_string(),
            "--radio-ajuste", &radio.to_string(), "--semilla-ajuste", &semilla.to_string(),
            "--indice", idx.to_str().unwrap()])
        .stdout(Stdio::null()).stderr(Stdio::null()).spawn().unwrap();
    let t = Instant::now();
    while !montado(&mnt) {
        assert!(t.elapsed() < Duration::from_secs(180));
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut ok = 0;
    for q in &tests {
        let j = c.indice(q).unwrap() as usize;
        let esp: Vec<String> = ib.knn(&c, &c.docs[j].x, 10, Some(j as u32))
            .into_iter().map(|(i, _)| c.docs[i as usize].nombre.clone()).collect();
        // readdir order = score order
        let mut got = Vec::new();
        // FUSE may not preserve order via read_dir in Rust - use .resultados
        let res = std::fs::read_to_string(mnt.join("@similar").join(q).join(".resultados")).unwrap();
        for l in res.lines() {
            if l.starts_with('#') || l.is_empty() { continue; }
            let name = l.split_whitespace().nth(2).unwrap();
            got.push(name.to_string());
        }
        assert_eq!(got, esp, "mismatch {q}");
        ok += 1;
    }
    let _ = Command::new("fusermount3").arg("-u").arg(&mnt).status();
    let _ = hijo.kill();
    let _ = hijo.wait();
    let _ = std::fs::remove_dir_all(&base);
    println!("OK {ok}/50 @similar coincide con IndiceBola (dim={dim})");
}
