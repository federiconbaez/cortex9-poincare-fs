//! Adaptador FUSE (crate `fuser`, solo lectura).
//!
//! Inodos:
//! * directorio con ranura `s` (una ruta de `cd`): `4s + 1` (la raíz es 1);
//! * `.foco` de ese directorio: `4s + 2`; `.vista`: `4s + 3`;
//! * contenido del documento `j` (`<nombre>.txt`): `BASE_DOC + j`;
//! * directorios mágicos (v0.3, se resuelven con `lookup` desde cualquier
//!   directorio pero no se listan): `@doc` (`cd @doc/<nombre>` salta a la
//!   vista de ese documento), `@similar` (`ls @similar/<nombre>`), `@buscar`
//!   (`ls "@buscar/<consulta>"`, híbrido léxico → bola) y `@buscar-bola`
//!   (solo bola) y `@similar-hibrido` (`ls @similar-hibrido/<nombre>`): `BASE_ESP + m`;
//! * directorio de una consulta de ranura `q`: `BASE_CONS + 2q`; su
//!   `.resultados`: `BASE_CONS + 2q + 1`.
//! * Cobertura: la vista de un foco incluye además a sus hijos del árbol de
//!   alcance que no estaban entre los k (ver `directo::Cobertura`).

use crate::bola;
use crate::directo::{self, Cobertura, IndiceBola, IndiceLexico, Modo};
use crate::proyeccion::{self, Corpus, Params, Vista, MAX_PROFUNDIDAD};
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request,
};
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

pub const BASE_DOC: u64 = 1 << 60;
pub const MAX_RANURAS: usize = 1 << 20;
pub const BASE_ESP: u64 = 1 << 59;
pub const BASE_CONS: u64 = 1 << 58;
pub const MAX_CONSULTAS: usize = 1 << 20;
pub const MAGICOS: [&str; 5] = ["@doc", "@similar", "@buscar", "@buscar-bola", "@similar-hibrido"];
const TTL: Duration = Duration::from_secs(1);
/// Las entradas mágicas y de consulta son inmutables (corpus de solo lectura): TTL largo.
const TTL_DIRECTO: Duration = Duration::from_secs(3600);

enum Nodo {
    Dir(u64),
    Foco(u64),
    Vista(u64),
    Doc(u32),
    /// 0 = @doc; 1..=4 = @similar, @buscar, @buscar-bola, @similar-hibrido.
    Esp(u64),
    Consulta(u64),
    Resultados(u64),
}

fn modo_esp(m: u64) -> Option<Modo> {
    match m {
        1 => Some(Modo::Similar),
        2 => Some(Modo::Hibrido),
        3 => Some(Modo::Bola),
        4 => Some(Modo::SimilarHibrido),
        _ => None,
    }
}

/// Caché LRU de vistas por ranura (las vistas son deterministas: nunca se invalidan).
pub struct Lru {
    cap: usize,
    reloj: u64,
    mapa: HashMap<u64, (Rc<Vista>, u64)>,
    orden: BTreeMap<u64, u64>,
    pub aciertos: u64,
    pub fallos: u64,
}

impl Lru {
    pub fn new(cap: usize) -> Lru {
        Lru { cap, reloj: 0, mapa: HashMap::new(), orden: BTreeMap::new(), aciertos: 0, fallos: 0 }
    }
    pub fn len(&self) -> usize {
        self.mapa.len()
    }
    pub fn is_empty(&self) -> bool {
        self.mapa.is_empty()
    }
    pub fn obtener(&mut self, s: u64) -> Option<Rc<Vista>> {
        self.reloj += 1;
        let reloj = self.reloj;
        match self.mapa.get_mut(&s) {
            Some((v, t)) => {
                self.orden.remove(t);
                *t = reloj;
                self.orden.insert(reloj, s);
                self.aciertos += 1;
                Some(v.clone())
            }
            None => {
                self.fallos += 1;
                None
            }
        }
    }
    pub fn insertar(&mut self, s: u64, v: Rc<Vista>) {
        if self.cap == 0 {
            return;
        }
        self.reloj += 1;
        if let Some((_, t)) = self.mapa.remove(&s) {
            self.orden.remove(&t);
        }
        while self.mapa.len() >= self.cap {
            let Some((&t, &viejo)) = self.orden.iter().next() else { break };
            self.orden.remove(&t);
            self.mapa.remove(&viejo);
        }
        self.mapa.insert(s, (v, self.reloj));
        self.orden.insert(self.reloj, s);
    }
}

