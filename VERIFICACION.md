# VERIFICACIÓN — 02-poincare-fs

Generado: 2026-10-20 22:57 hora de Buenos Aires (UTC-3; el reloj del sistema está en UTC, por eso las salidas de ls muestran UTC), en la computadora compartida.

Comandos exactos y su salida real (sin editar, salvo donde se indica "extracto").

## Entorno
```text
$ uname -r; cargo --version; rustc --version; ls -l /dev/fuse /usr/bin/fusermount3 /etc/mtab
6.12.94+
cargo 1.85.1 (d73d2caf9 2024-12-31)
rustc 1.85.1 (4eb161250 2025-03-15) (built from a source tarball)
crw-rw-rw- 1 root root 10, 229 Oct 20 17:16 /dev/fuse
lrwxrwxrwx 1 root root      19 Oct 21 01:53 /etc/mtab -> ../proc/self/mounts
-rwsr-xr-x 1 root root   35128 Oct 21 01:35 /usr/bin/fusermount3
```

## Build
```text
$ cargo build --release
   Compiling poincare-fs v0.1.0 (/workspace/sistemas/02-poincare-fs)
    Finished `release` profile [optimized] target(s) in 1.22s
(código de salida: 0)
```

## Clippy
```text
$ cargo clippy --all-targets -- -D warnings
    Checking poincare-fs v0.1.0 (/workspace/sistemas/02-poincare-fs)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.36s
(código de salida: 0)
```

## cargo test — corrida 1
```text
$ cargo test

running 24 tests
test embed::tests::rasgos_basicos ... ok
test embed::tests::tfidf_normalizado ... ok
test embed::tests::corpus_trivial ... ok
test hiper::tests::al_origen_fuera_del_disco_falla - should panic ... ok
test hiper::tests::al_origen_manda_p_a_cero ... ok
test hiper::tests::distancia_al_origen_coincide ... ok
test hiper::tests::identidad ... ok
test hiper::tests::determinante_uno ... ok
test hiper::tests::composicion_es_aplicacion_sucesiva ... ok
test hiper::tests::borde_va_al_borde ... ok
test hiper::tests::inversa_deshace ... ok
test hiper::tests::lleva_el_disco_al_disco ... ok
test embed::tests::determinista ... ok
test hiper::tests::distancia_simetrica_y_triangular ... ok
test hiper::tests::preserva_distancia_hiperbolica ... ok
test proyeccion::tests::nombres_invalidos_y_repetidos ... ok
test embed::tests::todos_dentro_del_disco ... ok
test proyeccion::tests::distancias_de_la_vista_son_isometricas ... ok
test proyeccion::tests::cd_lleva_el_foco_al_origen ... ok
test proyeccion::tests::foco_texto_reporta_origen ... ok
test proyeccion::tests::determinista_para_la_misma_ruta ... ok
test hiper::tests::barrido_determinista_error_maximo ... ok
test proyeccion::tests::filtros_k_y_radio ... ok
test proyeccion::tests::raiz_ordenada_por_distancia ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.46s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-test-798647-ciclos
poincare-fs: desmontado
test rutas_largas_y_ciclos ... ok
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-test-798647-principal
poincare-fs: desmontado
test montar_cd_ls_cat_y_desmontar ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(código de salida: 0)
```

