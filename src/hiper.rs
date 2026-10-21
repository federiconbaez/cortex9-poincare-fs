//! Geometría del disco de Poincaré: complejos, distancia hiperbólica y
//! transformaciones de Möbius que preservan el disco (grupo SU(1,1)).
//!
//! Una transformación se representa con la matriz `[[α, β], [β̄, ᾱ]]`,
//! `|α|² − |β|² = 1`, y actúa como `z ↦ (α z + β) / (β̄ z + ᾱ)`.
//! Componer = multiplicar matrices; invertir = `[[ᾱ, −β], [−β̄, α]]`.

use std::ops::{Add, Div, Mul, Neg, Sub};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct C {
    pub re: f64,
    pub im: f64,
}

impl C {
    pub const CERO: C = C { re: 0.0, im: 0.0 };
    pub const UNO: C = C { re: 1.0, im: 0.0 };
    pub const fn new(re: f64, im: f64) -> C {
        C { re, im }
    }
    pub fn conj(self) -> C {
        C::new(self.re, -self.im)
    }
    pub fn norma2(self) -> f64 {
        self.re * self.re + self.im * self.im
    }
    pub fn abs(self) -> f64 {
        self.re.hypot(self.im)
    }
    pub fn arg(self) -> f64 {
        self.im.atan2(self.re)
    }
    pub fn escalar(self, k: f64) -> C {
        C::new(self.re * k, self.im * k)
    }
}

impl Add for C {
    type Output = C;
    fn add(self, o: C) -> C {
        C::new(self.re + o.re, self.im + o.im)
    }
}
impl Sub for C {
    type Output = C;
    fn sub(self, o: C) -> C {
        C::new(self.re - o.re, self.im - o.im)
    }
}
impl Neg for C {
    type Output = C;
    fn neg(self) -> C {
        C::new(-self.re, -self.im)
    }
}
impl Mul for C {
    type Output = C;
    fn mul(self, o: C) -> C {
        C::new(self.re * o.re - self.im * o.im, self.re * o.im + self.im * o.re)
    }
}
impl Div for C {
    type Output = C;
    fn div(self, o: C) -> C {
        let d = o.norma2();
        C::new((self.re * o.re + self.im * o.im) / d, (self.im * o.re - self.re * o.im) / d)
    }
}

/// Distancia hiperbólica en el disco de Poincaré:
/// `d(z, w) = 2·artanh(|z − w| / |1 − z̄ w|)`.
pub fn distancia(z: C, w: C) -> f64 {
    let num = (z - w).abs();
    let den = (C::UNO - z.conj() * w).abs();
    let t = (num / den).min(1.0 - f64::EPSILON);
    2.0 * t.atanh()
}

/// Distancia hiperbólica al origen: `2·artanh(|z|)`.
pub fn distancia_al_origen(z: C) -> f64 {
    2.0 * z.abs().min(1.0 - f64::EPSILON).atanh()
}

/// Automorfismo del disco unidad.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Mobius {
    pub alfa: C,
    pub beta: C,
}

impl Mobius {
    pub const IDENTIDAD: Mobius = Mobius { alfa: C::UNO, beta: C::CERO };

    /// La transformación que lleva `p` (|p| < 1) al origen:
    /// `z ↦ (z − p) / (1 − p̄ z)`.
    pub fn al_origen(p: C) -> Mobius {
        assert!(p.norma2() < 1.0, "el punto debe estar dentro del disco");
        let s = 1.0 / (1.0 - p.norma2()).sqrt();
        Mobius { alfa: C::new(s, 0.0), beta: -p.escalar(s) }
    }

    /// Rotación `z ↦ e^{iθ} z`.
    pub fn rotacion(theta: f64) -> Mobius {
        let h = theta / 2.0;
        Mobius { alfa: C::new(h.cos(), h.sin()), beta: C::CERO }
    }

    pub fn aplicar(&self, z: C) -> C {
        (self.alfa * z + self.beta) / (self.beta.conj() * z + self.alfa.conj())
    }

    /// `self ∘ otra` (primero `otra`, después `self`), renormalizada.
    pub fn componer(&self, otra: &Mobius) -> Mobius {
        // [[a, b],[b̄, ā]] · [[c, d],[d̄, c̄]] = [[ac + b d̄, ad + b c̄], ...]
        let a = self.alfa * otra.alfa + self.beta * otra.beta.conj();
        let b = self.alfa * otra.beta + self.beta * otra.alfa.conj();
        Mobius { alfa: a, beta: b }.normalizada()
    }

    pub fn inversa(&self) -> Mobius {
        Mobius { alfa: self.alfa.conj(), beta: -self.beta }
    }

    /// Determinante `|α|² − |β|²` (debe ser 1).
    pub fn det(&self) -> f64 {
        self.alfa.norma2() - self.beta.norma2()
    }

