# VALOR de poincare-fs contra líneas base simples

**Pregunta:** ¿poincare-fs ayuda a *encontrar* un documento relevante más rápido o mejor que herramientas simples (ripgrep, un índice vectorial plano)? No se mide si monta (eso está en [VERIFICACION.md](VERIFICACION.md)), sino si aporta valor.

**Respuesta corta: no.** En esta tarea poincare-fs, usado como sistema de archivos, **pierde** contra FAISS plano y contra TF‑IDF plano en tiempo y en aciertos. Contra ripgrep es más rápido, pero acierta menos. El umbral que fijó Research **no se cumple**. Lo único que muestra algo bueno es el embedding de 16 dimensiones de la bola, consultado directamente y sin pasar por el FS. Para «más como este» empata en aciertos con TF‑IDF/FAISS (85,0 % contra 84,0 %), con mejor P@10 (0,722 contra 0,629). Pero el FS no ofrece ese acceso directo, y aun así queda en 0,73× FAISS, por encima del 0,70× que pide el umbral.

- **Base:** commit `d69f8cb` (v0.2). El arnés y estos resultados se agregaron en el commit que trae este archivo.
- **Medido:** 21/10/2026 a las 00:05 (hora de Buenos Aires, UTC−3), en la computadora compartida: kernel 6.12.94+, 8 núcleos, load average de 7 a 8 por otros agentes.
- **Herramientas:** ripgrep 14.1.1 y faiss‑cpu 1.15.1, instalado en un venv en `/tmp`, no en el sistema.
- **Dónde no se corrió:** en la máquina de Federico, porque `host-de-pruebas` no respondía.

## Umbral (fijado por Research)

Para pasar tienen que cumplirse las tres condiciones:

- mediana del tiempo hasta encontrar ≤ **0,70× ripgrep**;
- mediana del tiempo hasta encontrar ≤ **0,70× FAISS plano**;
- **recall@1 ≥ el de la línea base**.

## Diseño del experimento

* **Corpus 1 (principal):** `poincare-fs generar stdlib 1000`. Son 1000 trozos de texto real (docstrings y comentarios del stdlib de Python 3.13) repartidos en **27 temas**, donde el tema es el paquete: `email`, `asyncio`, `tkinter`… **Corpus 2:** los 18 documentos de `corpus/`, en 6 temas.
* **Relevante = mismo tema.** Es objetivo, porque el tema sale del paquete de origen.
* **Nombres anonimizados.** Los archivos se copian como `dNNNNN.txt`, en orden barajado con semilla fija. Así ningún método, ni el FS ni rg, puede aprovechar que el nombre diga el tema. En un disco real los nombres sí ayudan, y con nombres `ls | grep asyncio` le ganaría a todos.
* **T1 «más como este».** El usuario tiene un documento semilla y quiere otro del mismo tema. Se usan 200 semillas al azar con semilla fija (las 18 en el corpus chico).
* **T2 «consulta».** Son **54 consultas escritas a mano antes de medir** ([`bench/consultas-stdlib.tsv`](bench/consultas-stdlib.tsv)), 2 por tema:
  - K: palabras clave, por ejemplo `event loop coroutine`;
  - D: una frase descriptiva, por ejemplo `schedule a callback to run later on the event loop`.
* **Métricas:**
  - **éxito@1 (= recall@1)**: el primer resultado es del tema;
  - **P@10**: aciertos en los 10 primeros dividido por min(10, relevantes);
  - **tiempo de máquina por consulta**: mediana (p50) y p99 en ms, sobre 5 repeticiones;
  - **acciones**: cantidad de `ls`/`cd`/`cat`, o 1 llamada a rg.

  No se mide el tiempo de una persona: ver «Qué falta».

### Métodos

