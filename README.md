# 02-poincare-fs

Sistema de archivos FUSE en Rust (crate `fuser` 0.15, solo lectura) donde los
directorios **no son un árbol**: cada directorio es una *proyección* de un
corpus de documentos sobre el **disco de Poincaré**, y hacer `cd <nombre>`
aplica una **transformación de Möbius** que lleva ese documento al origen.
`ls` muestra los documentos más cercanos al nuevo origen según la **distancia
hiperbólica**.

**Dueña:** Federica Ω Apex.

## Cómo funciona

1. **Embedding local y determinista** (`src/embed.rs`, sin red ni modelos):
   palabras + trigramas de caracteres → *hashing trick* con signo a 1024
   dimensiones → TF‑IDF normalizado → PCA a 2D (iteración de potencia con
   vector inicial fijo y signo fijado) → escala para que el documento más
   lejano quede en |z| = 0,9. Cada documento es un punto `z` del disco.
2. **Geometría** (`src/hiper.rs`): números complejos propios, distancia
   `d(z,w) = 2·artanh(|z−w| / |1−z̄w|)` y transformaciones de Möbius del disco
   como matrices de SU(1,1) `[[α, β], [β̄, ᾱ]]` (componer = multiplicar,
   inversa explícita, renormalización del determinante).
3. **Rutas = secuencia de `cd`** (`src/proyeccion.rs`): la ruta `/a/b` significa
   "desde la identidad, llevar `a` al origen; luego llevar la imagen de `b` al
   origen". La transformación acumulada depende solo de la ruta, por eso
   **`ls` es determinista para la misma posición** y la caché del kernel no
   queda inconsistente.
4. **Contenido de cada directorio**: los `k` documentos (por defecto 10) más
   cercanos al origen dentro de un radio hiperbólico (por defecto 6), sin el
   propio foco, ordenados por (distancia, nombre). Para cada uno aparecen:
   * `<nombre>/` → directorio: hacer `cd` aplica la Möbius;
   * `<nombre>.txt` → el contenido del documento (`cat`).
   * Además: `.vista` (tabla rango/distancia/posición transformada) y
     `.foco` (ruta, imagen del foco, coeficientes α y β, determinante).
   * El orden de `readdir` es el orden por distancia: se ve con `ls -U`
     (`ls` a secas ordena alfabéticamente).
   * Lo que no está en la proyección **no existe** ahí: `cd` a un documento
     lejano falla con `ENOENT` (hay que acercarse saltando).

## Cómo correrlo

Requisitos: Linux con `/dev/fuse` y `fusermount3` (paquete `fuse3`). No hace
falta `libfuse3-dev` (se compila `fuser` sin la feature `libfuse`).
`fusermount3 -u` necesita que exista `/etc/mtab`; en la computadora compartida
no existía y se creó el enlace estándar de Debian
`/etc/mtab -> ../proc/self/mounts` (con `sudo`).

```sh
cargo build --release
mkdir -p /tmp/disco
./target/release/poincare-fs montar corpus /tmp/disco &      # [--k N] [--radio R]
cd /tmp/disco && ls -U && cd musica-folklore && cd cocina-empanadas && cat .foco
cd / && fusermount3 -u /tmp/disco                            # el proceso termina solo

./target/release/poincare-fs coords corpus                   # coordenada de cada documento
./target/release/poincare-fs vista corpus musica-folklore    # la vista sin montar nada
./target/release/poincare-fs calidad corpus                  # cuánto conserva la semántica
```

El corpus de ejemplo (`corpus/`, 18 textos cortos en castellano de 6 temas:
rust, kernel, cocina, astro, música, geometría) se puede reemplazar por
cualquier directorio con archivos `*.txt` (no recursivo, ≤ 1 MiB cada uno,
≤ 10 000 archivos; se ignoran symlinks y nombres que empiezan con `.`).

## Resultados medidos (20‑21/10/2026, computadora compartida, kernel 6.12, Rust 1.85.1)

* `cargo build --release`: OK. `cargo clippy --all-targets -- -D warnings`: 0 avisos.
* `cargo test` corrido **3 veces**: las 3 veces **26 tests en verde**
  (24 unitarios/propiedades + 2 de integración con montaje real), 0 fallas.
  Detalle y salidas en [VERIFICACION.md](VERIFICACION.md).
