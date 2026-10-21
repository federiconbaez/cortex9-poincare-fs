# VERIFICACIÓN — 02-poincare-fs (v0.2)

Generado: 2026-10-20 23:31 hora de Buenos Aires (UTC-3), en la computadora compartida. El reloj del sistema está en UTC: las salidas de `ls -l`/`date` muestran UTC.

Comandos exactos y su salida real. Donde se filtró la salida (p. ej. sin líneas `Compiling`), el comando lo muestra.

## Entorno
```text
$ uname -r; nproc; cat /proc/loadavg; cargo --version; rustc --version; cat /proc/sys/kernel/io_uring_disabled; grep Seccomp: /proc/self/status; ls /sys/module/fuse/parameters/; ls /proc/config.gz; ls -l /dev/fuse /usr/bin/fusermount3 /etc/mtab
6.12.94+
8
17.40 19.69 15.78 13/1546 1027692
cargo 1.85.1 (d73d2caf9 2024-12-31)
rustc 1.85.1 (4eb161250 2025-03-15) (built from a source tarball)
0
Seccomp:	0
allow_sys_admin_access
max_user_bgreq
max_user_congthresh
ls: cannot access '/proc/config.gz': No such file or directory
crw-rw-rw- 1 root root 10, 229 Oct 20 17:16 /dev/fuse
lrwxrwxrwx 1 root root      19 Oct 21 01:53 /etc/mtab -> ../proc/self/mounts
-rwsr-xr-x 1 root root   35128 Oct 21 01:35 /usr/bin/fusermount3
```

`enable_uring` no figura entre los parámetros del módulo fuse: no hay FUSE‑over‑io_uring en este kernel (6.12; llegó en 6.14).

## Build
```text
$ cargo build --release
   Compiling poincare-fs v0.1.0 (/workspace/sistemas/02-poincare-fs)
    Finished `release` profile [optimized] target(s) in 7.74s
(código de salida: 0)
```

## Clippy
```text
$ cargo clippy --all-targets -- -D warnings
    Checking poincare-fs v0.1.0 (/workspace/sistemas/02-poincare-fs)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 1.06s
(código de salida: 0)
```

## cargo test — corrida 1
```text
$ cargo test 2>&1 | grep -v -E "^\s+(Compiling|Running|Finished|Doc-tests)"

running 53 tests
test ajuste::tests::disco_no_aleja ... ok
test ajuste::tests::gradiente_coincide_con_diferencias_finitas ... ok
test ajuste::tests::ajuste_determinista_y_dentro_de_la_bola ... ok
test ajuste::tests::vecinos_exactos ... ok
test bola::tests::coincide_con_el_disco_complejo ... ok
test bola::tests::exp0_y_distancia ... ok
test bola::tests::cancelacion_izquierda ... ok
test bola::tests::distancia_rapida_igual_definicion ... ok
test bola::tests::conmutatividad_girada ... ok
test bola::tests::identidad_y_neutro ... ok
test bola::tests::exp_log_inversos ... ok
test bola::tests::gyr_es_ortogonal ... ok
test embed::tests::corpus_trivial ... ok
test bola::tests::giroasociatividad ... ok
test embed::tests::rasgos_basicos ... ok
test embed::tests::tfidf_normalizado ... ok
test bola::tests::gyr_formula_igual_definicion ... ok
test bola::tests::traslacion_es_isometria ... ok
test es::tests::auto_y_error_de_archivo ... ok
test embed::tests::determinista ... ok
test generar::tests::extrae_comentarios_y_docstrings ... ok
test generar::tests::repartir_es_uniforme_y_determinista ... ok
test hiper::tests::al_origen_fuera_del_disco_falla - should panic ... ok
test es::tests::pread_lee_exacto ... ok
test embed::tests::todos_dentro_del_disco ... ok
test es::tests::uring_lee_exacto ... ok
test hiper::tests::al_origen_manda_p_a_cero ... ok
test hiper::tests::borde_va_al_borde ... ok
test hiper::tests::determinante_uno ... ok
test hiper::tests::distancia_al_origen_coincide ... ok
test hiper::tests::identidad ... ok
test hiper::tests::inversa_deshace ... ok
test hiper::tests::distancia_simetrica_y_triangular ... ok
test hiper::tests::composicion_es_aplicacion_sucesiva ... ok
test hiper::tests::lleva_el_disco_al_disco ... ok
test hiper::tests::preserva_distancia_hiperbolica ... ok
test proyeccion::tests::cd_lleva_el_foco_al_origen ... ok
test hiper::tests::barrido_determinista_error_maximo ... ok
test proyeccion::tests::el_camino_importa ... ok
test proyeccion::tests::distancias_de_la_vista_son_isometricas ... ok
test bola::tests::transf_compuesta_es_isometria ... ok
test proyeccion::tests::determinista_para_la_misma_ruta ... ok
test proyeccion::tests::nombres_invalidos_y_repetidos ... ok
test proyeccion::tests::pesos_normalizados ... ok
test bola::tests::bola_en_bola ... ok
test proyeccion::tests::filtros_k_y_radio ... ok
test proyeccion::tests::foco_texto_reporta_origen ... ok
test proyeccion::tests::seleccion_k_igual_a_orden_completo ... ok
test proyeccion::tests::memoria_cero_es_la_semantica_anterior ... ok
test proyeccion::tests::raiz_ordenada_por_distancia ... ok
test bola::tests::enfocar_lleva_al_origen ... ok
test bola::tests::composicion_igual_aplicacion_sucesiva ... ok
test proyeccion::tests::indice_ida_y_vuelta_y_rechazo ... ok

test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.23s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 3 tests
test montaje_con_pread_es_identico ... ok
test montar_cd_ls_cat_y_desmontar ... ok
test el_camino_importa_montado ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 7.33s


running 1 test
test auto_usa_uring_y_con_seccomp_cae_a_pread ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(código de salida: 0)
```