| método | qué hace exactamente |
|---|---|
| **poincare-fs montado** (T1) | FUSE real. El FS **no permite `cd` a un documento que no esté en la vista actual**, así que hay que navegar hasta la semilla. Desde `/`, en cada paso se hace `ls -U`; si la semilla está visible, se entra (`cd semilla; ls -U`) y la respuesta es ese listado. Si no, se entra al documento visible más cercano a la semilla en la bola. Esto es navegación **con oráculo**, porque conoce las coordenadas: es el mejor caso para el FS. «Frío» es la primera pasada tras montar (las vistas se calculan) y «caliente» son las 4 siguientes (caché LRU). |
| **bola 16d sin FUSE, γ=0** (T1) | Los 10 vecinos hiperbólicos de la semilla con acceso directo. **El FS no expone esto.** Sirve para separar la calidad de la geometría del costo de la interfaz. |
| **TF‑IDF plano** | Coseno sobre los mismos vectores TF‑IDF de 1024 dimensiones con los que el FS arma la bola, en Rust, con barrido exacto en f64. En T2 el tiempo incluye vectorizar la consulta. |
| **FAISS plano** | `faiss.IndexFlatIP` (exacto, 1 hilo) sobre **los mismos** vectores TF‑IDF, exportados en f32 ([`bench/valor_faiss.py`](bench/valor_faiss.py)). El tiempo **no** incluye vectorizar la consulta. |
| **ripgrep** | Un proceso por consulta: `rg --no-config --count-matches -i -w -F -e w1 -e w2 …` sobre los archivos en disco, ordenando por cantidad de coincidencias. En T1 las palabras son las 3 más frecuentes de la semilla (al menos 4 letras, sin palabras vacías); en T2, las de la consulta sin palabras vacías. El tiempo incluye crear el proceso, unos 4 ms en esta máquina. |
| **híbrido rg → poincare-fs** (T2) | rg da el mejor documento, se navega hasta él en el FS (con oráculo) y se completa con su `ls -U`. Mide si el FS agrega documentos relevantes a lo que encontró rg. |
| **agente que navega poincare-fs** (T2) | Usa solo el FS. Desde `/` lee los `.txt` visibles, los puntúa por coincidencias con la consulta y entra con `cd` al mejor no visitado. Corta a los 20 `cd` o tras 3 pasos sin mejorar, y responde con los mejores que vio. |

## Resultados: corpus stdlib de 1000 documentos y 27 temas

Salida completa en [`bench/valor-resultados-2026-10-21.txt`](bench/valor-resultados-2026-10-21.txt). Los datos por consulta están en `bench/valor-stdlib1k.tsv` y `bench/valor-c18.tsv`. Los aciertos fueron **idénticos** en dos corridas completas; los tiempos variaron alrededor de ±20 % por la carga.

### T1 «más como este» (200 semillas)

| método | recall@1 | P@10 | p50 (ms) | p99 (ms) | acciones |
|---|---:|---:|---:|---:|---:|
| poincare-fs montado, frío | **31,5 %** (63/200) | 0,301 | 1,946 | 44,938 | 34,1 |
| poincare-fs montado, caliente | **31,5 %** (63/200) | 0,301 | 1,055 | 4,778 | 34,1 |
| bola 16d directa (no existe en el FS) | 85,0 % (170/200) | **0,722** | 0,103 | 0,178 | 0 |
| TF‑IDF plano (Rust) | 84,0 % (168/200) | 0,629 | 0,673 | 1,070 | 0 |
| **FAISS IndexFlatIP** | 84,0 % (168/200) | 0,629 | **0,141** | 0,164 | 0 |
| ripgrep (3 palabras de la semilla) | 48,5 % (97/200) | 0,424 | 7,019 | 13,092 | 1 |

**Por qué el FS acierta tan poco:** no llega a la semilla. Con la navegación con oráculo **no llegó en 127 de 200** casos. Hacían falta 33 acciones de mediana y hasta 79, y si no llega, la respuesta queda vacía. No es un problema del arnés; repetido en proceso, sin FUSE, da lo mismo:

* **γ = 0,5 (por defecto):** llega a 73/200 semillas. **γ = 0:** a 83/200.
* **Con γ = 0** cada directorio es la lista de 10 vecinos del foco, así que el FS es un grafo dirigido de 10 vecinos. Un BFS por **cualquier** camino desde `/` alcanza solo **651 de 1000** documentos. **349 documentos no se pueden alcanzar nunca**, porque no están entre los 10 más cercanos de nadie (el problema de los «hubs» en grafos de k vecinos).
* Con γ = 0,5 no se hizo un BFS exhaustivo, porque la vista depende de todo el camino. Solo se probó la navegación codiciosa.

### T2 consultas escritas a mano (54: 27 K y 27 D)

| método | recall@1 K | recall@1 D | recall@1 total | P@10 total | p50 (ms) | p99 (ms) | acciones |
|---|---:|---:|---:|---:|---:|---:|---:|
| ripgrep | 81,5 % | 74,1 % | 77,8 % (42/54) | 0,588 | 7,191 | 30,833 | 1 |
| TF‑IDF plano (Rust, incluye vectorizar) | 96,3 % | 81,5 % | 88,9 % (48/54) | 0,667 | 0,798 | 1,470 | 0 |
| **FAISS IndexFlatIP** (sin vectorizar) | 96,3 % | 81,5 % | **88,9 %** (48/54) | 0,667 | **0,136** | 0,167 | 0 |
| híbrido rg → poincare-fs, frío | 81,5 % | 74,1 % | 77,8 % | **0,281** | 11,960 | 48,414 | 35,7 |
| híbrido rg → poincare-fs, caliente | 81,5 % | 74,1 % | 77,8 % | **0,281** | 8,203 | 31,851 | 35,7 |
| agente que navega poincare-fs, frío | 22,2 % | 22,2 % | **22,2 %** (12/54) | 0,193 | 1,647 | 9,937 | 27,9 |
| agente que navega poincare-fs, caliente | 22,2 % | 22,2 % | **22,2 %** (12/54) | 0,193 | 0,965 | 3,221 | 27,9 |

* **El híbrido empeora a rg.** Tiene el mismo recall@1, porque el primer resultado es el de rg, pero la P@10 baja de 0,588 a 0,281: completar con la vista del FS agrega menos documentos relevantes que el propio ranking de rg. Además tarda 1,14× rg, porque hay que navegar hasta el documento.
* **El agente que solo usa el FS acierta en 12/54.** Falla en todas las consultas de los temas con 46 documentos o menos (de `multiprocessing` en adelante en la lista), en las dos de `tkinter` y en una de `logging` y otra de `http`. Solo acierta en temas grandes que aparecen cerca de la raíz.

## Corpus de 18 documentos (6 temas), solo T1

| método | recall@1 | P@10 | p50 (ms) | p99 (ms) |
|---|---:|---:|---:|---:|
| poincare-fs montado, caliente | 77,8 % (14/18) | 1,000 | 0,117 | 0,327 |
| bola 16d directa | 83,3 % (15/18) | 0,972 | 0,013 | 0,017 |
| TF‑IDF plano | 88,9 % (16/18) | 1,000 | 0,013 | 0,023 |
| FAISS IndexFlatIP | 88,9 % (16/18) | 1,000 | 0,005 | 0,018 |
| ripgrep | 50,0 % (9/18) | 0,444 | 4,413 | 8,978 |

Con 18 documentos todo es alcanzable (18/18) en 3 a 5 acciones. Aun así, el FS acierta menos que FAISS y tarda 23× más que FAISS.

## Veredicto contra el umbral (corpus de 1000, mediana caliente)

| caso | t / rg | t / FAISS | recall@1 contra rg | recall@1 contra FAISS | ¿pasa? |
|---|---:|---:|---|---|---|
| T1, FS montado | 0,15× ✔ | **7,5×** ✘ | 31,5 % < 48,5 % ✘ | 31,5 % < 84,0 % ✘ | **NO** |
| T2, agente en el FS | 0,13× ✔ | **7,1×** ✘ | 22,2 % < 77,8 % ✘ | 22,2 % < 88,9 % ✘ | **NO** |
| T2, híbrido rg → FS | 1,14× ✘ | 60× ✘ | igual ✔ | 77,8 % < 88,9 % ✘ | **NO** |
| T1, bola directa (hipotético, sin FS) | 0,015× ✔ | **0,73×** ✘ (por poco) | 85,0 % ≥ 48,5 % ✔ | 85,0 % ≥ 84,0 % ✔ (2 docs de 200) | **NO** (solo falla el tiempo contra FAISS) |

