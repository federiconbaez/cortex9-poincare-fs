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
(commit clonado: 7f80fea)
test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.77s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 4.01s
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
(código de salida: 0)
```

## io_uring y respaldo pread
```text
$ POINCARE_IO=auto target/release/poincare-fs es-info corpus
modo_pedido=Auto modo_usado=Uring sonda_uring=ok archivos=18 bytes=2931 fnv=fee6ccf03c5dde07 segundos=0.0002
$ POINCARE_IO=uring target/release/poincare-fs es-info corpus
modo_pedido=Uring modo_usado=Uring sonda_uring=ok archivos=18 bytes=2931 fnv=fee6ccf03c5dde07 segundos=0.0001
$ POINCARE_IO=pread target/release/poincare-fs es-info corpus
modo_pedido=Pread modo_usado=Pread sonda_uring=ok archivos=18 bytes=2931 fnv=fee6ccf03c5dde07 segundos=0.0000
$ POINCARE_IO=auto ./target/release/poincare-fs es-info /tmp/poincare-bench-corpus/sint-10k
modo_pedido=Auto modo_usado=Uring sonda_uring=ok archivos=10000 bytes=4754335 fnv=06720202767fa6c7 segundos=0.0517
$ POINCARE_IO=pread ./target/release/poincare-fs es-info /tmp/poincare-bench-corpus/sint-10k
modo_pedido=Pread modo_usado=Pread sonda_uring=ok archivos=10000 bytes=4754335 fnv=06720202767fa6c7 segundos=0.0269
$ cargo test --release --test seccomp -- --nocapture   # binario con seccomp que bloquea io_uring_setup
test auto_usa_uring_y_con_seccomp_cae_a_pread ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

## Calidad: vecino más cercano del mismo tema
```text
$ ./target/release/poincare-fs calidad corpus
poincare-fs: 18 documentos, dim=16, E/S=Some(Uring), lectura=0.000s, embedding=0.100s, índice=no
documentos: 18  temas: 6
vecino_mismo_tema tfidf_1024d:            16/18
vecino_mismo_tema disco_2d_pca (v0.1):   10/18
vecino_mismo_tema bola_16d (RSGD):       15/18
vecino_mismo_tema bola->disco_2d (vista): 13/18
recall@5 vecinos_tfidf en la bola:      0.6778
norma en la bola: min 0.7505 max 0.9500
$ ./target/release/poincare-fs calidad /tmp/poincare-bench-corpus/stdlib-1k
poincare-fs: 1000 documentos, dim=16, E/S=Some(Uring), lectura=0.006s, embedding=0.897s, índice=no
documentos: 1000  temas: 27
vecino_mismo_tema tfidf_1024d:            835/1000
vecino_mismo_tema disco_2d_pca (v0.1):   269/1000
vecino_mismo_tema bola_16d (RSGD):       810/1000
vecino_mismo_tema bola->disco_2d (vista): 519/1000
recall@5 vecinos_tfidf en la bola:      0.5674
norma en la bola: min 0.6814 max 0.9500
$ ./target/release/poincare-fs calidad corpus --semilla-ajuste 1 | grep bola_16d
vecino_mismo_tema bola_16d (RSGD):       15/18
$ ./target/release/poincare-fs calidad corpus --semilla-ajuste 2 | grep bola_16d
vecino_mismo_tema bola_16d (RSGD):       14/18
$ ./target/release/poincare-fs calidad corpus --semilla-ajuste 3 | grep bola_16d
vecino_mismo_tema bola_16d (RSGD):       15/18
$ ./target/release/poincare-fs calidad corpus --semilla-ajuste 4 | grep bola_16d
vecino_mismo_tema bola_16d (RSGD):       15/18
$ ./target/release/poincare-fs calidad corpus --semilla-ajuste 5 | grep bola_16d
vecino_mismo_tema bola_16d (RSGD):       15/18
$ ./target/release/poincare-fs calidad corpus --dim 2 | grep 'bola_'
vecino_mismo_tema bola_2d (RSGD):       14/18
$ ./target/release/poincare-fs calidad corpus --dim 8 | grep 'bola_'
vecino_mismo_tema bola_8d (RSGD):       15/18
$ ./target/release/poincare-fs calidad corpus --dim 32 | grep 'bola_'
vecino_mismo_tema bola_32d (RSGD):       15/18
```