pub struct PoincareFs {
    corpus: Corpus,
    params: Params,
    rutas: Vec<Vec<u32>>,
    ranura_de: HashMap<Vec<u32>, u64>,
    uid: u32,
    gid: u32,
    creado: SystemTime,
    pub cache: Lru,
    pub indice_bola: IndiceBola,
    pub lexico: IndiceLexico,
    pub cobertura: Cobertura,
    consultas: Vec<(Modo, String)>,
    consulta_de: HashMap<(Modo, String), u64>,
}

impl PoincareFs {
    pub fn new(corpus: Corpus, params: Params) -> Self {
        PoincareFs::con_cache(corpus, params, 4096)
    }

    pub fn con_cache(corpus: Corpus, params: Params, cache: usize) -> Self {
        let mut ranura_de = HashMap::new();
        ranura_de.insert(Vec::new(), 0);
        // SAFETY: getuid/getgid no fallan.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        let indice_bola = IndiceBola::new(&corpus);
        let lexico = IndiceLexico::new(&corpus);
        let cobertura = Cobertura::new(&corpus, params, &indice_bola);
        PoincareFs {
            corpus,
            params,
            rutas: vec![Vec::new()],
            ranura_de,
            uid,
            gid,
            creado: SystemTime::now(),
            cache: Lru::new(cache),
            indice_bola,
            lexico,
            cobertura,
            consultas: Vec::new(),
            consulta_de: HashMap::new(),
        }
    }

    fn nodo(&self, ino: u64) -> Option<Nodo> {
        if ino >= BASE_DOC {
            let j = ino - BASE_DOC;
            return (j < self.corpus.docs.len() as u64).then_some(Nodo::Doc(j as u32));
        }
        if ino >= BASE_ESP {
            let m = ino - BASE_ESP;
            return (m < MAGICOS.len() as u64).then_some(Nodo::Esp(m));
        }
        if ino >= BASE_CONS {
            let q = (ino - BASE_CONS) / 2;
            if q as usize >= self.consultas.len() {
                return None;
            }
            return Some(if (ino - BASE_CONS) % 2 == 0 { Nodo::Consulta(q) } else { Nodo::Resultados(q) });
        }
        if ino == 0 {
            return None;
        }
        let s = (ino - 1) / 4;
        if s as usize >= self.rutas.len() {
            return None;
        }
        match (ino - 1) % 4 {
            0 => Some(Nodo::Dir(s)),
            1 => Some(Nodo::Foco(s)),
            2 => Some(Nodo::Vista(s)),
            _ => None,
        }
    }

    fn ranura(&mut self, ruta: Vec<u32>) -> Option<u64> {
        if let Some(&s) = self.ranura_de.get(&ruta) {
            return Some(s);
        }
        if self.rutas.len() >= MAX_RANURAS {
            return None;
        }
        let s = self.rutas.len() as u64;
        self.rutas.push(ruta.clone());
        self.ranura_de.insert(ruta, s);
        Some(s)
    }

    fn vista(&mut self, s: u64) -> Rc<Vista> {
        if let Some(v) = self.cache.obtener(s) {
            return v;
        }
        let v = Rc::new(proyeccion::vista(&self.corpus, &self.rutas[s as usize], self.params));
        self.cache.insertar(s, v.clone());
        v
    }

    /// Hijos del árbol de alcance del foco que no están entre los k de la vista.
    pub fn extras(&self, v: &Vista) -> Vec<u32> {
        self.cobertura
            .hijos(v.ruta.last().copied())
            .iter()
            .copied()
            .filter(|h| !v.entradas.iter().any(|e| e.doc == *h))
            .collect()
    }

    fn ranura_consulta(&mut self, modo: Modo, clave: &str) -> Option<u64> {
        let k = (modo, clave.to_string());
        if let Some(&q) = self.consulta_de.get(&k) {
            return Some(q);
        }
        if self.consultas.len() >= MAX_CONSULTAS {
            return None;
        }
        let q = self.consultas.len() as u64;
        self.consultas.push(k.clone());
        self.consulta_de.insert(k, q);
        Some(q)
    }

