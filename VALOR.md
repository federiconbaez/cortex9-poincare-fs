# VALOR de poincare-fs contra líneas base simples

## v0.3: re-intento con acceso directo (21/10/2026)

**Respuesta corta: pasa con condiciones, y no por la geometría sola.**

- **Lo pre-registrado:**
  - En T2', `@buscar` (híbrido léxico → bola) **pasa** las cuatro celdas en bucle caliente.
  - En T1', `@similar` (bola pura) **falla** el recall@1 contra FAISS: 78,0 % contra 82,5 %.
  - Por lo tanto, el diseño pre-registrado **no pasa completo**.
- **Lo post-hoc:**
  - `@similar-hibrido` se diseñó después de ver un agregado de T1' (ver «Desvío»).
  - Sobre un segundo conjunto reservado, T1'', **pasa** las cuatro celdas.
  - En el híbrido, el recall@1 **empata** con FAISS por construcción: el primer resultado es el vecino TF‑IDF. La bola solo reordena las posiciones 2–10, y ahí mejora la P@10.
- **El tiempo:** pasa en bucle caliente, con las dos corridas entre 0,27× y 0,48× FAISS. Con consultas intercaladas por otro trabajo (una corrida), T2' **falla** el tiempo (0,83×) y T1 queda justo (0,60–0,62×).

### Protocolo y conjunto reservado (condición de Research)

- `e45e51e` (21/10 00:19, hora de Buenos Aires, UTC−3) fija T1' y T2' **antes** de re-medir o ajustar:
  - script determinista: [`bench/heldout/generar_heldout.py`](bench/heldout/generar_heldout.py);
  - T1': 200 semillas con la semilla 20261021, disjuntas de las de T1;
  - T2': 54 consultas automáticas con la semilla 20261022, 27 K de términos de IDF alto y 27 S de tramos de 3 a 8 palabras;
  - más el pre-registro del diseño.

  sha256 completos en [`MANIFEST.txt`](bench/heldout/MANIFEST.txt):
  - `generar_heldout.py`: `60e6654bdd379693151b1ce38b9415ed6c1fb35d530783dd9f6762d82f12f9be`
  - `t1p-semillas.tsv`: `9b14c0ae97341da68b1febcd15e0a8571485f4733b6655e596d45d6df72cf04d`
  - `t2p-consultas.tsv`: `811ed794a03bf1aa5a7d14575ee848382176f7c3a493d20b045660a601e7445e`
- `e483cfb` (00:30) fija **T1''**: 200 semillas con la semilla 20261023, disjuntas de T1 s7/s11/s23 y de T1'. Lo hace antes de medir el diseño post-hoc `@similar-hibrido`. Ver [`ADENDA-T1pp.md`](bench/heldout/ADENDA-T1pp.md).
- `7196986` (00:32) es el código medido. Las corridas 1 y 2 son de las 00:32–00:34, y la intercalada de las 00:43–00:56.
- **Desvío declarado:**
  - Una corrida de prueba midió T1' y T2' sin querer, porque el filtro «solo referencia» no estaba compilado.
  - Se vieron **solo agregados**. Su TSV queda en `bench/heldout/desvio-corrida-accidental.tsv`, sha256 `7738c350d595568b7fb2c7055b58b14f07756df0024cb41423b13c4d0880c106`.
  - Después no se tocó el ranking pre-registrado, solo la velocidad: el TTL de las entradas mágicas y vectorizar sin reservar memoria, que da el mismo resultado bit a bit.
  - El diseño nuevo de T1 se juzga en T1'', no en T1'.

### Qué cambió en el FS

Todo está en la interfaz FUSE real, no en una API aparte ([`src/directo.rs`](src/directo.rs), [`src/fs.rs`](src/fs.rs)).

