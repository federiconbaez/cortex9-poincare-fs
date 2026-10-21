//! Entrada/salida del almacén (corpus e índice): io_uring con respaldo pread.
//!
//! * [`Modo::Uring`]: abre los archivos y envía lecturas por io_uring (crate
//!   `io-uring`), con hasta `PROFUNDIDAD` lecturas en vuelo y reintento de
//!   lecturas cortas.
//! * [`Modo::Pread`]: `pread(2)` secuencial (vía `FileExt::read_at`).
//! * [`Modo::Auto`]: prueba io_uring (crea un anillo y ejecuta un NOP); si el
//!   kernel, seccomp o el contenedor lo impiden, usa pread.
//!
//! Variable de entorno `POINCARE_IO=auto|uring|pread` (por defecto `auto`).

use std::fs::File;
use std::io;
use std::os::unix::fs::FileExt;
use std::os::unix::io::AsRawFd;
use std::path::PathBuf;

const PROFUNDIDAD: u32 = 64;
const TROZO: usize = 1 << 20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modo {
    Auto,
    Uring,
    Pread,
}

impl Modo {
    pub fn desde_entorno() -> io::Result<Modo> {
        match std::env::var("POINCARE_IO").as_deref() {
            Err(_) | Ok("auto") | Ok("") => Ok(Modo::Auto),
            Ok("uring") => Ok(Modo::Uring),
            Ok("pread") => Ok(Modo::Pread),
            Ok(o) => Err(io::Error::new(io::ErrorKind::InvalidInput, format!("POINCARE_IO inválido: {o}"))),
        }
    }
}

/// ¿Funciona io_uring acá? Crea un anillo y ejecuta un NOP.
pub fn sondear_uring() -> Result<(), String> {
    let mut ring = io_uring::IoUring::new(2).map_err(|e| format!("io_uring_setup: {e}"))?;
    let nop = io_uring::opcode::Nop::new().build().user_data(7);
    // SAFETY: el NOP no referencia memoria.
    unsafe { ring.submission().push(&nop) }.map_err(|_| "cola llena".to_string())?;
    ring.submit_and_wait(1).map_err(|e| format!("io_uring_enter: {e}"))?;
    let cqe = ring.completion().next().ok_or("sin completado")?;
    if cqe.user_data() != 7 || cqe.result() < 0 {
        return Err(format!("NOP falló: {}", cqe.result()));
    }
    Ok(())
}

/// Resuelve `Auto` al modo real que se va a usar.
pub fn resolver(m: Modo) -> io::Result<Modo> {
    match m {
        Modo::Pread => Ok(Modo::Pread),
        Modo::Uring => sondear_uring()
            .map(|_| Modo::Uring)
            .map_err(|e| io::Error::new(io::ErrorKind::Unsupported, format!("io_uring no disponible: {e}"))),
        Modo::Auto => Ok(if sondear_uring().is_ok() { Modo::Uring } else { Modo::Pread }),
    }
}

fn abrir(rutas: &[PathBuf]) -> io::Result<Vec<(File, usize)>> {
    rutas
        .iter()
        .map(|r| {
            let f = File::open(r)?;
            let n = f.metadata()?.len() as usize;
            Ok((f, n))
        })
        .collect()
}

fn leer_pread(rutas: &[PathBuf]) -> io::Result<Vec<Vec<u8>>> {
    let mut out = Vec::with_capacity(rutas.len());
    for (f, n) in abrir(rutas)? {
        let mut buf = vec![0u8; n];
        let mut hecho = 0;
        while hecho < n {
            let r = f.read_at(&mut buf[hecho..], hecho as u64)?;
            if r == 0 {
                buf.truncate(hecho); // el archivo se achicó
                break;
            }
            hecho += r;
        }
        out.push(buf);
    }
    Ok(out)
}