    /// Resultados de la consulta de ranura `q`: se calculan en cada llamada (sin caché).
    pub fn resultados(&self, q: u64) -> Vec<(u32, f64)> {
        let (modo, clave) = &self.consultas[q as usize];
        directo::buscar(&self.corpus, &self.indice_bola, &self.lexico, *modo, clave, self.params.k).unwrap_or_default()
    }

    fn texto_resultados(&self, q: u64) -> String {
        let (modo, clave) = &self.consultas[q as usize];
        let mut s = format!("# {modo:?}: {clave}\n# rango puntaje nombre (puntaje = distancia hiperbólica; el ancla del híbrido lleva −coseno)\n");
        for (r, (j, d)) in self.resultados(q).iter().enumerate() {
            s.push_str(&format!("{:>3} {:>12.9} {}\n", r + 1, d, self.corpus.docs[*j as usize].nombre));
        }
        s
    }

    fn contenido(&mut self, n: &Nodo) -> Vec<u8> {
        match *n {
            Nodo::Foco(s) => {
                let v = self.vista(s);
                proyeccion::texto_foco(&self.corpus, &v, self.params).into_bytes()
            }
            Nodo::Vista(s) => {
                let v = self.vista(s);
                let mut t = proyeccion::texto_vista(&self.corpus, &v);
                let ex = self.extras(&v);
                if !ex.is_empty() {
                    let origen = vec![0.0; self.corpus.dim];
                    let xf = v.ruta.last().map(|&i| self.corpus.docs[i as usize].x.as_slice()).unwrap_or(&origen);
                    t.push_str("# cobertura (árbol de alcance; hijos de este foco que no están entre los k): distancia_al_foco nombre\n");
                    for j in ex {
                        let d = &self.corpus.docs[j as usize];
                        t.push_str(&format!("  + {:>12.9} {}\n", bola::distancia(xf, &d.x), d.nombre));
                    }
                }
                t.into_bytes()
            }
            Nodo::Doc(j) => self.corpus.docs[j as usize].contenido.clone(),
            Nodo::Resultados(q) => self.texto_resultados(q).into_bytes(),
            Nodo::Dir(_) | Nodo::Esp(_) | Nodo::Consulta(_) => Vec::new(),
        }
    }

    fn attr(&mut self, ino: u64, n: &Nodo) -> FileAttr {
        let (kind, perm, size, nlink) = match n {
            Nodo::Dir(_) | Nodo::Esp(_) | Nodo::Consulta(_) => (FileType::Directory, 0o555, 0, 2),
            Nodo::Doc(j) => (FileType::RegularFile, 0o444, self.corpus.docs[*j as usize].contenido.len() as u64, 1),
            _ => (FileType::RegularFile, 0o444, self.contenido(n).len() as u64, 1),
        };
        FileAttr {
            ino,
            size,
            blocks: size.div_ceil(512),
            atime: self.creado,
            mtime: self.creado,
            ctime: self.creado,
            crtime: self.creado,
            kind,
            perm,
            nlink,
            uid: self.uid,
            gid: self.gid,
            rdev: 0,
            blksize: 4096,
            flags: 0,
        }
    }

    /// Resuelve `nombre` dentro del directorio de ranura `s`.
    fn buscar(&mut self, s: u64, nombre: &str) -> Option<u64> {
        match nombre {
            ".foco" => return Some(4 * s + 2),
            ".vista" => return Some(4 * s + 3),
            _ => {}
        }
        if let Some(m) = MAGICOS.iter().position(|x| *x == nombre) {
            return Some(BASE_ESP + m as u64);
        }
        let v = self.vista(s);
        let ex = self.extras(&v);
        let visible = |j: u32| v.entradas.iter().any(|e| e.doc == j) || ex.contains(&j);
        if let Some(stem) = nombre.strip_suffix(".txt") {
            let j = self.corpus.indice(stem)?;
            return visible(j).then_some(BASE_DOC + j as u64);
        }
        let j = self.corpus.indice(nombre)?;
        if !visible(j) || v.ruta.len() >= MAX_PROFUNDIDAD {
            return None;
        }
        let mut ruta = v.ruta.clone();
        ruta.push(j);
        self.ranura(ruta).map(|c| 4 * c + 1)
    }