| ruta | qué hace |
|---|---|
| `cd @doc/<nombre>` | salta a la vista de cualquier documento en una operación |
| `ls -U @similar/<nombre>` | los 10 vecinos hiperbólicos (**bola pura**) |
| `ls -U @similar-hibrido/<nombre>` | 1.º el vecino TF‑IDF más cercano; 2.º–10.º los 50 vecinos TF‑IDF ordenados por distancia hiperbólica a la semilla (**híbrido**) |
| `ls -U "@buscar/<consulta>"` | 1.º el mejor TF‑IDF; 2.º–10.º los 50 mejores TF‑IDF ordenados por distancia hiperbólica al 1.º (**híbrido**) |
| `ls -U "@buscar-bola/<consulta>"` | orden **solo por la bola**, desde el punto medio de Einstein de los 5 mejores léxicos. El léxico solo ubica el punto, porque no hay codificador de texto a la bola. |
| `…/.resultados` | ranking con puntajes |

- **Directorios mágicos:**
  - Se resuelven con `lookup` desde cualquier directorio, pero **no se listan**. Así el listado normal no cambia y los 58 tests viejos siguen iguales.
  - Los nombres de documento que empiezan con `@` se rechazan.
  - Cada `ls` llega al proceso y **recalcula**: no hay caché de resultados ni `FOPEN_CACHE_DIR`.
  - Se probó `FUSE_NO_OPENDIR_SUPPORT`: con eso el kernel cachea el listado, el `ls` repetido baja a 3,7 µs y no recalcula. **Se descartó** por deshonesto para medir.
  - Las entradas mágicas tienen TTL de 1 h, porque son inmutables.
- **Bola:** coordenadas f32 por dimensión (SoA), con `a_i = 1/(1−|x_i|²)` precalculado. Como `cosh d = 1 + 2|q−x|² a_q a_x`, el prefiltro f32 es exacto en orden. Se toman k+16 candidatos y se reordenan en f64 con la distancia hiperbólica exacta. Un test lo compara contra fuerza bruta.
  - Tarda **~8 µs** en proceso, contra ~130 µs de FAISS sobre TF‑IDF de 1024 dimensiones.
- **Léxico:** los mismos TF‑IDF que FAISS, guardados por columnas en f32. La consulta es dispersa, así que se recorren solo las dimensiones no nulas, unos 10 µs incluyendo vectorizar.
  - En `@similar-hibrido` los 50 vecinos TF‑IDF de cada documento se **precalculan al montar** (n ≤ 5000). El montaje completo tarda ~1,0 s, con índices y cobertura.
- **Cobertura (alcance completo):**
  - Hay un árbol de alcance: BFS desde `/` por el grafo de k vecinos con γ = 0. Los 349 documentos que no son vecinos de nadie se cuelgan, estilo Prim, del documento alcanzado más cercano, con profundidad ≤ 63.
  - La vista de cualquier ruta cuyo foco sea el padre incluye al hijo si no estaba entre los k. En `.vista` aparece aparte, como `# cobertura`.
  - En el corpus de 1000 documentos hay como máximo 43 entradas extra en una vista `/<doc>`.

### Alcance (corpus de 1000 documentos, en las tres corridas)

- Árbol: **1000/1000**, con 349 colgados por cobertura y profundidad máxima 22.
- Con γ = 0, por cualquier camino: **1000/1000**. Antes era 651/1000.
- **Verificado montado con γ = 0,5** (por defecto): recorriendo el árbol con `cd` y `ls` reales aparecen **1000/1000**, con 4 `ls` de mediana y 22 como máximo.
- Además, `cd @doc/<x>` llega a cualquiera en una operación.

### Resultados: bucle caliente, misma forma de medir que FAISS

Cada método corre en su propio bucle: 5 repeticiones, la 1.ª es la «fría» y las 4 siguientes las «calientes». Hubo dos corridas completas, con load average de 4 a 5,3 ([`bench/valor3-corrida1.txt`](bench/valor3-corrida1.txt), [`corrida2`](bench/valor3-corrida2.txt), TSV por consulta al lado).

- **Los aciertos fueron idénticos en las dos corridas.**
- Los tiempos son medianas calientes en ms, como «corrida 1 / corrida 2».
- En el FS el tiempo es el `read_dir` completo (lookup + opendir + readdir×2 + releasedir) con el cálculo incluido. En T2 incluye además vectorizar la consulta; **FAISS no la incluye**, igual que antes.

#### Conjuntos reservados (sobre estos se da el veredicto)