## Barrido determinista del disco 2D (garantía de la v0.1, se conserva)
```text
$ cargo test --release --lib barrido -- --nocapture
error_maximo_distancia=2.495781359357352e-13 max_|T(z)|=0.9985747620918385
test hiper::tests::barrido_determinista_error_maximo ... ok
test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 52 filtered out; finished in 0.21s
```

## Generación de corpus e índices
```text
$ ./target/release/poincare-fs generar stdlib 100000 /tmp/x-todo   # cuántos trozos reales hay
2079 documentos, 27 temas, fuente /usr/lib/python3.13 -> /tmp/x-todo
$ ./target/release/poincare-fs indexar /tmp/poincare-bench-corpus/c18 /tmp/poincare-bench-corpus/c18.idx
poincare-fs: 18 documentos, dim=16, E/S=Some(Uring), lectura=0.000s, embedding=0.000s, índice=cargado de /tmp/poincare-bench-corpus/c18.idx
$ ./target/release/poincare-fs indexar /tmp/poincare-bench-corpus/stdlib-1k /tmp/poincare-bench-corpus/stdlib-1k.idx
poincare-fs: 1000 documentos, dim=16, E/S=Some(Uring), lectura=0.003s, embedding=0.001s, índice=cargado de /tmp/poincare-bench-corpus/stdlib-1k.idx
$ ./target/release/poincare-fs indexar /tmp/poincare-bench-corpus/sint-10k /tmp/poincare-bench-corpus/sint-10k.idx
poincare-fs: 10000 documentos, dim=16, E/S=Some(Uring), lectura=0.038s, embedding=0.009s, índice=cargado de /tmp/poincare-bench-corpus/sint-10k.idx
```

