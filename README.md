# 02-poincare-fs

> **ES:** Sistema de archivos FUSE en Rust (solo lectura) que organiza documentos con embeddings en geometría hiperbólica (bola de Poincaré). Compilar/probar: `cargo build --release && cargo test --release`.
> **EN:** Read-only Rust FUSE filesystem that organizes documents with hyperbolic (Poincaré ball) embeddings. Build/test: `cargo build --release && cargo test --release`.
>
> **Estado / Status (AUDIT.md, AUDIT-valor-v03.md):** **FALLA el umbral #2 / FAILS threshold #2 — «valor no demostrado».** Como FS navegable pierde en recall@1 contra ripgrep y FAISS. En v0.3, el diseño pre-registrado no pasa completo: `@buscar` híbrido pasa en caliente, `@similar` (bola pura) falla el recall@1 contra FAISS (78,0 % vs 82,5 %). Licencia / License: Apache-2.0.


Sistema de archivos FUSE en Rust (crate `fuser` 0.15, solo lectura) donde los
directorios **no son un árbol**: cada directorio es una *proyección* de un
corpus de documentos sobre la **bola de Poincaré de dimensión d** (16 por
defecto). Hacer `cd <nombre>` aplica una **traslación de Möbius a izquierda**
que lleva ese documento al origen. `ls` muestra los documentos más cercanos
según la distancia hiperbólica, ponderada por la **trayectoria** (el camino
de `cd` que hiciste).

**Dueña:** Federica Ω Apex. Versión 0.3 (sobre la 0.2, commit `849983a`).

## Geometría en datos jerárquicos (21/10/2026)

Experimento pre-registrado (`bench/geometria/PREREGISTRO-geometria.md`, commit `1403985`):
WordNet 3.0 (4000 synsets) y `linux-6.12/Documentation` (3071 `.rst`), H vs E nativa en
d ∈ {2,4,8,16}, misma pérdida/épocas/grilla/semillas. **Veredicto:** H no gana en recall@1
de `@similar` en ninguna celda (regla: ≥+2 pp con IC95>0 en 3 semillas). La distorsión del
árbol sí favorece a H en 2–4d (confirmación publicada). Detalle: `bench/geometria/RESULTADOS-geometria.md`.

## v0.3: acceso directo y alcance completo (21/10/2026)

Esta versión responde a la falla de valor de la v0.2. Resultados, protocolo con conjunto reservado y veredicto por celda: **[VALOR.md](VALOR.md)**.

- **Directorios mágicos.** Se resuelven desde cualquier directorio pero no aparecen en `ls`:
  - `cd @doc/<nombre>`: salta directo a la vista de ese documento.
  - `ls -U @similar/<nombre>`: los 10 vecinos en la bola (bola pura).
  - `ls -U @similar-hibrido/<nombre>`: el primero es el vecino TF‑IDF y el resto sale reordenado por la bola.
  - `ls -U "@buscar/<consulta>"`: híbrido léxico → bola.
  - `ls -U "@buscar-bola/<consulta>"`: ordenado solo por la bola.
  - Cada uno trae un `.resultados` con los puntajes. Cada `ls` recalcula, sin caché de resultados.
- **Alcance completo.** Un árbol de cobertura cuelga los documentos que no son vecinos de nadie del documento alcanzado más cercano. Esos hijos aparecen en la vista de su padre, sin importar γ.
  - En stdlib‑1000 se alcanzan **1000/1000** recorriendo el FS montado con `cd` y `ls` (antes, 651).
- **Velocidad.** La bola usa f32 por dimensión con factores conformes precalculados. Primero filtra en f32 y después reordena exacto en f64.
  - El índice léxico es TF‑IDF por columnas con consulta dispersa.
  - Los vecinos TF‑IDF se precalculan al montar si n ≤ 5000. Montar 1000 documentos tarda ~1 s.
- **Veredicto, resumido:**
  - El diseño pre-registrado pasa en T2' con `@buscar`, pero falla en T1' con `@similar`: la bola pura tiene menos recall@1 que FAISS (78,0 contra 82,5 %).
  - El híbrido post-hoc `@similar-hibrido` pasa en T1''.
  - Los dos híbridos empatan con FAISS en recall@1 por construcción y le ganan en P@10.
  - Con consultas intercaladas, T2' no cumple el tiempo (0,83×).
- **Tests:**
  - `cargo test --release` da **68 en verde**: 62 de la biblioteca, 3 de montaje, 2 de acceso directo montado y 1 de seccomp. Los 58 anteriores no se tocaron.
  - `cargo clippy --all-targets -- -D warnings` da 0 avisos.