    /// Resuelve `nombre` dentro de un directorio mágico o de consulta.
    fn buscar_directo(&mut self, padre: &Nodo, nombre: &str) -> Option<u64> {
        match *padre {
            Nodo::Esp(0) => {
                let j = self.corpus.indice(nombre)?;
                self.ranura(vec![j]).map(|c| 4 * c + 1)
            }
            Nodo::Esp(m) => {
                let modo = modo_esp(m)?;
                if nombre.is_empty() || (matches!(modo, Modo::Similar | Modo::SimilarHibrido) && self.corpus.indice(nombre).is_none()) {
                    return None;
                }
                self.ranura_consulta(modo, nombre).map(|q| BASE_CONS + 2 * q)
            }
            Nodo::Consulta(q) => {
                if nombre == ".resultados" {
                    return Some(BASE_CONS + 2 * q + 1);
                }
                let (stem, es_txt) = match nombre.strip_suffix(".txt") {
                    Some(s) => (s, true),
                    None => (nombre, false),
                };
                let j = self.corpus.indice(stem)?;
                if !self.resultados(q).iter().any(|r| r.0 == j) {
                    return None;
                }
                if es_txt {
                    Some(BASE_DOC + j as u64)
                } else {
                    self.ranura(vec![j]).map(|c| 4 * c + 1)
                }
            }
            _ => None,
        }
    }

    /// Listado de una consulta. Los resultados se calculan en cada `readdir` con
    /// offset 0 (sin caché); un `readdir` posterior al final no recalcula.
    fn listar_consulta(&mut self, ino: u64, q: u64, offset: i64) -> Vec<(u64, FileType, String)> {
        if offset >= 3 + self.params.k as i64 {
            return Vec::new();
        }
        let res = self.resultados(q);
        let (modo, _) = self.consultas[q as usize];
        let padre = BASE_ESP + match modo {
            Modo::Similar => 1,
            Modo::Hibrido => 2,
            Modo::Bola => 3,
            Modo::SimilarHibrido => 4,
        };
        let mut lista = vec![
            (ino, FileType::Directory, ".".to_string()),
            (padre, FileType::Directory, "..".to_string()),
            (BASE_CONS + 2 * q + 1, FileType::RegularFile, ".resultados".to_string()),
        ];
        for (j, _) in res.iter() {
            lista.push((BASE_DOC + *j as u64, FileType::RegularFile, format!("{}.txt", self.corpus.docs[*j as usize].nombre)));
        }
        lista
    }
}