## cargo test — corrida 2
```text
$ cargo test

running 24 tests
test embed::tests::rasgos_basicos ... ok
test hiper::tests::al_origen_fuera_del_disco_falla - should panic ... ok
test embed::tests::tfidf_normalizado ... ok
test embed::tests::corpus_trivial ... ok
test hiper::tests::borde_va_al_borde ... ok
test hiper::tests::al_origen_manda_p_a_cero ... ok
test hiper::tests::identidad ... ok
test hiper::tests::determinante_uno ... ok
test hiper::tests::distancia_al_origen_coincide ... ok
test hiper::tests::composicion_es_aplicacion_sucesiva ... ok
test embed::tests::todos_dentro_del_disco ... ok
test hiper::tests::lleva_el_disco_al_disco ... ok
test hiper::tests::inversa_deshace ... ok
test hiper::tests::distancia_simetrica_y_triangular ... ok
test embed::tests::determinista ... ok
test proyeccion::tests::nombres_invalidos_y_repetidos ... ok
test hiper::tests::preserva_distancia_hiperbolica ... ok
test proyeccion::tests::cd_lleva_el_foco_al_origen ... ok
test proyeccion::tests::distancias_de_la_vista_son_isometricas ... ok
test proyeccion::tests::foco_texto_reporta_origen ... ok
test proyeccion::tests::raiz_ordenada_por_distancia ... ok
test proyeccion::tests::filtros_k_y_radio ... ok
test hiper::tests::barrido_determinista_error_maximo ... ok
test proyeccion::tests::determinista_para_la_misma_ruta ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.41s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-test-798719-ciclos
poincare-fs: desmontado
test rutas_largas_y_ciclos ... ok
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-test-798719-principal
poincare-fs: desmontado
test montar_cd_ls_cat_y_desmontar ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.70s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(código de salida: 0)
```

## cargo test — corrida 3
```text
$ cargo test

running 24 tests
test embed::tests::rasgos_basicos ... ok
test embed::tests::tfidf_normalizado ... ok
test hiper::tests::al_origen_fuera_del_disco_falla - should panic ... ok
test embed::tests::corpus_trivial ... ok
test hiper::tests::distancia_al_origen_coincide ... ok
test hiper::tests::borde_va_al_borde ... ok
test hiper::tests::identidad ... ok
test hiper::tests::determinante_uno ... ok
test hiper::tests::al_origen_manda_p_a_cero ... ok
test hiper::tests::composicion_es_aplicacion_sucesiva ... ok
test embed::tests::todos_dentro_del_disco ... ok
test hiper::tests::lleva_el_disco_al_disco ... ok
test hiper::tests::distancia_simetrica_y_triangular ... ok
test hiper::tests::preserva_distancia_hiperbolica ... ok
test hiper::tests::inversa_deshace ... ok
test proyeccion::tests::nombres_invalidos_y_repetidos ... ok
test embed::tests::determinista ... ok
test proyeccion::tests::cd_lleva_el_foco_al_origen ... ok
test proyeccion::tests::distancias_de_la_vista_son_isometricas ... ok
test proyeccion::tests::filtros_k_y_radio ... ok
test proyeccion::tests::foco_texto_reporta_origen ... ok
test proyeccion::tests::raiz_ordenada_por_distancia ... ok
test hiper::tests::barrido_determinista_error_maximo ... ok
test proyeccion::tests::determinista_para_la_misma_ruta ... ok

test result: ok. 24 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.40s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 2 tests
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-test-798788-ciclos
poincare-fs: desmontado
test rutas_largas_y_ciclos ... ok
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-test-798788-principal
poincare-fs: desmontado
test montar_cd_ls_cat_y_desmontar ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.68s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(código de salida: 0)
```

## Barrido determinista de 1 000 000 de transformaciones
```text
$ cargo test --release --lib barrido -- --nocapture
error_maximo_distancia=2.495781359357352e-13 max_|T(z)|=0.9985747620918385
test hiper::tests::barrido_determinista_error_maximo ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.20s
```

## Calidad semántica
```text
$ ./target/release/poincare-fs calidad corpus
corpus: corpus (18 documentos)
distancia_hiperbolica_media_mismo_tema: 0.662853
distancia_hiperbolica_media_distinto_tema: 1.893971
vecino_mas_cercano_mismo_tema_en_disco_2d: 10/18
vecino_mas_cercano_mismo_tema_en_tfidf_1024d: 16/18
```

## Coordenadas del corpus
```text
$ ./target/release/poincare-fs coords corpus
+0.070440222 +0.010290829 astro-agujeros-negros
-0.167444310 +0.231328584 astro-exoplanetas
+0.013803460 +0.172730180 astro-galaxias
+0.628065345 -0.010547685 cocina-empanadas
+0.806315919 -0.044010270 cocina-pan
+0.898075299 -0.058828202 cocina-pizza
-0.184229445 +0.517976186 geometria-euclides
-0.254076030 +0.723436231 geometria-mobius
-0.210201737 +0.566809378 geometria-poincare
-0.261837364 -0.666697113 kernel-ebpf
-0.140575654 -0.403167029 kernel-fuse
-0.432276602 -0.567780366 kernel-planificador
+0.150875123 -0.117337971 musica-folklore
-0.058081139 +0.112754867 musica-jazz
-0.041156630 -0.120790849 musica-tango
-0.243901468 -0.012014261 rust-async
-0.200721233 -0.325180383 rust-cargo
-0.373073758 -0.008972128 rust-ownership
```