## Qué cambió respecto de la 0.1

| | 0.1 | 0.2 |
|---|---|---|
| Espacio | disco 2D (números complejos) | bola d‑dimensional (girovectores de Ungar); el disco 2D queda solo para mostrar posiciones |
| Embedding | TF‑IDF → PCA a 2D | TF‑IDF → grafo de vecinos por coseno → **SGD Riemanniano** en la bola (semilla fija) |
| `cd x` | Möbius de SU(1,1) que lleva x al origen | `T ← L_{−T(x)} ∘ T`, con `T = (t, R)`: `z ↦ t ⊕ R z`, donde `R` acumula las giraciones |
| Orden de `ls` | distancia al último foco | puntaje de trayectoria (definición abajo); con `--memoria 0` vuelve a ser como en la 0.1 |
| E/S del corpus | `std::fs::read` | **io_uring** (crate `io-uring`) con respaldo `pread`, y caché de índice en disco |
| Vistas | se recalculaban en cada operación | **caché LRU** (4096 vistas) + cálculo repartido en hilos para corpus grandes |
| Corpus | 18 textos | + generadores `stdlib` (texto real local) y `sintetico` (hasta 10 000) |

## Cómo funciona

1. **Embedding** (`src/embed.rs`, `src/ajuste.rs`). Rasgos: palabras y trigramas de caracteres, con hashing con signo a 1024 dimensiones y peso TF‑IDF. Con eso se buscan los 5 vecinos de cada documento por coseno (exacto, O(n²), en hilos).
   - El ajuste usa la pérdida de Nickel & Kiela (2017), `d(i,j) + log Σ exp(−d(i,k))` con 10 negativos por par.
   - El gradiente de `d = arcosh(1 + 2|u−v|²/((1−|u|²)(1−|v|²)))` se reescala por la métrica `(1−|u|²)²/4`.
   - Los puntos se retraen a |x| ≤ 0,95 para que la aritmética quede bien condicionada.
   - La semilla es fija (splitmix64), así que el mismo corpus da siempre los mismos puntos.
2. **Geometría** (`src/bola.rs`): suma de Möbius, distancia, giración (fórmula cerrada y definición), `exp`/`log` en el origen y en cualquier punto, y la isometría `Transf {t, R}`. Componer `L_a ∘ (t, R)` da `(a ⊕ t, gyr[a,t]·R)`. El módulo 2D de la 0.1 (`src/hiper.rs`) sigue igual y sirve de referencia: en d = 2 la suma de Möbius coincide con `(a+z)/(1+āz)`.
3. **Ruta = trayectoria** (`src/proyeccion.rs`). La trayectoria es `p_0 = 0` (el origen), seguido de los puntos de cada `cd` hasta el foco actual `p_n`.
   - **`puntaje(x) = Σ_k γ^{n−k} · d(p_k, x) / Σ_k γ^{n−k}`**, con γ = `--memoria` (0,5 por defecto). El foco actual pesa más, pero el camino por el que se llegó también cuenta.
   - Se listan los `k` (10) de menor puntaje, sin el foco y a distancia ≤ `radio` (12) del foco. Los empates se resuelven por nombre, así que el orden es total y determinista.
   - La posición 2D de `.vista` es `exp_0(proyección de log_0(T x))` sobre un plano fijo (PCA). Como `T` lleva la giración acumulada, esa posición también depende de todo el camino.
4. **FUSE** (`src/fs.rs`). En cada directorio aparecen `<nombre>/` (para hacer `cd`), `<nombre>.txt` (el contenido) y dos archivos especiales:
   - `.vista`: rango, puntaje, distancia al foco, posición 2D y nombre.
   - `.foco`: ruta, |T(foco)|, error de ortogonalidad de R, γ y la trayectoria con sus pesos.
   - `readdir` devuelve las entradas en orden de puntaje (se ve con `ls -U`). `cd` a algo que no está en la proyección da `ENOENT`, y escribir da `EROFS`.
   - Hay un límite de 64 niveles de profundidad.
   - Las vistas se guardan en una caché LRU por ruta. Nunca hay que invalidarlas, porque son deterministas.
5. **E/S** (`src/es.rs`). `POINCARE_IO=auto|uring|pread`:
   - `auto` prueba io_uring creando un anillo y ejecutando un NOP. Si no anda (kernel viejo, seccomp, contenedor), usa `pread`.
   - `uring` falla con un error explícito si io_uring no está disponible.
   - El corpus completo y el índice (`--indice archivo`) se leen por esa vía, con hasta 64 lecturas en vuelo y reintento de lecturas cortas.

## Cómo correrlo

