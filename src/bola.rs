//! Bola de Poincaré d-dimensional (curvatura −1) con operaciones de
//! girovectores de Ungar.
//!
//! * Suma de Möbius: `x ⊕ y = ((1 + 2⟨x,y⟩ + |y|²) x + (1 − |x|²) y) / (1 + 2⟨x,y⟩ + |x|²|y|²)`
//! * Distancia: `d(x, y) = 2·artanh |(−x) ⊕ y|`
//! * Giración: `gyr[a,b]c = ⊖(a ⊕ b) ⊕ (a ⊕ (b ⊕ c))`; es lineal y ortogonal, y se
//!   calcula con la fórmula cerrada `c + 2(A a + B b)/D`.
//! * Mapas exponencial/logarítmico en el origen y en un punto cualquiera.
//! * [`Transf`]: isometría `z ↦ t ⊕ R z` (traslación a izquierda ∘ rotación).
//!   Componer una traslación `L_a` con `(t, R)` da `(a ⊕ t, gyr[a,t]·R)`.

pub type V = Vec<f64>;

pub fn dot(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}
pub fn norma2(a: &[f64]) -> f64 {
    dot(a, a)
}
pub fn norma(a: &[f64]) -> f64 {
    norma2(a).sqrt()
}
pub fn neg(a: &[f64]) -> V {
    a.iter().map(|x| -x).collect()
}
pub fn escalar(a: &[f64], k: f64) -> V {
    a.iter().map(|x| x * k).collect()
}

/// Radio máximo permitido (proyección numérica dentro de la bola).
pub const RADIO_SEGURO: f64 = 1.0 - 1e-9;

/// Proyecta adentro de la bola si quedó afuera por redondeo.
pub fn proyectar(x: &mut [f64], radio: f64) {
    let n = norma(x);
    if n > radio {
        let k = radio / n;
        x.iter_mut().for_each(|v| *v *= k);
    }
}

/// Suma de Möbius `x ⊕ y`.
pub fn suma(x: &[f64], y: &[f64]) -> V {
    let xy = dot(x, y);
    let x2 = norma2(x);
    let y2 = norma2(y);
    let a = 1.0 + 2.0 * xy + y2;
    let b = 1.0 - x2;
    let den = 1.0 + 2.0 * xy + x2 * y2;
    x.iter().zip(y).map(|(xi, yi)| (a * xi + b * yi) / den).collect()
}

/// Distancia hiperbólica `2·artanh|(−x) ⊕ y|`, sin reservar memoria, usando
/// `|(−x) ⊕ y|² = |x − y|² / (1 − 2⟨x,y⟩ + |x|²|y|²)` (numerador como suma de
/// diferencias al cuadrado: preciso para puntos cercanos).
pub fn distancia(x: &[f64], y: &[f64]) -> f64 {
    let (mut dif2, mut xy, mut x2, mut y2) = (0.0, 0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        let d = a - b;
        dif2 += d * d;
        xy += a * b;
        x2 += a * a;
        y2 += b * b;
    }
    let den = 1.0 - 2.0 * xy + x2 * y2;
    let m = (dif2 / den).sqrt().min(1.0 - f64::EPSILON);
    2.0 * m.atanh()
}

/// Distancia por la definición con suma de Möbius (referencia para tests).
pub fn distancia_por_suma(x: &[f64], y: &[f64]) -> f64 {
    let m = norma(&suma(&neg(x), y)).min(1.0 - f64::EPSILON);
    2.0 * m.atanh()
}

pub fn distancia_al_origen(x: &[f64]) -> f64 {
    2.0 * norma(x).min(1.0 - f64::EPSILON).atanh()
}

/// Giración por la fórmula cerrada (lineal en `c`, válida para todo `c ∈ ℝ^d`).
pub fn gyr(a: &[f64], b: &[f64], c: &[f64]) -> V {
    let ab = dot(a, b);
    let ac = dot(a, c);
    let bc = dot(b, c);
    let a2 = norma2(a);
    let b2 = norma2(b);
    let aa = -ac * b2 + bc + 2.0 * ab * bc;
    let bb = -bc * a2 - ac;
    let d = 1.0 + 2.0 * ab + a2 * b2;
    c.iter()
        .zip(a.iter().zip(b))
        .map(|(ci, (ai, bi))| ci + 2.0 * (aa * ai + bb * bi) / d)
        .collect()
}

/// Giración por su definición `⊖(a ⊕ b) ⊕ (a ⊕ (b ⊕ c))` (solo para `c` en la bola).
pub fn gyr_definicion(a: &[f64], b: &[f64], c: &[f64]) -> V {
    suma(&neg(&suma(a, b)), &suma(a, &suma(b, c)))
}

