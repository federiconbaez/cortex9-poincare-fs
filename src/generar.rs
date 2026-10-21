//! Generadores de corpus grandes a partir de texto local (sin red, sin modelos).
//!
//! * `stdlib`: texto real — comentarios y docstrings de la biblioteca estándar
//!   de Python instalada (`/usr/lib/python3.*`), en trozos de `PALABRAS`
//!   palabras. Tema = paquete (`asyncio`, `email`, `xml`, …).
//! * `sintetico`: documentos muestreados (semilla fija) de la distribución de
//!   palabras de cada paquete (70 %) mezclada con la global (30 %). Sirve para
//!   llegar a 10 000 documentos; el texto no es prosa real.

use crate::ajuste::Rng;
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};

pub const PALABRAS: usize = 80;
const EXCLUIDOS: [&str; 9] =
    ["encodings", "test", "pydoc_data", "__phello__", "idlelib", "ensurepip", "site-packages", "lib-dynload", "__pycache__"];

/// Busca `/usr/lib/python3.*` (el de mayor versión).
pub fn python_por_defecto() -> Option<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir("/usr/lib")
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("python3.") && n[8..].chars().all(|c| c.is_ascii_digit()))
        })
        .filter(|p| p.join("asyncio").is_dir())
        .collect();
    v.sort();
    v.pop()
}

/// Texto en prosa de un archivo Python: comentarios `#` y cadenas triples.
pub fn prosa_python(fuente: &str) -> String {
    let mut out = String::new();
    let mut en_triple: Option<&str> = None;
    for linea in fuente.lines() {
        let mut resto = linea;
        loop {
            match en_triple {
                Some(q) => match resto.find(q) {
                    Some(p) => {
                        out.push_str(&resto[..p]);
                        out.push(' ');
                        resto = &resto[p + 3..];
                        en_triple = None;
                    }
                    None => {
                        out.push_str(resto);
                        out.push(' ');
                        break;
                    }
                },
                None => {
                    let pd = resto.find("\"\"\"");
                    let ps = resto.find("'''");
                    let ph = resto.find('#');
                    let triple = match (pd, ps) {
                        (Some(a), Some(b)) => Some(if a < b { (a, "\"\"\"") } else { (b, "'''") }),
                        (Some(a), None) => Some((a, "\"\"\"")),
                        (None, Some(b)) => Some((b, "'''")),
                        (None, None) => None,
                    };
                    match (triple, ph) {
                        (Some((p, q)), h) if h.is_none_or(|h| p < h) => {
                            resto = &resto[p + 3..];
                            en_triple = Some(q);
                        }
                        (_, Some(h)) => {
                            out.push_str(&resto[h + 1..]);
                            out.push(' ');
                            break;
                        }
                        _ => break,
                    }
                }
            }
        }
    }
    out
}

fn palabras(texto: &str) -> Vec<String> {
    texto
        .split(|c: char| !c.is_alphabetic())
        .filter(|w| w.chars().count() >= 2)
        .map(|w| w.to_lowercase())
        .collect()
}

fn sanear(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()
}