impl Filesystem for PoincareFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let Some(nombre) = name.to_str() else {
            return reply.error(libc::ENOENT);
        };
        let r = match self.nodo(parent) {
            Some(Nodo::Dir(s)) => self.buscar(s, nombre),
            Some(n @ (Nodo::Esp(_) | Nodo::Consulta(_))) => self.buscar_directo(&n, nombre),
            _ => return reply.error(libc::ENOTDIR),
        };
        match r.and_then(|ino| self.nodo(ino).map(|n| (ino, n))) {
            Some((ino, n)) => {
                let ttl = if matches!(n, Nodo::Esp(_) | Nodo::Consulta(_) | Nodo::Resultados(_)) { &TTL_DIRECTO } else { &TTL };
                reply.entry(ttl, &self.attr(ino, &n), 0)
            }
            None => reply.error(libc::ENOENT),
        }
    }

    fn getattr(&mut self, _req: &Request<'_>, ino: u64, _fh: Option<u64>, reply: ReplyAttr) {
        match self.nodo(ino) {
            Some(n) => reply.attr(&TTL, &self.attr(ino, &n)),
            None => reply.error(libc::ENOENT),
        }
    }

    fn open(&mut self, _req: &Request<'_>, ino: u64, flags: i32, reply: ReplyOpen) {
        if flags & libc::O_ACCMODE != libc::O_RDONLY {
            return reply.error(libc::EROFS);
        }
        match self.nodo(ino) {
            Some(Nodo::Dir(_) | Nodo::Esp(_) | Nodo::Consulta(_)) => reply.error(libc::EISDIR),
            Some(_) => reply.opened(0, 0),
            None => reply.error(libc::ENOENT),
        }
    }

    fn read(
        &mut self,
        _req: &Request<'_>,
        ino: u64,
        _fh: u64,
        offset: i64,
        size: u32,
        _flags: i32,
        _lock_owner: Option<u64>,
        reply: ReplyData,
    ) {
        let Some(n) = self.nodo(ino) else {
            return reply.error(libc::ENOENT);
        };
        if let Nodo::Dir(_) | Nodo::Esp(_) | Nodo::Consulta(_) = n {
            return reply.error(libc::EISDIR);
        }
        let datos = self.contenido(&n);
        let ini = (offset.max(0) as usize).min(datos.len());
        let fin = ini.saturating_add(size as usize).min(datos.len());
        reply.data(&datos[ini..fin]);
    }

    fn opendir(&mut self, _req: &Request<'_>, ino: u64, _flags: i32, reply: ReplyOpen) {
        match self.nodo(ino) {
            // Sin FOPEN_CACHE_DIR: cada `ls` llega al proceso y recalcula los resultados.
            // (Se probó FUSE_NO_OPENDIR_SUPPORT: el kernel entonces cachea el listado y
            // el `ls` repetido no recalcula; se descartó para medir honestamente.)
            Some(Nodo::Dir(_) | Nodo::Esp(_) | Nodo::Consulta(_)) => reply.opened(0, 0),
            Some(_) => reply.error(libc::ENOTDIR),
            None => reply.error(libc::ENOENT),
        }
    }

    fn readdir(&mut self, _req: &Request<'_>, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let s = match self.nodo(ino) {
            Some(Nodo::Dir(s)) => s,
            Some(Nodo::Esp(_)) => {
                let lista = [(ino, FileType::Directory, "."), (1, FileType::Directory, "..")];
                for (i, (ino, tipo, nombre)) in lista.into_iter().enumerate().skip(offset.max(0) as usize) {
                    if reply.add(ino, (i + 1) as i64, tipo, nombre) {
                        break;
                    }
                }
                return reply.ok();
            }
            Some(Nodo::Consulta(q)) => {
                let lista = self.listar_consulta(ino, q, offset);
                for (i, (ino, tipo, nombre)) in lista.into_iter().enumerate().skip(offset.max(0) as usize) {
                    if reply.add(ino, (i + 1) as i64, tipo, nombre) {
                        break;
                    }
                }
                return reply.ok();
            }
            _ => return reply.error(libc::ENOTDIR),
        };
        let v = self.vista(s);
        let padre = if v.ruta.is_empty() {
            1
        } else {
            let mut r = v.ruta.clone();
            r.pop();
            self.ranura(r).map(|p| 4 * p + 1).unwrap_or(1)
        };
        let mut lista: Vec<(u64, FileType, String)> = vec![
            (ino, FileType::Directory, ".".into()),
            (padre, FileType::Directory, "..".into()),
            (4 * s + 2, FileType::RegularFile, ".foco".into()),
            (4 * s + 3, FileType::RegularFile, ".vista".into()),
        ];
        // Orden de readdir = orden por distancia hiperbólica (ver con `ls -U`).
        let docs: Vec<u32> = v.entradas.iter().map(|e| e.doc).chain(self.extras(&v)).collect();
        for doc in docs {
            let nombre = self.corpus.docs[doc as usize].nombre.clone();
            if v.ruta.len() < MAX_PROFUNDIDAD {
                let mut r = v.ruta.clone();
                r.push(doc);
                if let Some(c) = self.ranura(r) {
                    lista.push((4 * c + 1, FileType::Directory, nombre.clone()));
                }
            }
            lista.push((BASE_DOC + doc as u64, FileType::RegularFile, format!("{nombre}.txt")));
        }
        for (i, (ino, tipo, nombre)) in lista.into_iter().enumerate().skip(offset.max(0) as usize) {
            if reply.add(ino, (i + 1) as i64, tipo, nombre) {
                break;
            }
        }
        reply.ok();
    }
}