/// `exp_0(v) = tanh|v| · v/|v|`.
pub fn exp0(v: &[f64]) -> V {
    let n = norma(v);
    if n == 0.0 {
        return v.to_vec();
    }
    escalar(v, n.tanh() / n)
}

/// `log_0(y) = artanh|y| · y/|y|`.
pub fn log0(y: &[f64]) -> V {
    let n = norma(y);
    if n == 0.0 {
        return y.to_vec();
    }
    escalar(y, n.min(1.0 - f64::EPSILON).atanh() / n)
}

fn lambda(x: &[f64]) -> f64 {
    2.0 / (1.0 - norma2(x))
}

/// `exp_x(v) = x ⊕ (tanh(λ_x |v| / 2) · v/|v|)`.
pub fn exp(x: &[f64], v: &[f64]) -> V {
    let n = norma(v);
    if n == 0.0 {
        return x.to_vec();
    }
    suma(x, &escalar(v, (lambda(x) * n / 2.0).tanh() / n))
}

/// `log_x(y) = (2/λ_x) · artanh|−x ⊕ y| · (−x ⊕ y)/|−x ⊕ y|`.
pub fn log(x: &[f64], y: &[f64]) -> V {
    let w = suma(&neg(x), y);
    let n = norma(&w);
    if n == 0.0 {
        return w;
    }
    escalar(&w, 2.0 / lambda(x) * n.min(1.0 - f64::EPSILON).atanh() / n)
}

/// Isometría `z ↦ t ⊕ R z` con `R` ortogonal (fila mayor, d×d).
#[derive(Debug, Clone, PartialEq)]
pub struct Transf {
    pub t: V,
    pub r: V,
    pub d: usize,
}

impl Transf {
    pub fn identidad(d: usize) -> Transf {
        let mut r = vec![0.0; d * d];
        for i in 0..d {
            r[i * d + i] = 1.0;
        }
        Transf { t: vec![0.0; d], r, d }
    }

    pub fn rotar(&self, z: &[f64]) -> V {
        (0..self.d).map(|i| dot(&self.r[i * self.d..(i + 1) * self.d], z)).collect()
    }

    pub fn aplicar(&self, z: &[f64]) -> V {
        suma(&self.t, &self.rotar(z))
    }

    /// `L_a ∘ self`: `z ↦ a ⊕ (t ⊕ Rz) = (a ⊕ t) ⊕ gyr[a,t](Rz)`.
    pub fn trasladar_izq(&self, a: &[f64]) -> Transf {
        let d = self.d;
        let t = suma(a, &self.t);
        // Matriz de gyr[a,t] (lineal): G·e_j por columnas.
        let mut g = vec![0.0; d * d];
        let mut e = vec![0.0; d];
        for j in 0..d {
            e[j] = 1.0;
            let col = gyr(a, &self.t, &e);
            for i in 0..d {
                g[i * d + j] = col[i];
            }
            e[j] = 0.0;
        }
        let mut r = vec![0.0; d * d];
        for i in 0..d {
            for k in 0..d {
                let gik = g[i * d + k];
                if gik != 0.0 {
                    for j in 0..d {
                        r[i * d + j] += gik * self.r[k * d + j];
                    }
                }
            }
        }
        Transf { t, r, d }
    }

    /// El `cd`: lleva el punto `x` (coordenadas originales) al origen.
    pub fn enfocar(&self, x: &[f64]) -> Transf {
        let p = self.aplicar(x);
        self.trasladar_izq(&neg(&p))
    }