fn leer_uring(rutas: &[PathBuf]) -> io::Result<Vec<Vec<u8>>> {
    use io_uring::{opcode, types, IoUring};
    let archivos = abrir(rutas)?;
    let mut bufs: Vec<Vec<u8>> = archivos.iter().map(|(_, n)| vec![0u8; *n]).collect();
    let mut hecho: Vec<usize> = vec![0; archivos.len()];
    let mut fin: Vec<bool> = archivos.iter().map(|(_, n)| *n == 0).collect();
    // cola de trabajo: índices de archivo con lectura pendiente
    let mut pendientes: std::collections::VecDeque<usize> = (0..archivos.len()).filter(|&i| !fin[i]).collect();
    let mut ring = IoUring::new(PROFUNDIDAD)?;
    let mut en_vuelo = 0usize;
    while !pendientes.is_empty() || en_vuelo > 0 {
        while en_vuelo < PROFUNDIDAD as usize {
            let Some(i) = pendientes.pop_front() else { break };
            let ini = hecho[i];
            let len = (bufs[i].len() - ini).min(TROZO);
            let ptr = bufs[i][ini..].as_mut_ptr();
            let sqe = opcode::Read::new(types::Fd(archivos[i].0.as_raw_fd()), ptr, len as u32)
                .offset(ini as u64)
                .build()
                .user_data(i as u64);
            // SAFETY: el buffer vive (en `bufs`) hasta que llega su completado,
            // y no se toca mientras la lectura está en vuelo.
            unsafe { ring.submission().push(&sqe) }
                .map_err(|_| io::Error::other("cola de envío llena"))?;
            en_vuelo += 1;
        }
        ring.submit_and_wait(1)?;
        let completados: Vec<(u64, i32)> = ring.completion().map(|c| (c.user_data(), c.result())).collect();
        for (ud, res) in completados {
            en_vuelo -= 1;
            let i = ud as usize;
            if res < 0 {
                return Err(io::Error::from_raw_os_error(-res));
            }
            if res == 0 {
                bufs[i].truncate(hecho[i]);
                fin[i] = true;
                continue;
            }
            hecho[i] += res as usize;
            if hecho[i] >= bufs[i].len() {
                fin[i] = true;
            } else {
                pendientes.push_back(i); // lectura corta o archivo > TROZO
            }
        }
    }
    Ok(bufs)
}

/// Lee archivos completos con el modo pedido. Devuelve el modo efectivo.
pub fn leer_todos(rutas: &[PathBuf], modo: Modo) -> io::Result<(Modo, Vec<Vec<u8>>)> {
    let m = resolver(modo)?;
    let datos = match m {
        Modo::Uring => leer_uring(rutas)?,
        _ => leer_pread(rutas)?,
    };
    Ok((m, datos))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(nombre: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("poincare-es-{}-{nombre}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn archivos_de_prueba(d: &std::path::Path) -> (Vec<PathBuf>, Vec<Vec<u8>>) {
        // tamaños variados: vacío, chico, 3 MiB (> TROZO, fuerza varias lecturas) y 200 chicos
        let mut rutas = Vec::new();
        let mut datos = Vec::new();
        let tams = [0usize, 1, 4095, 4096, 3 * TROZO + 17];
        for (k, &t) in tams.iter().chain([100usize; 200].iter()).enumerate() {
            let v: Vec<u8> = (0..t).map(|i| (i * 31 + k * 7) as u8).collect();
            let r = d.join(format!("f{k}"));
            std::fs::write(&r, &v).unwrap();
            rutas.push(r);
            datos.push(v);
        }
        (rutas, datos)
    }

    #[test]
    fn pread_lee_exacto() {
        let d = tmp("pread");
        let (rutas, esperado) = archivos_de_prueba(&d);
        let (m, leido) = leer_todos(&rutas, Modo::Pread).unwrap();
        assert_eq!(m, Modo::Pread);
        assert_eq!(leido, esperado);
        let _ = std::fs::remove_dir_all(&d);
    }

    /// Exige io_uring salvo que se pida explícitamente permitir su ausencia.
    #[test]
    fn uring_lee_exacto() {
        let d = tmp("uring");
        let (rutas, esperado) = archivos_de_prueba(&d);
        match leer_todos(&rutas, Modo::Uring) {
            Ok((m, leido)) => {
                assert_eq!(m, Modo::Uring);
                assert_eq!(leido, esperado);
            }
            Err(e) if std::env::var("POINCARE_PERMITIR_SIN_URING").is_ok() => {
                eprintln!("io_uring no disponible (permitido por entorno): {e}");
            }
            Err(e) => panic!("io_uring no disponible: {e}"),
        }
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn auto_y_error_de_archivo() {
        let d = tmp("auto");
        let (rutas, esperado) = archivos_de_prueba(&d);
        assert_eq!(leer_todos(&rutas, Modo::Auto).unwrap().1, esperado);
        assert!(leer_todos(&[d.join("no-existe")], Modo::Pread).is_err());
        assert!(leer_todos(&[d.join("no-existe")], Modo::Auto).is_err());
        let _ = std::fs::remove_dir_all(&d);
    }
}