Requisitos: Linux con `/dev/fuse` y `fusermount3` (paquete `fuse3`). No hace falta `libfuse3-dev`. `fusermount3 -u` necesita que exista `/etc/mtab`; en la computadora compartida no existía y se creó el enlace estándar `/etc/mtab -> ../proc/self/mounts`.

```sh
cargo build --release
B=./target/release/poincare-fs
mkdir -p /tmp/disco
$B montar corpus /tmp/disco &        # [--k N] [--radio R] [--memoria G] [--dim D] [--indice F] [--cache N]
cd /tmp/disco && ls -U && cd kernel-ebpf && cd kernel-planificador && cat .foco && cat kernel-fuse.txt
cd / && fusermount3 -u /tmp/disco   # el proceso termina solo

$B calidad corpus                    # métrica de vecino más cercano
$B vista corpus kernel-ebpf          # vista sin montar nada
$B es-info corpus                    # qué E/S se usó (Uring/Pread) y huella de lo leído
$B generar stdlib 1000 /tmp/c1k      # texto real: docstrings y comentarios del stdlib de Python
$B generar sintetico 10000 /tmp/c10k # 10 000 docs muestreados del vocabulario de cada paquete
$B indexar /tmp/c10k /tmp/c10k.idx   # guarda el embedding (se valida con una huella)
sh bench/correr.sh                   # benchmark completo (ver abajo)
sh bench/valor.sh                    # experimento de valor vs ripgrep / TF-IDF / FAISS (ver VALOR.md)
```

## Resultados medidos (20/10/2026, computadora compartida: kernel 6.12, 8 núcleos, Rust 1.85.1)

### Calidad

* `cargo build --release`: OK. `cargo clippy --all-targets -- -D warnings`: 0 avisos.
* `cargo test` corrido **3 veces**: las 3 veces dio **57 tests en verde** y 0 fallas:
  - 53 de la biblioteca (unitarios y de propiedades);
  - 3 de integración con montaje FUSE real;
  - 1 de seccomp.

  Además, un clon nuevo del repositorio pasa todo. Ver [VERIFICACION.md](VERIFICACION.md).
* **Propiedades en d = 16** (proptest, 2048 casos aleatorios por propiedad, |x| ≤ 0,95). Todas se cumplen dentro de 1e‑9:
  - la traslación de Möbius y la composición de 1 a 4 `cd` son **isometrías**;
  - la bola va en la bola;
  - **cancelación a izquierda** `⊖a ⊕ (a ⊕ b) = b`;
  - **giroasociatividad** `a ⊕ (b ⊕ c) = (a ⊕ b) ⊕ gyr[a,b]c`;
  - giración por fórmula = giración por definición, y la giración es ortogonal (1e‑12);
  - `a ⊕ b = gyr[a,b](b ⊕ a)`;
  - `exp_x(log_x y) = y`;
  - `cd` lleva el foco a < 1e‑9 del origen, con R ortogonal (1e‑9);
  - composición = aplicación sucesiva;
  - distancia sin reservar memoria = distancia por definición;
  - en d = 2, coincide con el disco complejo de la 0.1.

  Se conservan todos los tests de la 0.1 del disco 2D, incluido el barrido de 10⁶ casos: error máximo 2,50e‑13.
* **Vecino más cercano del mismo tema** (`poincare-fs calidad`; el tema es el prefijo del nombre):

| corpus | TF‑IDF 1024d (techo) | 0.1: disco 2D PCA | **0.2: bola 16d (RSGD)** | 0.2: bola → disco 2D de la vista |
|---|---:|---:|---:|---:|
| ejemplo, 18 docs, 6 temas | 16/18 | 10/18 | **15/18** | 13/18 |
| stdlib real, 1000 docs, 27 temas | 835/1000 | 269/1000 | **810/1000** | 519/1000 |

  - Con otras semillas del ajuste, en el corpus de 18 dio 15, 14, 15, 15 y 15 (semillas 1 a 5). La meta de ≥ 15/18 se cumple con la semilla por defecto y en 4 de 5 semillas, no en todas.
  - Con d = 2, 8 y 32: 14, 15 y 15 de 18.
  - Recall@5 de los vecinos TF‑IDF dentro de la bola: 0,68 (18 docs) y 0,57 (1000 docs).