/// Todos los trozos (nombre, texto) del stdlib, ordenados por nombre.
pub fn trozos_stdlib(raiz: &Path) -> io::Result<Vec<(String, String)>> {
    let mut paquetes: Vec<PathBuf> = std::fs::read_dir(raiz)?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !EXCLUIDOS.contains(&n) && !n.starts_with("config-") && !n.starts_with('_')
        })
        .collect();
    paquetes.sort();
    let mut out = Vec::new();
    for pkg in paquetes {
        let tema = sanear(pkg.file_name().and_then(|n| n.to_str()).unwrap_or("x")).replace('-', "_");
        let mut archivos = Vec::new();
        let mut pila = vec![pkg.clone()];
        while let Some(d) = pila.pop() {
            for e in std::fs::read_dir(&d)?.filter_map(|e| e.ok()) {
                let p = e.path();
                let ft = e.file_type()?;
                if ft.is_dir() && p.file_name().is_some_and(|n| n != "__pycache__") {
                    pila.push(p);
                } else if ft.is_file() && p.extension().is_some_and(|x| x == "py") {
                    archivos.push(p);
                }
            }
        }
        archivos.sort();
        for a in archivos {
            let Ok(fuente) = std::fs::read_to_string(&a) else { continue };
            let rel = a.strip_prefix(&pkg).unwrap_or(&a).with_extension("");
            let modulo = sanear(&rel.to_string_lossy());
            let ws = palabras(&prosa_python(&fuente));
            for (k, c) in ws.chunks(PALABRAS).enumerate() {
                if c.len() < PALABRAS / 2 {
                    continue;
                }
                out.push((format!("{tema}-{modulo}-{k:03}"), c.join(" ")));
            }
        }
    }
    out.sort();
    Ok(out)
}

/// Elige `n` elementos repartidos uniformemente (determinista).
fn repartir<T: Clone>(v: &[T], n: usize) -> Vec<T> {
    if v.len() <= n {
        return v.to_vec();
    }
    (0..n).map(|i| v[i * v.len() / n].clone()).collect()
}

pub fn generar_stdlib(raiz: &Path, n: usize) -> io::Result<Vec<(String, String)>> {
    Ok(repartir(&trozos_stdlib(raiz)?, n))
}

pub fn generar_sintetico(raiz: &Path, n: usize, semilla: u64) -> io::Result<Vec<(String, String)>> {
    let trozos = trozos_stdlib(raiz)?;
    let mut por_tema: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut global = Vec::new();
    for (nombre, texto) in &trozos {
        let tema = nombre.split('-').next().unwrap_or("").to_string();
        let ws = palabras(texto);
        global.extend(ws.iter().cloned());
        por_tema.entry(tema).or_default().extend(ws);
    }
    let temas: Vec<(&String, &Vec<String>)> = por_tema.iter().filter(|(_, w)| w.len() >= 500).collect();
    if temas.is_empty() || global.is_empty() {
        return Err(io::Error::new(io::ErrorKind::NotFound, "no hay texto suficiente"));
    }
    let mut rng = Rng::new(semilla);
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let (tema, ws) = temas[i % temas.len()];
        let doc: Vec<&str> = (0..PALABRAS)
            .map(|_| {
                if rng.uniforme() < 0.7 {
                    ws[rng.indice(ws.len())].as_str()
                } else {
                    global[rng.indice(global.len())].as_str()
                }
            })
            .collect();
        out.push((format!("{tema}-s{i:05}"), doc.join(" ")));
    }
    Ok(out)
}

/// Escribe los documentos como `<nombre>.txt` en `dir` (que debe no existir o estar vacío).
pub fn escribir(dir: &Path, docs: &[(String, String)]) -> io::Result<()> {
    std::fs::create_dir_all(dir)?;
    if std::fs::read_dir(dir)?.next().is_some() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, "el directorio destino no está vacío"));
    }
    for (n, t) in docs {
        std::fs::write(dir.join(format!("{n}.txt")), t)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extrae_comentarios_y_docstrings() {
        let f = "import os  # comentario uno\n\"\"\"Doc de\nvarias líneas\"\"\"\nx = '#no' \ndef f():\n    '''otra'''\n";
        let p = prosa_python(f);
        assert!(p.contains("comentario uno"));
        assert!(p.contains("Doc de"));
        assert!(p.contains("varias líneas"));
        assert!(p.contains("otra"));
        assert!(!p.contains("import"));
    }

    #[test]
    fn repartir_es_uniforme_y_determinista() {
        let v: Vec<u32> = (0..100).collect();
        assert_eq!(repartir(&v, 4), vec![0, 25, 50, 75]);
        assert_eq!(repartir(&v, 1000).len(), 100);
    }
}
