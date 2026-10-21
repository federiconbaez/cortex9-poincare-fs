//! Adaptador FUSE (crate `fuser`, solo lectura).
//!
//! Inodos:
//! * directorio con ranura `s` (una ruta de `cd`): `4s + 1` (la raíz es 1);
//! * `.foco` de ese directorio: `4s + 2`; `.vista`: `4s + 3`;
//! * contenido del documento `j` (`<nombre>.txt`): `BASE_DOC + j`.

use crate::proyeccion::{self, Corpus, Params, Vista, MAX_PROFUNDIDAD};
use fuser::{
    FileAttr, FileType, Filesystem, ReplyAttr, ReplyData, ReplyDirectory, ReplyEntry, ReplyOpen, Request,
};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::time::{Duration, SystemTime};

pub const BASE_DOC: u64 = 1 << 60;
pub const MAX_RANURAS: usize = 1 << 20;
const TTL: Duration = Duration::from_secs(1);

enum Nodo {
    Dir(u64),
    Foco(u64),
    Vista(u64),
    Doc(u32),
}

pub struct PoincareFs {
    corpus: Corpus,
    params: Params,
    rutas: Vec<Vec<u32>>,
    ranura_de: HashMap<Vec<u32>, u64>,
    uid: u32,
    gid: u32,
    creado: SystemTime,
}

impl PoincareFs {
    pub fn new(corpus: Corpus, params: Params) -> Self {
        let mut ranura_de = HashMap::new();
        ranura_de.insert(Vec::new(), 0);
        // SAFETY: getuid/getgid no fallan.
        let (uid, gid) = unsafe { (libc::getuid(), libc::getgid()) };
        PoincareFs {
            corpus,
            params,
            rutas: vec![Vec::new()],
            ranura_de,
            uid,
            gid,
            creado: SystemTime::now(),
        }
    }

    fn nodo(&self, ino: u64) -> Option<Nodo> {
        if ino >= BASE_DOC {
            let j = ino - BASE_DOC;
            return (j < self.corpus.docs.len() as u64).then_some(Nodo::Doc(j as u32));
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

    fn vista(&self, s: u64) -> Vista {
        proyeccion::vista(&self.corpus, &self.rutas[s as usize], self.params)
    }

    fn contenido(&self, n: &Nodo) -> Vec<u8> {
        match *n {
            Nodo::Foco(s) => proyeccion::texto_foco(&self.corpus, &self.vista(s)).into_bytes(),
            Nodo::Vista(s) => proyeccion::texto_vista(&self.corpus, &self.vista(s)).into_bytes(),
            Nodo::Doc(j) => self.corpus.docs[j as usize].contenido.clone(),
            Nodo::Dir(_) => Vec::new(),
        }
    }

    fn attr(&self, ino: u64, n: &Nodo) -> FileAttr {
        let (kind, perm, size, nlink) = match n {
            Nodo::Dir(_) => (FileType::Directory, 0o555, 0, 2),
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
        let v = self.vista(s);
        let visible = |j: u32| v.entradas.iter().any(|e| e.doc == j);
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
}

impl Filesystem for PoincareFs {
    fn lookup(&mut self, _req: &Request<'_>, parent: u64, name: &OsStr, reply: ReplyEntry) {
        let Some(Nodo::Dir(s)) = self.nodo(parent) else {
            return reply.error(libc::ENOTDIR);
        };
        let Some(nombre) = name.to_str() else {
            return reply.error(libc::ENOENT);
        };
        match self.buscar(s, nombre).and_then(|ino| self.nodo(ino).map(|n| (ino, n))) {
            Some((ino, n)) => reply.entry(&TTL, &self.attr(ino, &n), 0),
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
            Some(Nodo::Dir(_)) => reply.error(libc::EISDIR),
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
        if let Nodo::Dir(_) = n {
            return reply.error(libc::EISDIR);
        }
        let datos = self.contenido(&n);
        let ini = (offset.max(0) as usize).min(datos.len());
        let fin = ini.saturating_add(size as usize).min(datos.len());
        reply.data(&datos[ini..fin]);
    }

    fn readdir(&mut self, _req: &Request<'_>, ino: u64, _fh: u64, offset: i64, mut reply: ReplyDirectory) {
        let Some(Nodo::Dir(s)) = self.nodo(ino) else {
            return reply.error(libc::ENOTDIR);
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
        for e in &v.entradas {
            let nombre = self.corpus.docs[e.doc as usize].nombre.clone();
            if v.ruta.len() < MAX_PROFUNDIDAD {
                let mut r = v.ruta.clone();
                r.push(e.doc);
                if let Some(c) = self.ranura(r) {
                    lista.push((4 * c + 1, FileType::Directory, nombre.clone()));
                }
            }
            lista.push((BASE_DOC + e.doc as u64, FileType::RegularFile, format!("{nombre}.txt")));
        }
        for (i, (ino, tipo, nombre)) in lista.into_iter().enumerate().skip(offset.max(0) as usize) {
            if reply.add(ino, (i + 1) as i64, tipo, nombre) {
                break;
            }
        }
        reply.ok();
    }
}