| conjunto | método | recall@1 | P@10 | p50 (ms) | p99 (ms) |
|---|---|---:|---:|---:|---:|
| T1' | FS `@similar` (bola pura, pre-registrado) | 78,0 % | 0,677 | 0,043 / 0,043 | 0,107 / 0,068 |
| T1' | FS `@similar-hibrido` (post-hoc, solo referencia) | 82,5 % | 0,689 | 0,043 / 0,040 | 0,067 / 0,061 |
| T1' | FAISS IndexFlatIP | 82,5 % | 0,608 | 0,128 / 0,138 | 0,161 / 0,165 |
| T1' | ripgrep | 50,0 % | 0,392 | 6,52 / 6,45 | 8,61 / 8,94 |
| T1'' | FS `@similar-hibrido` (post-hoc) | 81,5 % | 0,652 | 0,037 / 0,037 | 0,058 / 0,051 |
| T1'' | FS `@similar` (bola pura) | 78,0 % | 0,637 | 0,043 / 0,042 | 0,055 / 0,101 |
| T1'' | FAISS IndexFlatIP | 81,5 % | 0,582 | 0,127 / 0,139 | 0,155 / 0,193 |
| T1'' | ripgrep | 50,5 % | 0,366 | 6,58 / 6,56 | 9,11 / 9,41 |
| T2' | FS `@buscar` (híbrido, pre-registrado) | 79,6 % | 0,357 | 0,051 / 0,069 | 0,114 / 0,135 |
| T2' | FS `@buscar-bola` (bola pura) | **13,0 %** | 0,245 | 0,055 / 0,076 | 0,118 / 0,101 |
| T2' | FAISS IndexFlatIP | 79,6 % | 0,337 | 0,134 / 0,144 | 0,160 / 0,182 |
| T2' | ripgrep | 68,5 % | 0,340 | 6,66 / 6,79 | 9,85 / 8,95 |

#### Referencia (T1 con 3 semillas de muestreo; T2 con las 54 consultas a mano)

| conjunto | `@similar` (bola) | `@similar-hibrido` | FAISS | rg | p50 FS bola / híbrido / FAISS (ms) |
|---|---|---|---|---|---|
| T1 s7 | 85,0 % · P@10 0,722 | 84,0 % · 0,706 | 84,0 % · 0,629 | 48,5 % | 0,047 / 0,038 / 0,131–0,135 |
| T1 s11 | 81,5 % · 0,697 | 88,0 % · 0,705 | 88,0 % · 0,615 | 50,0 % | 0,045–0,048 / 0,038–0,041 / 0,129–0,138 |
| T1 s23 | 82,0 % · 0,691 | 81,0 % · 0,686 | 81,0 % · 0,602 | 51,0 % | 0,043–0,046 / 0,039–0,043 / 0,130–0,137 |

- Con la bola pura, T1 da entre 81,5 y 85,0 % según la semilla; FAISS da entre 81,0 y 88,0 %. **Gana en 2 de 3 semillas y pierde en s11.** El empate de la v0.2 (85 contra 84) era la semilla favorable.
- En T2: `@buscar` 88,9 % · P@10 0,666 · 0,053–0,058 ms; `@buscar-bola` 46,3 % · 0,481; FAISS 88,9 % · 0,667 · 0,136–0,138 ms; rg 77,8 % · 0,588.

### Veredicto por celda (conjuntos reservados, mediana caliente, corridas 1 / 2)

| caso | t / rg | t / FAISS | recall@1 contra rg | recall@1 contra FAISS | ¿pasa? |
|---|---|---|---|---|---|
| **T1'**, `@similar` bola pura (pre-registrado) | 0,007 / 0,007 ✔ | 0,34 / 0,31 ✔ | 78,0 ≥ 50,0 ✔ | **78,0 < 82,5 ✘** | **NO** |
| **T2'**, `@buscar` híbrido (pre-registrado) | 0,008 / 0,010 ✔ | 0,38 / 0,48 ✔ | 79,6 ≥ 68,5 ✔ | 79,6 = 79,6 ✔ (empate) | **SÍ** |
| T2', `@buscar-bola` bola pura | ✔ | 0,41 / 0,52 ✔ | 13,0 < 68,5 ✘ | 13,0 < 79,6 ✘ | NO |
| **T1''**, `@similar-hibrido` (post-hoc) | 0,006 / 0,006 ✔ | 0,29 / 0,27 ✔ | 81,5 ≥ 50,5 ✔ | 81,5 = 81,5 ✔ (empate) | **SÍ** |