    /// Máximo |RᵀR − I| (cuánto se alejó R de ser ortogonal).
    pub fn error_ortogonal(&self) -> f64 {
        let d = self.d;
        let mut m: f64 = 0.0;
        for i in 0..d {
            for j in 0..d {
                let s: f64 = (0..d).map(|k| self.r[k * d + i] * self.r[k * d + j]).sum();
                m = m.max((s - if i == j { 1.0 } else { 0.0 }).abs());
            }
        }
        m
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hiper::{Mobius, C};
    use proptest::prelude::*;

    const D: usize = 16;

    /// Punto de la bola con |x| ≤ 0.95 en dimensión `d`.
    fn punto_d(d: usize) -> impl Strategy<Value = V> {
        (proptest::collection::vec(-1.0f64..1.0, d), 0.0f64..0.95).prop_map(|(v, r)| {
            let n = norma(&v);
            if n == 0.0 {
                v
            } else {
                escalar(&v, r / n)
            }
        })
    }
    fn punto() -> impl Strategy<Value = V> {
        punto_d(D)
    }
    fn cerca(a: &[f64], b: &[f64], tol: f64) -> bool {
        a.iter().zip(b).all(|(x, y)| (x - y).abs() <= tol)
    }
    /// Transformación como composición de 1 a 4 `cd` a puntos aleatorios.
    fn transf() -> impl Strategy<Value = Transf> {
        proptest::collection::vec(punto(), 1..5).prop_map(|ps| {
            ps.iter().fold(Transf::identidad(D), |t, p| t.enfocar(p))
        })
    }

    proptest! {
        #![proptest_config(ProptestConfig { cases: 2048, ..ProptestConfig::default() })]

        #[test]
        fn traslacion_es_isometria(a in punto(), x in punto(), y in punto()) {
            let d0 = distancia(&x, &y);
            let d1 = distancia(&suma(&a, &x), &suma(&a, &y));
            prop_assert!((d0 - d1).abs() <= 1e-9, "{d0} vs {d1}");
        }

        #[test]
        fn transf_compuesta_es_isometria(t in transf(), x in punto(), y in punto()) {
            let d0 = distancia(&x, &y);
            let d1 = distancia(&t.aplicar(&x), &t.aplicar(&y));
            prop_assert!((d0 - d1).abs() <= 1e-9, "{d0} vs {d1}");
        }

        #[test]
        fn bola_en_bola(a in punto(), x in punto(), t in transf()) {
            prop_assert!(norma(&suma(&a, &x)) < 1.0);
            prop_assert!(norma(&t.aplicar(&x)) < 1.0);
        }

        #[test]
        fn distancia_rapida_igual_definicion(x in punto(), y in punto()) {
            prop_assert!((distancia(&x, &y) - distancia_por_suma(&x, &y)).abs() <= 1e-9);
        }

        #[test]
        fn cancelacion_izquierda(a in punto(), b in punto()) {
            prop_assert!(cerca(&suma(&neg(&a), &suma(&a, &b)), &b, 1e-9));
        }

        #[test]
        fn giroasociatividad(a in punto(), b in punto(), c in punto()) {
            let izq = suma(&a, &suma(&b, &c));
            let der = suma(&suma(&a, &b), &gyr(&a, &b, &c));
            prop_assert!(cerca(&izq, &der, 1e-9));
        }

        #[test]
        fn gyr_formula_igual_definicion(a in punto(), b in punto(), c in punto()) {
            prop_assert!(cerca(&gyr(&a, &b, &c), &gyr_definicion(&a, &b, &c), 1e-9));
        }

        #[test]
        fn gyr_es_ortogonal(a in punto(), b in punto(), u in punto(), v in punto()) {
            prop_assert!((dot(&gyr(&a, &b, &u), &gyr(&a, &b, &v)) - dot(&u, &v)).abs() <= 1e-12);
        }

        #[test]
        fn conmutatividad_girada(a in punto(), b in punto()) {
            // a ⊕ b = gyr[a,b](b ⊕ a)
            prop_assert!(cerca(&suma(&a, &b), &gyr(&a, &b, &suma(&b, &a)), 1e-9));
        }

        #[test]
        fn exp_log_inversos(x in punto(), y in punto()) {
            prop_assert!(cerca(&exp(&x, &log(&x, &y)), &y, 1e-9));
            prop_assert!(cerca(&exp0(&log0(&y)), &y, 1e-12));
        }

        #[test]
        fn exp0_y_distancia(y in punto()) {
            // d(0, y) = 2 |log_0 y|
            prop_assert!((distancia_al_origen(&y) - 2.0 * norma(&log0(&y))).abs() <= 1e-9);
        }

        #[test]
        fn enfocar_lleva_al_origen(t in transf(), x in punto()) {
            let t2 = t.enfocar(&x);
            prop_assert!(norma(&t2.aplicar(&x)) <= 1e-9);
            prop_assert!(t2.error_ortogonal() <= 1e-9);
        }

        #[test]
        fn composicion_igual_aplicacion_sucesiva(t in transf(), a in punto(), z in punto()) {
            let directa = suma(&a, &t.aplicar(&z));
            let compuesta = t.trasladar_izq(&a).aplicar(&z);
            prop_assert!(cerca(&directa, &compuesta, 1e-9));
        }

        #[test]
        fn coincide_con_el_disco_complejo(a in punto_d(2), z in punto_d(2)) {
            // En d = 2, a ⊕ z = (a + z) / (1 + ā z) = Mobius::al_origen(−a)(z).
            let m = Mobius::al_origen(C::new(-a[0], -a[1]));
            let w = m.aplicar(C::new(z[0], z[1]));
            let s = suma(&a, &z);
            prop_assert!((w.re - s[0]).abs() <= 1e-12 && (w.im - s[1]).abs() <= 1e-12);
        }
    }

    #[test]
    fn identidad_y_neutro() {
        let x = vec![0.1, -0.2, 0.3];
        assert_eq!(suma(&[0.0; 3], &x), x);
        assert!(norma(&suma(&neg(&x), &x)) < 1e-15);
        assert_eq!(Transf::identidad(3).aplicar(&x), x);
    }
}