## Benchmark (salida completa de `sh bench/correr.sh`, también en bench/resultados-2026-10-20.txt)
```text
1000 documentos, 27 temas, fuente /usr/lib/python3.13 -> /tmp/poincare-bench-corpus/stdlib-1k
10000 documentos, 27 temas, fuente /usr/lib/python3.13 -> /tmp/poincare-bench-corpus/sint-10k
poincare-fs: 18 documentos, dim=16, E/S=Some(Uring), lectura=0.000s, embedding=0.258s, índice=escrito en /tmp/poincare-bench-corpus/c18.idx
poincare-fs: 1000 documentos, dim=16, E/S=Some(Uring), lectura=0.007s, embedding=2.014s, índice=escrito en /tmp/poincare-bench-corpus/stdlib-1k.idx
poincare-fs: 10000 documentos, dim=16, E/S=Some(Uring), lectura=0.081s, embedding=13.298s, índice=escrito en /tmp/poincare-bench-corpus/sint-10k.idx
carga antes: 23.21 21.33 15.27 13/2010 960669
## c18 cache=4096 repetición 1
corpus=/tmp/poincare-bench-corpus/c18 pasos=2000 cache=4096 montaje=0.013s caminata=0.690s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 101.7 | 4899.0 | 217.6 | 4595 |
| ls (repetido) | 2000 | 26.1 | 990.3 | 72.5 | 13788 |
| cd | 1940 | 9.1 | 23.0 | 16.7 | 59780 |
| cat | 2000 | 20.5 | 53.7 | 37.4 | 26758 |
## c18 cache=4096 repetición 2
corpus=/tmp/poincare-bench-corpus/c18 pasos=2000 cache=4096 montaje=0.020s caminata=0.959s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 110.1 | 5755.3 | 334.7 | 2988 |
| ls (repetido) | 2000 | 26.3 | 1063.4 | 71.8 | 13919 |
| cd | 1940 | 9.3 | 86.0 | 23.1 | 43260 |
| cat | 2000 | 20.5 | 437.9 | 49.1 | 20381 |
## c18 cache=4096 repetición 3
corpus=/tmp/poincare-bench-corpus/c18 pasos=2000 cache=4096 montaje=0.011s caminata=0.625s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 103.2 | 4699.0 | 219.3 | 4561 |
| ls (repetido) | 2000 | 26.4 | 63.9 | 42.0 | 23820 |
| cd | 1940 | 8.9 | 21.0 | 16.7 | 60043 |
| cat | 2000 | 20.4 | 39.3 | 33.7 | 29690 |
## c18 cache=0 (sin LRU)
corpus=/tmp/poincare-bench-corpus/c18 pasos=2000 cache=0 montaje=0.013s caminata=1.564s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 161.9 | 3879.2 | 252.3 | 3963 |
| ls (repetido) | 2000 | 153.9 | 4690.0 | 273.3 | 3659 |
| cd | 1940 | 71.0 | 155.1 | 80.8 | 12373 |
| cat | 2000 | 83.8 | 3978.9 | 176.7 | 5661 |
## stdlib-1k cache=4096 repetición 1
corpus=/tmp/poincare-bench-corpus/stdlib-1k pasos=2000 cache=4096 montaje=0.021s caminata=4.391s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 1167.9 | 9304.6 | 1958.9 | 510 |
| ls (repetido) | 2000 | 28.0 | 4025.3 | 140.8 | 7103 |
| cd | 1940 | 10.2 | 93.9 | 33.1 | 30237 |
| cat | 2000 | 24.5 | 561.4 | 61.5 | 16257 |
## stdlib-1k cache=4096 repetición 2
corpus=/tmp/poincare-bench-corpus/stdlib-1k pasos=2000 cache=4096 montaje=0.035s caminata=2.839s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 898.2 | 6885.4 | 1311.7 | 762 |
| ls (repetido) | 2000 | 25.9 | 484.3 | 60.9 | 16430 |
| cd | 1940 | 9.6 | 28.2 | 11.6 | 86233 |
| cat | 2000 | 21.8 | 74.1 | 34.2 | 29265 |
## stdlib-1k cache=4096 repetición 3
corpus=/tmp/poincare-bench-corpus/stdlib-1k pasos=2000 cache=4096 montaje=0.023s caminata=2.898s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 940.0 | 7054.3 | 1335.4 | 749 |
| ls (repetido) | 2000 | 27.1 | 120.7 | 47.5 | 21047 |
| cd | 1940 | 11.3 | 75.6 | 26.4 | 37921 |
| cat | 2000 | 24.0 | 108.7 | 38.6 | 25889 |
## stdlib-1k cache=0 (sin LRU)
corpus=/tmp/poincare-bench-corpus/stdlib-1k pasos=2000 cache=0 montaje=0.018s caminata=18.495s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 2058.1 | 11229.1 | 3136.1 | 319 |
| ls (repetido) | 2000 | 1964.3 | 10502.7 | 3044.5 | 328 |
| cd | 1940 | 770.3 | 4662.4 | 846.3 | 1182 |
| cat | 2000 | 1006.1 | 9783.9 | 2242.9 | 446 |
## sint-10k cache=4096 repetición 1
corpus=/tmp/poincare-bench-corpus/sint-10k pasos=2000 cache=4096 montaje=0.130s caminata=13.923s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 6660.3 | 18959.5 | 6791.9 | 147 |
| ls (repetido) | 2000 | 40.4 | 1872.8 | 81.4 | 12281 |
| cd | 1940 | 13.1 | 52.0 | 26.9 | 37210 |
| cat | 2000 | 37.8 | 247.0 | 58.7 | 17025 |
## sint-10k cache=4096 repetición 2
corpus=/tmp/poincare-bench-corpus/sint-10k pasos=2000 cache=4096 montaje=0.129s caminata=14.974s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 7067.9 | 23826.3 | 7296.4 | 137 |
| ls (repetido) | 2000 | 41.8 | 1804.8 | 86.8 | 11526 |
| cd | 1940 | 13.2 | 38.8 | 27.2 | 36796 |
| cat | 2000 | 38.8 | 479.1 | 74.4 | 13432 |
## sint-10k cache=4096 repetición 3
corpus=/tmp/poincare-bench-corpus/sint-10k pasos=2000 cache=4096 montaje=0.232s caminata=18.995s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 2000 | 8001.5 | 30088.6 | 9287.7 | 108 |
| ls (repetido) | 2000 | 44.3 | 2903.7 | 120.2 | 8319 |
| cd | 1940 | 13.7 | 104.5 | 22.5 | 44376 |
| cat | 2000 | 40.8 | 483.3 | 64.2 | 15568 |
## sint-10k cache=0 (sin LRU)
corpus=/tmp/poincare-bench-corpus/sint-10k pasos=300 cache=0 montaje=0.191s caminata=16.060s
| operación | n | p50 (µs) | p99 (µs) | media (µs) | ops/s |
|---|---:|---:|---:|---:|---:|
| ls (vista nueva) | 300 | 15304.9 | 48002.0 | 17143.6 | 58 |
| ls (repetido) | 300 | 15808.8 | 45892.4 | 17799.6 | 56 |
| cd | 291 | 7673.6 | 36094.7 | 8937.2 | 112 |
| cat | 300 | 7954.3 | 32746.5 | 9911.4 | 101 |
carga después: 20.58 20.61 15.58 22/1605 1021894
```