## Sesión real montada (bash con `set -v`: cada comando aparece antes de su salida)

```text
$ ./target/release/poincare-fs montar corpus /tmp/poincare-demo &   # stderr del proceso:
poincare-fs: 18 documentos, k=10, radio=6, montando en /tmp/poincare-demo
poincare-fs: desmontado

$ bash sesion.sh
mount | grep /tmp/poincare-demo
poincarefs on /tmp/poincare-demo type fuse.poincare (ro,nosuid,nodev,relatime,user_id=1000,group_id=1000,default_permissions)
cd /tmp/poincare-demo
ls -U
astro-agujeros-negros
astro-agujeros-negros.txt
musica-jazz
musica-jazz.txt
musica-tango
musica-tango.txt
astro-galaxias
astro-galaxias.txt
musica-folklore
musica-folklore.txt
rust-async
rust-async.txt
astro-exoplanetas
astro-exoplanetas.txt
rust-ownership
rust-ownership.txt
rust-cargo
rust-cargo.txt
kernel-fuse
kernel-fuse.txt
cd musica-folklore
pwd
/tmp/poincare-demo/musica-folklore
ls -U | head -6
astro-agujeros-negros
astro-agujeros-negros.txt
musica-tango
musica-tango.txt
musica-jazz
musica-jazz.txt
cd cocina-empanadas
cat .foco
ruta: /musica-folklore/cocina-empanadas
profundidad: 2
foco: cocina-empanadas
foco_coordenada_original: (+0.628065345, -0.010547685)
foco_imagen: (-0.000000000, -0.000000000)  |imagen| = 4.289e-16
mobius: z -> (alfa*z + beta) / (conj(beta)*z + conj(alfa))
alfa: (+1.281131095, +0.102184798)
beta: (-0.805711857, -0.050665762)
det: 1.000000000000000
cat .vista
# ruta: /musica-folklore/cocina-empanadas
# rango distancia_hiperbolica posicion_transformada nombre
  1  0.771094492 (+0.366735045, -0.023934054) cocina-pan
  2  1.206076056 (-0.515163924, -0.159220656) musica-folklore
  3  1.336299149 (-0.578882934, -0.075311829) astro-agujeros-negros
  4  1.478953812 (+0.626778211, -0.050744518) cocina-pizza
  5  1.521825780 (-0.641461505, +0.014004324) astro-galaxias
  6  1.586143477 (-0.639929011, -0.162120760) musica-tango
  7  1.622569792 (-0.669496096, -0.032787262) musica-jazz
  8  1.920846642 (-0.744446748, +0.005302428) astro-exoplanetas
  9  1.974523007 (-0.747462854, -0.114568422) rust-async
 10  2.065919071 (-0.713706351, -0.302310719) kernel-fuse
cat cocina-pan.txt
Para el pan casero se mezcla harina, agua, sal y levadura, se amasa la masa, se deja levar y se hornea el pan hasta que la corteza quede dorada.
cd cocina-pizza
ls
astro-agujeros-negros
astro-agujeros-negros.txt
astro-exoplanetas
astro-exoplanetas.txt
astro-galaxias
astro-galaxias.txt
cocina-empanadas
cocina-empanadas.txt
cocina-pan
cocina-pan.txt
kernel-fuse
kernel-fuse.txt
musica-folklore
musica-folklore.txt
musica-jazz
musica-jazz.txt
musica-tango
musica-tango.txt
rust-async
rust-async.txt
cd /tmp/poincare-demo/cocina-pan
/tmp/verif/sesion.sh: line 14: cd: /tmp/poincare-demo/cocina-pan: No such file or directory
cd /
fusermount3 -u /tmp/poincare-demo
```

Después del desmontaje: `mount | grep -c /tmp/poincare-demo` → 0
