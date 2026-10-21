//! Respaldo real de io_uring: se ejecuta el binario con un filtro seccomp que
//! hace fallar `io_uring_setup` (como en contenedores que lo bloquean) y se
//! verifica que `auto` cae a pread y lee exactamente lo mismo.

#![cfg(all(target_os = "linux", target_arch = "x86_64"))]

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_poincare-fs");
const NR_IO_URING_SETUP: u32 = 425;
const AUDIT_ARCH_X86_64: u32 = 0xC000_003E;

fn instalar_filtro() -> std::io::Result<()> {
    // BPF clásico: si arch == x86_64 y nr == io_uring_setup => ERRNO(ENOSYS)
    let f = |code: u16, jt: u8, jf: u8, k: u32| libc::sock_filter { code, jt, jf, k };
    const LD_W_ABS: u16 = (libc::BPF_LD | libc::BPF_W | libc::BPF_ABS) as u16;
    const JEQ_K: u16 = (libc::BPF_JMP | libc::BPF_JEQ | libc::BPF_K) as u16;
    const RET_K: u16 = (libc::BPF_RET | libc::BPF_K) as u16;
    let prog = [
        f(LD_W_ABS, 0, 0, 4), // seccomp_data.arch
        f(JEQ_K, 0, 3, AUDIT_ARCH_X86_64),
        f(LD_W_ABS, 0, 0, 0), // seccomp_data.nr
        f(JEQ_K, 0, 1, NR_IO_URING_SETUP),
        f(RET_K, 0, 0, libc::SECCOMP_RET_ERRNO | libc::ENOSYS as u32),
        f(RET_K, 0, 0, libc::SECCOMP_RET_ALLOW),
    ];
    let fprog = libc::sock_fprog { len: prog.len() as u16, filter: prog.as_ptr() as *mut _ };
    // SAFETY: llamadas prctl estándar en el hijo antes de exec.
    unsafe {
        if libc::prctl(libc::PR_SET_NO_NEW_PRIVS, 1, 0, 0, 0) != 0 {
            return Err(std::io::Error::last_os_error());
        }
        if libc::prctl(libc::PR_SET_SECCOMP, libc::SECCOMP_MODE_FILTER, &fprog as *const _ as libc::c_ulong) != 0 {
            return Err(std::io::Error::last_os_error());
        }
    }
    Ok(())
}

fn es_info(modo: &str, con_filtro: bool) -> (bool, String) {
    let corpus = Path::new(env!("CARGO_MANIFEST_DIR")).join("corpus");
    let mut c = Command::new(BIN);
    c.arg("es-info").arg(corpus).env("POINCARE_IO", modo);
    if con_filtro {
        // SAFETY: el cierre solo hace llamadas async-signal-safe (prctl).
        unsafe { c.pre_exec(instalar_filtro) };
    }
    let o = c.output().unwrap();
    let s = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    (o.status.success(), s)
}

fn campo<'a>(s: &'a str, k: &str) -> &'a str {
    s.split_whitespace().find_map(|t| t.strip_prefix(k)).unwrap_or("")
}

#[test]
fn auto_usa_uring_y_con_seccomp_cae_a_pread() {
    let (ok, normal) = es_info("auto", false);
    assert!(ok, "{normal}");
    assert_eq!(campo(&normal, "modo_usado="), "Uring", "{normal}");

    let (ok, bloqueado) = es_info("auto", true);
    assert!(ok, "{bloqueado}");
    assert_eq!(campo(&bloqueado, "modo_usado="), "Pread", "{bloqueado}");
    assert!(campo(&bloqueado, "sonda_uring=").starts_with("io_uring_setup"), "{bloqueado}");
    // mismos bytes leídos
    assert_eq!(campo(&normal, "fnv="), campo(&bloqueado, "fnv="));
    assert_eq!(campo(&normal, "bytes="), campo(&bloqueado, "bytes="));

    // pedir io_uring explícitamente con seccomp debe fallar (no esconder el problema)
    let (ok, forzado) = es_info("uring", true);
    assert!(!ok, "{forzado}");
    assert!(forzado.contains("io_uring no disponible"), "{forzado}");

    let (ok, pread) = es_info("pread", false);
    assert!(ok);
    assert_eq!(campo(&pread, "modo_usado="), "Pread");
    assert_eq!(campo(&normal, "fnv="), campo(&pread, "fnv="));
}