**¿Le gana a ripgrep?** En tiempo sí, de 7 a 8 veces más rápido, pero porque rg lanza un proceso y lee los 1000 archivos en cada consulta. En aciertos no: en T1 tiene 31,5 % contra 48,5 %, y en T2 22,2 % contra 77,8 %.
**¿Le gana a TF‑IDF/FAISS plano?** No, ni en tiempo ni en aciertos.
**¿Gana en temas y pierde en palabras clave?** Solo **la geometría** (bola directa) muestra algo en la tarea por tema (T1): empata en recall@1 (85 contra 84 %, una diferencia de 2 casos que no es concluyente) y tiene mejor P@10 (0,722 contra 0,629). En palabras clave (T2‑K), FAISS/TF‑IDF llegan a 96,3 %, rg a 81,5 % y el FS a 22,2 %. **El FS como interfaz pierde en ambas.** El cuello de botella es la navegación y el alcance, no la geometría.

## Qué sugieren los números (no está implementado ni medido)

* Un acceso directo, por ejemplo `cd @dNNNN` o un archivo `.buscar`, haría que T1 rindiera como la «bola directa». Así pasaría contra rg, y contra FAISS quedaría al borde (0,73×).
* Hacer alcanzables todos los documentos: por ejemplo con vecinos inversos o k adaptativo, para evitar que haya documentos que no son vecinos de nadie.

## Qué falta (no medido)

* **Tiempo de una persona** encontrando documentos (requiere la sesión de Federico). Acá solo hay tiempo de máquina y cantidad de acciones.
* Documentos reales del usuario: solo se usó el stdlib de Python y el corpus de 18 documentos. Tampoco hay corpus más grandes: con 10 000 documentos no se corrió esta prueba.
* Líneas base con embeddings neuronales (no hay GPU ni modelo local) y consultas en paráfrasis pura sin palabras del corpus.
* BFS exhaustivo de alcance con γ = 0,5.
* Otro diseño de agente navegador. Este es uno solo y simple; uno más listo podría acertar más, pero no puede pasar del límite de alcance: con γ = 0, 651/1000.
* FAISS se midió desde Python con la consulta ya vectorizada, y TF‑IDF en Rust con la vectorización incluida. Las dos comparaciones favorecen a la línea base o son neutras para el FS, no al revés.
* Máquina compartida con load average de 7 a 8; no se midió en una máquina quieta ni en `host-de-pruebas`.

## Cómo reproducir

```sh
python3 -m venv /tmp/pfs-faiss-venv && /tmp/pfs-faiss-venv/bin/pip install faiss-cpu numpy
sh bench/valor.sh            # genera /tmp/pfs-valor/c1k si falta; escribe bench/valor-*.tsv
# o por partes:
./target/release/poincare-fs valor /tmp/pfs-valor/c1k --consultas bench/consultas-stdlib.tsv \
    --reps 5 --semillas 200 --tsv bench/valor-stdlib1k.tsv --exportar /tmp/pfs-valor/exp-1k
/tmp/pfs-faiss-venv/bin/python bench/valor_faiss.py /tmp/pfs-valor/exp-1k 5
```

El arnés está en `src/valor.rs` (subcomando `valor`). Las funciones `embed::idf` y `embed::vectorizar` se agregaron para vectorizar consultas, con un test que comprueba que, sobre el corpus, dan bit a bit lo mismo que `embed::tfidf`. No se tocó ni se debilitó ningún test existente: `cargo test --release` da 54 + 3 + 1 = **58 en verde** (eran 57, más el test nuevo), y `cargo clippy --all-targets -- -D warnings` da 0 avisos.