**Intercalado** (1 corrida, [`bench/valor3-intercalado.txt`](bench/valor3-intercalado.txt)): entre cada consulta medida se corre un `rg` sin medir, y se hace igual con FAISS. Las cachés se enfrían y el hilo de FUSE tiene que despertar. Un primer intento de esta corrida se descartó porque un `find /` lanzado por error recorrió el punto de montaje y ocupó el proceso del FS. El código del modo intercalado (`PFS_INTERCALAR`) entró en el commit de documentación, después de `7196986`; solo cambia el arnés.

| conjunto | método | p50 FS (ms) | p50 FAISS (ms) | t / FAISS | p99 FS / FAISS (ms) |
|---|---|---:|---:|---:|---:|
| T1' | `@similar` | 0,113 | 0,181 | 0,62 ✔ | 0,75 / 0,26 |
| T1'' | `@similar-hibrido` | 0,106 | 0,177 | 0,60 ✔ | 0,82 / 0,29 |
| T2' | `@buscar` | 0,141 | 0,170 | **0,83 ✘** | 0,99 / 0,24 |

### Lectura honesta

- **Lo que se arregló:**
  - El alcance: de 651 a 1000/1000, probado montado.
  - La interfaz: una operación en vez de 33 acciones.
  - La velocidad: el camino FUSE completo queda en 0,27–0,48× FAISS en caliente.
- **Lo que no:** la bola sola **no** iguala a FAISS en recall@1 sobre datos nuevos:
  - T1': 78,0 contra 82,5;
  - T1'': 78,0 contra 81,5;
  - T1 s11: 81,5 contra 88.
  
  Para palabras clave, la bola sola es inútil: 13 % en T2'.
- **Qué aporta la bola en los híbridos:** el primer resultado es el de TF‑IDF, así que el recall@1 empata **por construcción**. La bola aporta la P@10: 0,652 contra 0,582 en T1'', 0,689 contra 0,608 en T1' y 0,357 contra 0,337 en T2'.
- **De dónde sale la ventaja de tiempo:** no es de «la geometría hiperbólica» en sí. Sale de:
  - 16 dimensiones contra 1024;
  - la consulta léxica dispersa;
  - en `@similar-hibrido`, el precálculo al montar de los vecinos TF‑IDF, que FAISS plano no hace. Un FAISS con vecinos precalculados también sería casi instantáneo.
- **Bajo carga intercalada:** el p99 del FS (0,75–0,99 ms) es 3 a 4 veces el de FAISS, y en T2' la mediana no cumple 0,70×.

### Qué falta

- Repetir la corrida intercalada (hubo una sola) y medir en una máquina quieta o en `host-de-pruebas`.
- Bajar el costo en frío de T2:
  - el índice léxico ocupa 4 MB en f32; con listas invertidas o int8 cabría en caché;
  - la latencia de despertar el hilo de FUSE.
- Un codificador de consultas a la bola, que evite el ancla léxica. Sin eso, la bola pura no sirve para palabras clave.
- Corpus más grandes (10k) y documentos reales del usuario. El precálculo léxico se desactiva con n > 5000 y no se midió.
- Tiempo de una persona usando `@buscar` y `@similar`.

## Cómo reproducir (v0.3)

```sh
sh bench/valor3.sh 1          # bucle caliente -> bench/valor3-corrida1.{txt,tsv}
PFS_INTERCALAR=1 ./target/release/poincare-fs valor-directo /tmp/pfs-valor/c1k --exportar /tmp/pfs-valor/exp3-i --tsv bench/valor3-intercalado.tsv
PFS_INTERCALAR=1 /tmp/pfs-faiss-venv/bin/python bench/valor_faiss_directo.py /tmp/pfs-valor/exp3-i 5
python3 bench/heldout/generar_heldout.py /tmp/pfs-valor/c1k /tmp/x   # mismos sha256 que MANIFEST.txt
```

---

# v0.2 (histórico, commit `849983a`; auditado en `20cc898`: FALLA)

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