* **El camino importa.** El test montado busca un foco `b` al que se llega por dos caminos (`/a1/b` y `/a2/b`). Comprueba que las `.vista` son distintas (cambian puntajes y orden), pero que la distancia de cada documento común al foco es idéntica (< 1e‑8, isometría). Con `--memoria 0` se recupera la semántica de la 0.1 (test unitario).
* **Montaje real** (`tests/montaje.rs`):
  - `ls -U` en la raíz y tras dos `cd` coincide, en contenido y orden, con el cálculo sin FUSE;
  - `.vista` montada es idéntica byte a byte y estable;
  - la distancia listada es la hiperbólica original en la bola;
  - `|T(foco)| < 1e‑12` y `R` es ortogonal (< 1e‑12);
  - `ENOENT` y `EROFS` donde corresponde, y `dd bs=7` lee el archivo exacto;
  - 64 `cd` anidados funcionan y el 65 falla;
  - montado con `POINCARE_IO=pread` da exactamente lo mismo;
  - `fusermount3 -u` desmonta y el proceso sale con 0.

### io_uring

* **FUSE‑over‑io_uring no existe en este kernel.** Es 6.12, y esa función entró en el 6.14. `/sys/module/fuse/parameters/` solo tiene `allow_sys_admin_access`, `max_user_bgreq` y `max_user_congthresh`: no aparece `enable_uring`. Tampoco hay `/proc/config.gz` para consultar `CONFIG_FUSE_IO_URING`. Por eso el canal kernel↔FUSE sigue siendo `/dev/fuse` clásico.
* **io_uring sí funciona para la E/S del almacén** (`kernel.io_uring_disabled = 0`, sin seccomp). `auto` elige `Uring` en esta máquina, y el test lo exige.
* **El respaldo está probado de verdad.** `tests/seccomp.rs` ejecuta el binario con un filtro seccomp que hace fallar `io_uring_setup` con ENOSYS. Con eso, `auto` cae a `Pread` y lee exactamente los mismos bytes (misma huella FNV), y `uring` forzado falla con un error explícito. Los tests unitarios leen con ambos modos archivos de 0 B, 4 KiB y 3 MiB (este último en varias lecturas) y 200 archivos chicos.
* **io_uring no es más rápido acá.** Leyendo 10 000 archivos (4,75 MB, ya en la caché de páginas) con `es-info`, io_uring tardó 0,052 s y `pread` 0,027 s, con la máquina cargada. El costo dominante es abrir los archivos, que se hace con `open` normal (sin `IORING_OP_OPENAT`). io_uring está por el requisito y por la arquitectura, no porque se haya medido una mejora.

### Rendimiento (montado, caminata aleatoria por `cd`)

Script: `sh bench/correr.sh` (arnés propio en Rust: `poincare-fs bench`; no es criterion ni hyperfine). Cada paso hace:

1. `ls` de un directorio nuevo (la vista se calcula);
2. `ls` repetido del mismo directorio;
3. `cat` de un `.txt` al azar;
4. `cd` a un subdirectorio al azar (vuelve a la raíz a profundidad 32).

Se hicieron 2000 pasos por corrida (300 en 10 000 sin LRU). Con LRU se tomó la mediana de 3 corridas; sin LRU, una sola corrida. Las latencias se miden en el proceso cliente, así que incluyen ir y volver por el kernel. Salida completa en `bench/resultados-2026-10-20.txt`.

> **Atención:** durante la medición la computadora compartida estaba muy cargada por otros agentes (load average 20–23 en 8 núcleos), así que los p99 tienen mucho ruido. Los números sirven para comparar entre sí, no como cifra absoluta.

| corpus | LRU | operación | p50 (µs) | p99 (µs) | ops/s |
|---|---|---|---:|---:|---:|
| 18 | 4096 | ls (vista nueva) | 103.2 | 4899.0 | 4561 |
| 18 | 4096 | ls (repetido) | 26.3 | 990.3 | 13919 |
| 18 | 4096 | cd | 9.1 | 23.0 | 59780 |
| 18 | 4096 | cat | 20.5 | 53.7 | 26758 |
| 18 | sin | ls (repetido) | 153.9 | 4690.0 | 3659 |
| 18 | sin | cd | 71.0 | 155.1 | 12373 |
| 1000 (stdlib) | 4096 | ls (vista nueva) | 940.0 | 7054.3 | 749 |
| 1000 (stdlib) | 4096 | ls (repetido) | 27.1 | 484.3 | 16430 |
| 1000 (stdlib) | 4096 | cd | 10.2 | 75.6 | 37921 |
| 1000 (stdlib) | 4096 | cat | 24.0 | 108.7 | 25889 |
| 1000 (stdlib) | sin | ls (repetido) | 1964.3 | 10502.7 | 328 |
| 1000 (stdlib) | sin | cd | 770.3 | 4662.4 | 1182 |
| 10 000 (sint.) | 4096 | ls (vista nueva) | 7067.9 | 23826.3 | 137 |
| 10 000 (sint.) | 4096 | ls (repetido) | 41.8 | 1872.8 | 11526 |
| 10 000 (sint.) | 4096 | cd | 13.2 | 52.0 | 37210 |
| 10 000 (sint.) | 4096 | cat | 38.8 | 479.1 | 15568 |
| 10 000 (sint.) | sin | ls (repetido) | 15808.8 | 45892.4 | 56 |
| 10 000 (sint.) | sin | cd | 7673.6 | 36094.7 | 112 |