    fn normalizada(self) -> Mobius {
        let s = 1.0 / self.det().sqrt();
        Mobius { alfa: self.alfa.escalar(s), beta: self.beta.escalar(s) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    /// Puntos dentro del disco con |z| ≤ 0.95 (zona donde exigimos 1e-9).
    fn punto() -> impl Strategy<Value = C> {
        (0.0f64..0.95, -std::f64::consts::PI..std::f64::consts::PI)
            .prop_map(|(r, t)| C::new(r * t.cos(), r * t.sin()))
    }

    fn mobius() -> impl Strategy<Value = Mobius> {
        (punto(), -std::f64::consts::PI..std::f64::consts::PI)
            .prop_map(|(p, t)| Mobius::rotacion(t).componer(&Mobius::al_origen(p)))
    }

    fn cerca(a: C, b: C, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 4096, ..ProptestConfig::default() })]

        #[test]
        fn preserva_distancia_hiperbolica(m in mobius(), z in punto(), w in punto()) {
            let d0 = distancia(z, w);
            let d1 = distancia(m.aplicar(z), m.aplicar(w));
            // Error absoluto ≤ 1e-9.
            prop_assert!((d0 - d1).abs() <= 1e-9, "d0={d0} d1={d1}");
        }

        #[test]
        fn lleva_el_disco_al_disco(m in mobius(), z in punto()) {
            prop_assert!(m.aplicar(z).abs() < 1.0);
        }

        #[test]
        fn borde_va_al_borde(m in mobius(), t in -3.2f64..3.2) {
            let z = C::new(t.cos(), t.sin());
            prop_assert!((m.aplicar(z).abs() - 1.0).abs() < 1e-9);
        }

        #[test]
        fn al_origen_manda_p_a_cero(p in punto()) {
            prop_assert!(Mobius::al_origen(p).aplicar(p).abs() < 1e-12);
        }

        #[test]
        fn distancia_al_origen_coincide(z in punto()) {
            prop_assert!((distancia(C::CERO, z) - distancia_al_origen(z)).abs() < 1e-12);
        }

        #[test]
        fn composicion_es_aplicacion_sucesiva(a in mobius(), b in mobius(), z in punto()) {
            let directo = a.aplicar(b.aplicar(z));
            let compuesto = a.componer(&b).aplicar(z);
            prop_assert!(cerca(directo, compuesto, 1e-9), "{directo:?} vs {compuesto:?}");
        }

        #[test]
        fn inversa_deshace(m in mobius(), z in punto()) {
            prop_assert!(cerca(m.inversa().aplicar(m.aplicar(z)), z, 1e-9));
            prop_assert!(cerca(m.componer(&m.inversa()).aplicar(z), z, 1e-9));
        }

        #[test]
        fn determinante_uno(a in mobius(), b in mobius()) {
            prop_assert!((a.componer(&b).det() - 1.0).abs() < 1e-12);
        }

        #[test]
        fn distancia_simetrica_y_triangular(z in punto(), w in punto(), u in punto()) {
            prop_assert!((distancia(z, w) - distancia(w, z)).abs() < 1e-12);
            prop_assert!(distancia(z, u) <= distancia(z, w) + distancia(w, u) + 1e-9);
        }
    }

    /// Barrido determinista (semilla fija) de 1 000 000 de casos; imprime el error máximo.
    #[test]
    fn barrido_determinista_error_maximo() {
        let mut estado: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut u = move || {
            // splitmix64
            estado = estado.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = estado;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            ((z ^ (z >> 31)) >> 11) as f64 / (1u64 << 53) as f64
        };
        let mut p = || {
            let r = 0.95 * u();
            let t = std::f64::consts::TAU * u();
            C::new(r * t.cos(), r * t.sin())
        };
        let mut max_err: f64 = 0.0;
        let mut max_disco: f64 = 0.0;
        for _ in 0..1_000_000 {
            let (a, z, w) = (p(), p(), p());
            let m = Mobius::al_origen(a);
            let d0 = distancia(z, w);
            let d1 = distancia(m.aplicar(z), m.aplicar(w));
            max_err = max_err.max((d0 - d1).abs());
            max_disco = max_disco.max(m.aplicar(z).abs());
        }
        println!("error_maximo_distancia={max_err:e} max_|T(z)|={max_disco}");
        assert!(max_err <= 1e-9);
        assert!(max_disco < 1.0);
    }

    #[test]
    fn identidad() {
        let z = C::new(0.3, -0.4);
        assert_eq!(Mobius::IDENTIDAD.aplicar(z), z);
        assert_eq!(distancia(z, z), 0.0);
    }

    #[test]
    #[should_panic]
    fn al_origen_fuera_del_disco_falla() {
        let _ = Mobius::al_origen(C::new(1.0, 0.0));
    }
}