## cargo test — corrida 2
```text
$ cargo test 2>&1 | grep -v -E "^\s+(Compiling|Running|Finished|Doc-tests)"

running 53 tests
test ajuste::tests::disco_no_aleja ... ok
test ajuste::tests::gradiente_coincide_con_diferencias_finitas ... ok
test ajuste::tests::ajuste_determinista_y_dentro_de_la_bola ... ok
test ajuste::tests::vecinos_exactos ... ok
test bola::tests::coincide_con_el_disco_complejo ... ok
test bola::tests::conmutatividad_girada ... ok
test bola::tests::exp0_y_distancia ... ok
test bola::tests::cancelacion_izquierda ... ok
test bola::tests::distancia_rapida_igual_definicion ... ok
test bola::tests::identidad_y_neutro ... ok
test bola::tests::exp_log_inversos ... ok
test bola::tests::giroasociatividad ... ok
test embed::tests::corpus_trivial ... ok
test bola::tests::gyr_formula_igual_definicion ... ok
test embed::tests::rasgos_basicos ... ok
test embed::tests::tfidf_normalizado ... ok
test bola::tests::gyr_es_ortogonal ... ok
test bola::tests::traslacion_es_isometria ... ok
test embed::tests::todos_dentro_del_disco ... ok
test embed::tests::determinista ... ok
test generar::tests::extrae_comentarios_y_docstrings ... ok
test generar::tests::repartir_es_uniforme_y_determinista ... ok
test hiper::tests::al_origen_fuera_del_disco_falla - should panic ... ok
test es::tests::auto_y_error_de_archivo ... ok
test es::tests::pread_lee_exacto ... ok
test es::tests::uring_lee_exacto ... ok
test hiper::tests::al_origen_manda_p_a_cero ... ok
test hiper::tests::borde_va_al_borde ... ok
test hiper::tests::composicion_es_aplicacion_sucesiva ... ok
test hiper::tests::determinante_uno ... ok
test hiper::tests::identidad ... ok
test hiper::tests::distancia_al_origen_coincide ... ok
test hiper::tests::distancia_simetrica_y_triangular ... ok
test hiper::tests::inversa_deshace ... ok
test hiper::tests::lleva_el_disco_al_disco ... ok
test bola::tests::bola_en_bola ... ok
test hiper::tests::preserva_distancia_hiperbolica ... ok
test hiper::tests::barrido_determinista_error_maximo ... ok
test proyeccion::tests::cd_lleva_el_foco_al_origen ... ok
test proyeccion::tests::distancias_de_la_vista_son_isometricas ... ok
test proyeccion::tests::el_camino_importa ... ok
test proyeccion::tests::filtros_k_y_radio ... ok
test proyeccion::tests::nombres_invalidos_y_repetidos ... ok
test proyeccion::tests::pesos_normalizados ... ok
test proyeccion::tests::foco_texto_reporta_origen ... ok
test proyeccion::tests::determinista_para_la_misma_ruta ... ok
test bola::tests::enfocar_lleva_al_origen ... ok
test proyeccion::tests::seleccion_k_igual_a_orden_completo ... ok
test proyeccion::tests::memoria_cero_es_la_semantica_anterior ... ok
test bola::tests::transf_compuesta_es_isometria ... ok
test proyeccion::tests::raiz_ordenada_por_distancia ... ok
test bola::tests::composicion_igual_aplicacion_sucesiva ... ok
test proyeccion::tests::indice_ida_y_vuelta_y_rechazo ... ok

test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 3.11s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 3 tests
test montar_cd_ls_cat_y_desmontar ... ok
test montaje_con_pread_es_identico ... ok
test el_camino_importa_montado ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 8.17s


running 1 test
test auto_usa_uring_y_con_seccomp_cae_a_pread ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(código de salida: 0)
```