* **Propiedades de Möbius** (proptest, 4096 casos aleatorios por propiedad,
  puntos con |z| ≤ 0,95): conserva la distancia hiperbólica con error
  **absoluto ≤ 1e‑9**; manda el disco al disco y el borde al borde;
  composición = aplicación sucesiva; la inversa deshace; det = 1 ± 1e‑12;
  `al_origen(p)` lleva `p` a < 1e‑12 del origen.
* **Barrido determinista** de 1 000 000 de casos (semilla fija): error máximo
  en la distancia **2,50e‑13**; máximo |T(z)| = 0,99857 (< 1).
* **Montaje real** (test `montar_cd_ls_cat_y_desmontar`): el listado de `ls -U`
  en la raíz y tras `cd musica-folklore/cocina-empanadas` coincide, en
  contenido y orden, con el cálculo sin FUSE; `.vista` montada es idéntica
  byte a byte al cálculo sin FUSE y entre lecturas; las distancias listadas
  coinciden (< 1e‑8, por imprimirse con 9 decimales) con la distancia
  hiperbólica original al foco; `|T(foco)|` = 4,3e‑16; `cd` a un documento no
  visible falla con `ENOENT`; escribir falla con `EROFS`; lectura por partes
  (`dd bs=7`) da el archivo exacto; `fusermount3 -u` desmonta y el proceso
  termina con código 0.
* **Rutas cíclicas y límite**: `/musica-tango/musica-jazz/musica-tango` lista
  lo mismo que `/musica-tango`; 64 `cd` anidados funcionan y el 65 falla
  (límite `MAX_PROFUNDIDAD`).
* **Calidad semántica** (`poincare-fs calidad corpus`, tema = prefijo del nombre):
  distancia hiperbólica media entre documentos del mismo tema **0,663** vs.
  **1,894** entre temas distintos; el vecino más cercano en el disco 2D es del
  mismo tema en **10 de 18** documentos (en el espacio TF‑IDF de 1024
  dimensiones, antes de proyectar: **16 de 18**). La proyección a 2D pierde
  información semántica de forma medible.

Sesión real (extracto; completa en VERIFICACION.md):

```text
$ cd /tmp/poincare-demo/musica-folklore/cocina-empanadas
$ cat .vista
# ruta: /musica-folklore/cocina-empanadas
# rango distancia_hiperbolica posicion_transformada nombre
  1  0.771094492 (+0.366735045, -0.023934054) cocina-pan
  2  1.206076056 (-0.515163924, -0.159220656) musica-folklore
  ...
$ cat cocina-pan.txt
Para el pan casero se mezcla harina, agua, sal y levadura, ...
```

## Qué NO está verificado

* **No hay io_uring.** La idea original decía "FUSE/io_uring"; esto es FUSE
  clásico por `/dev/fuse` con `fuser` 0.15 (un hilo). No se usa FUSE‑over‑io_uring.
* **No se midió rendimiento** (latencia de `lookup`/`readdir`, escala con
  corpus grandes). Cada operación recalcula la vista: O(N log N) por llamada;
  con 10 000 documentos y rutas largas puede ser lento. No probado.
* **La semántica es débil**: es un embedding por hashing + TF‑IDF + PCA a 2D,
  no un modelo de lenguaje. Con el corpus de ejemplo acierta el tema del
  vecino más cercano 10/18 veces; con otros corpus puede ser peor.
* Por la isometría, **el conjunto y el orden de `ls` dependen solo del último
  foco**, no del camino: el camino cambia la rotación acumulada (las
  posiciones transformadas y α/β en `.foco`), no las distancias. Es correcto
  matemáticamente, pero conviene saberlo.
* Precisión: las pruebas exigen 1e‑9 con |z| ≤ 0,95. Cerca del borde
  (|z| → 1) la distancia se vuelve numéricamente inestable; los documentos se
  ubican con |z| ≤ 0,9 a propósito.
* La tabla de inodos crece con cada ruta visitada (tope 2²⁰ rutas; después
  `lookup` devuelve `ENOENT`). No hay olvido/recolección. No probado el tope.
* El corpus se lee al montar; cambios posteriores en `corpus/` no se ven
  hasta volver a montar. Solo lectura.
* `cd ..` en bash funciona por la ruta lógica del shell; no se probaron
  herramientas que recorren todo (`find`, `du`): el "árbol" es infinito en
  profundidad (cortado a 64) y `find` sin `-maxdepth` recorrería muchísimo.
* Solo se probó en esta computadora (Debian 13, kernel 6.12, usuario sin
  `allow_other`). No se probó en la máquina de Federico.