## Sesión real montada (bash con `set -v`: cada comando aparece antes de su salida)

```text
$ ./target/release/poincare-fs montar corpus /tmp/poincare-demo &   # stderr del proceso:
poincare-fs: 18 documentos, dim=16, E/S=Some(Uring), lectura=0.000s, embedding=0.108s, índice=no
poincare-fs: k=10 radio=12 memoria=0.5 cache=4096; montando en /tmp/poincare-demo
poincare-fs: desmontado

$ bash sesion.sh
M=/tmp/poincare-demo
mount | grep $M
poincarefs on /tmp/poincare-demo type fuse.poincare (ro,nosuid,nodev,relatime,user_id=1000,group_id=1000,default_permissions)
cd $M
ls -U
astro-exoplanetas
astro-exoplanetas.txt
kernel-ebpf
kernel-ebpf.txt
geometria-euclides
geometria-euclides.txt
astro-galaxias
astro-galaxias.txt
musica-tango
musica-tango.txt
astro-agujeros-negros
astro-agujeros-negros.txt
geometria-mobius
geometria-mobius.txt
rust-async
rust-async.txt
cocina-pizza
cocina-pizza.txt
rust-ownership
rust-ownership.txt
cd kernel-ebpf
cd kernel-planificador
pwd
/tmp/poincare-demo/kernel-ebpf/kernel-planificador
ls -U
kernel-ebpf
kernel-ebpf.txt
kernel-fuse
kernel-fuse.txt
rust-async
rust-async.txt
rust-cargo
rust-cargo.txt
astro-exoplanetas
astro-exoplanetas.txt
rust-ownership
rust-ownership.txt
astro-agujeros-negros
astro-agujeros-negros.txt
musica-tango
musica-tango.txt
geometria-euclides
geometria-euclides.txt
geometria-mobius
geometria-mobius.txt
cat .foco
ruta: /kernel-ebpf/kernel-planificador
profundidad: 2
dimension_bola: 16
foco: kernel-planificador
foco_imagen_norma: 6.851e-15
transformacion: z -> t (+) R z   (suma de Möbius, R = giración acumulada)
t_norma: 0.950000000
R_error_ortogonal: 2.220e-16
memoria_gamma: 0.5
trayectoria (peso, distancia_al_foco_actual, nombre):
  0.142857 3.663561646 (origen)
  0.285714 1.623619835 kernel-ebpf
  0.571429 0.000000000 kernel-planificador
cat .vista
# ruta: /kernel-ebpf/kernel-planificador
# rango puntaje_trayectoria distancia_al_foco posicion_disco_2d nombre
  1  1.280625381  1.623619835 (-0.138197, +0.591304) kernel-ebpf
  2  2.415142487  2.195093593 (+0.474964, +0.482672) kernel-fuse
  3  4.403202619  5.062897812 (-0.039768, +0.979862) rust-async
  4  4.437434743  5.049121084 (-0.072362, +0.978078) rust-cargo
  5  4.531577401  5.375665676 (+0.060796, +0.983277) astro-exoplanetas
  6  4.534164757  5.190247913 (-0.042162, +0.981902) rust-ownership
  7  4.817898623  5.472115333 (+0.149973, +0.972565) astro-agujeros-negros
  8  5.290692493  6.066228667 (+0.094965, +0.988085) musica-tango
  9  5.377467628  6.215921957 (+0.053142, +0.991465) geometria-euclides
 10  5.593041451  6.431710101 (+0.053236, +0.992711) geometria-mobius
cat kernel-fuse.txt
FUSE deja implementar un sistema de archivos en espacio de usuario: el kernel Linux reenvía lookup, getattr, readdir y read a un proceso por /dev/fuse.
cat $M/astro-galaxias/astro-exoplanetas/.vista
# ruta: /astro-galaxias/astro-exoplanetas
# rango puntaje_trayectoria distancia_al_foco posicion_disco_2d nombre
  1  2.569443315  3.776138668 (+0.874680, +0.319809) astro-galaxias
  2  2.834778958  1.915806039 (-0.198065, +0.145640) geometria-euclides
  3  3.068120814  2.170938658 (-0.197058, +0.188420) geometria-mobius
  4  3.719716551  2.988039830 (+0.052919, +0.536799) geometria-poincare
  5  4.167860119  3.897466252 (+0.927673, -0.234217) musica-tango
  6  4.227408306  4.135009776 (+0.897908, -0.250832) kernel-ebpf
  7  4.575668282  4.936304399 (+0.972783, +0.097403) cocina-pizza
  8  4.590357093  4.298307416 (+0.944603, -0.226579) musica-jazz
  9  4.662073807  4.350209555 (+0.830567, -0.412499) rust-async
 10  4.743403441  5.103716035 (+0.975895, +0.098420) cocina-empanadas
cat $M/kernel-ebpf/astro-exoplanetas/.vista
# ruta: /kernel-ebpf/astro-exoplanetas
# rango puntaje_trayectoria distancia_al_foco posicion_disco_2d nombre
  1  2.715705347  4.135009776 (-0.761379, -0.598231) kernel-ebpf
  2  2.920258532  1.915806039 (-0.450272, +0.120046) geometria-euclides
  3  3.158314913  2.170938658 (-0.494685, +0.102027) geometria-mobius
  4  3.885462491  2.988039830 (-0.510823, -0.255654) geometria-poincare
  5  3.995952186  4.350209555 (-0.858738, -0.460232) rust-async
  6  4.051399685  3.897466252 (-0.577612, -0.758849) musica-tango
  7  4.059066288  5.375665676 (-0.768037, -0.625840) kernel-planificador
  8  4.081146274  3.776138668 (-0.376508, -0.869176) astro-galaxias
  9  4.151220589  4.520095619 (-0.860590, -0.465176) rust-ownership
 10  4.267352416  5.436460970 (-0.755032, -0.642269) kernel-fuse
cd $M/cocina-pan
/tmp/verif/sesion2.sh: line 15: cd: /tmp/poincare-demo/cocina-pan: No such file or directory
echo "hola" > $M/nuevo.txt
/tmp/verif/sesion2.sh: line 16: /tmp/poincare-demo/nuevo.txt: Read-only file system
cd /
fusermount3 -u $M
```

Después del desmontaje: `mount | grep -c /tmp/poincare-demo` → 0