## cargo test — corrida 3
```text
$ cargo test 2>&1 | grep -v -E "^\s+(Compiling|Running|Finished|Doc-tests)"

running 53 tests
test ajuste::tests::gradiente_coincide_con_diferencias_finitas ... ok
test ajuste::tests::disco_no_aleja ... ok
test ajuste::tests::vecinos_exactos ... ok
test ajuste::tests::ajuste_determinista_y_dentro_de_la_bola ... ok
test bola::tests::coincide_con_el_disco_complejo ... ok
test bola::tests::exp0_y_distancia ... ok
test bola::tests::conmutatividad_girada ... ok
test bola::tests::cancelacion_izquierda ... ok
test bola::tests::distancia_rapida_igual_definicion ... ok
test bola::tests::identidad_y_neutro ... ok
test bola::tests::giroasociatividad ... ok
test bola::tests::exp_log_inversos ... ok
test embed::tests::corpus_trivial ... ok
test bola::tests::gyr_formula_igual_definicion ... ok
test embed::tests::rasgos_basicos ... ok
test embed::tests::tfidf_normalizado ... ok
test embed::tests::todos_dentro_del_disco ... ok
test bola::tests::gyr_es_ortogonal ... ok
test bola::tests::traslacion_es_isometria ... ok
test embed::tests::determinista ... ok
test generar::tests::extrae_comentarios_y_docstrings ... ok
test generar::tests::repartir_es_uniforme_y_determinista ... ok
test hiper::tests::al_origen_fuera_del_disco_falla - should panic ... ok
test es::tests::uring_lee_exacto ... ok
test es::tests::pread_lee_exacto ... ok
test es::tests::auto_y_error_de_archivo ... ok
test hiper::tests::al_origen_manda_p_a_cero ... ok
test hiper::tests::borde_va_al_borde ... ok
test hiper::tests::determinante_uno ... ok
test hiper::tests::composicion_es_aplicacion_sucesiva ... ok
test hiper::tests::identidad ... ok
test hiper::tests::distancia_al_origen_coincide ... ok
test hiper::tests::inversa_deshace ... ok
test hiper::tests::lleva_el_disco_al_disco ... ok
test hiper::tests::distancia_simetrica_y_triangular ... ok
test hiper::tests::preserva_distancia_hiperbolica ... ok
test hiper::tests::barrido_determinista_error_maximo ... ok
test proyeccion::tests::cd_lleva_el_foco_al_origen ... ok
test proyeccion::tests::distancias_de_la_vista_son_isometricas ... ok
test bola::tests::composicion_igual_aplicacion_sucesiva ... ok
test bola::tests::bola_en_bola ... ok
test proyeccion::tests::el_camino_importa ... ok
test proyeccion::tests::nombres_invalidos_y_repetidos ... ok
test proyeccion::tests::pesos_normalizados ... ok
test proyeccion::tests::filtros_k_y_radio ... ok
test proyeccion::tests::determinista_para_la_misma_ruta ... ok
test proyeccion::tests::foco_texto_reporta_origen ... ok
test proyeccion::tests::memoria_cero_es_la_semantica_anterior ... ok
test bola::tests::transf_compuesta_es_isometria ... ok
test proyeccion::tests::raiz_ordenada_por_distancia ... ok
test proyeccion::tests::seleccion_k_igual_a_orden_completo ... ok
test bola::tests::enfocar_lleva_al_origen ... ok
test proyeccion::tests::indice_ida_y_vuelta_y_rechazo ... ok

test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s


running 3 tests
test montaje_con_pread_es_identico ... ok
test montar_cd_ls_cat_y_desmontar ... ok
test el_camino_importa_montado ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.00s


running 1 test
test auto_usa_uring_y_con_seccomp_cae_a_pread ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s


running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

(código de salida: 0)
```

## Clon nuevo
```text
$ git clone <repo> /tmp/clon-pfs && cd /tmp/clon-pfs && cargo test 2>&1 | grep "test result"