Qué muestran los números:

- Con la caché LRU, `cd`, `cat` y `ls` repetido no dependen casi del tamaño del corpus: 10 a 40 µs de p50. Sin caché, en 10 000 docs cada operación recalcula la vista y tarda unos 8 a 16 ms.
- El costo que queda es la **primera** vista de cada directorio, que es O(n · largo de la trayectoria): unos 7 ms de p50 con 10 000 docs, repartido en hilos.
- Calcular el embedding (vecinos O(n²) + SGD): 0,19 s con 18 docs, 1,5 s con 1000 y 13,2 s con 10 000. Con `--indice` se calcula una sola vez; montar después tarda 0,13 s.

## Qué NO está verificado

* **Valor contra líneas base: medido y NO demostrado** ([VALOR.md](VALOR.md)). Para encontrar un documento relevante en el stdlib de 1000 documentos, el FS montado:
  - en «más como este» acierta 31,5 % contra 84,0 % de FAISS y TF‑IDF planos y 48,5 % de ripgrep, y tarda 7,5× FAISS;
  - con consultas escritas, un agente que solo navega el FS acierta 22,2 % contra 88,9 % y 77,8 %.

  El motivo es que no se puede hacer `cd` a un documento fuera de la vista. Con γ = 0, 349 de 1000 documentos no se alcanzan por ningún camino, y con γ = 0,5 la navegación con oráculo llegó a 73 de 200. La geometría sola, consultada directamente (algo que el FS no ofrece), empata con TF‑IDF (85 contra 84 %), pero queda en 0,73× FAISS, por encima del umbral de 0,70×.

* **FUSE‑over‑io_uring**: no se usa ni se probó, porque el kernel 6.12 no lo trae. io_uring se usa solo para leer el almacén (corpus e índice) al montar, y en esta máquina no fue más rápido que `pread` (0,052 s contra 0,027 s con 10 000 archivos).
* **`cat` no lee del disco en el momento.** El corpus se precarga completo en memoria al montar (por io_uring). No hay lectura bajo demanda ni prefetch incremental, y los cambios en el corpus no se ven hasta volver a montar.
* **Benchmarks con ruido.** Se midió con la máquina compartida a load average ~20 en 8 núcleos. No se midió en una máquina quieta ni en la de Federico. No se usó criterion ni hyperfine, sino un arnés propio. Sin LRU con 10 000 docs se hicieron solo 300 pasos y una corrida.
* **La calidad semántica tiene límites.**
  - El techo es TF‑IDF por hashing, no un modelo de lenguaje, y la bola queda por debajo de ese techo (15/18 contra 16/18; 810 contra 835 de 1000).
  - Una de 5 semillas da 14/18.
  - La métrica de 10 000 docs no se calculó: es O(n²) en 1024 dimensiones.
  - El corpus de 10 000 es **sintético** (palabras muestreadas, no prosa). El real del stdlib da como máximo unos 2079 trozos.
* **El disco 2D de `.vista` pierde información**: 13/18 y 519/1000 de vecinos del mismo tema, contra 15/18 y 810/1000 en la bola. Sirve para visualizar, no para medir.
* Precisión: las propiedades se exigen con |x| ≤ 0,95, que es el radio máximo del ajuste. Más cerca del borde no se probó.
* La definición del puntaje de trayectoria es una elección de diseño, no un resultado. Se probó que el camino cambia la vista y que con γ = 0 se recupera la 0.1, pero no que sea "mejor" para navegar.
* La tabla de inodos crece con cada ruta visitada (tope de 2²⁰; después da `ENOENT`). El tope no se probó.
* `find` o `du` sin límite de profundidad recorrerían un árbol enorme (hasta 64 niveles con 10 ramas cada uno). No se probó.
* FUSE de un solo hilo (`fuser` 0.15). No se probó con varios clientes concurrentes.
* El test de seccomp es solo para x86_64. No se probó en un contenedor real que bloquee io_uring, sino simulado con seccomp.
* Solo se probó en esta computadora (Debian 13, kernel 6.12). No se probó en la máquina de Federico.
