# Adenda al pre-registro: T1'' (antes de medir `@similar-hibrido`)

**Desvío que se declara.** El arnés tenía una variable `PFS_SOLO_REFERENCIA` para correr solo T1/T2;
en la primera corrida el filtro no estaba compilado (falló un reemplazo de texto) y se midieron también
T1'/T2' con el diseño pre-registrado. Se vieron **solo los agregados** (no resultados por consulta):

* T1' bola pura (`@similar`): recall@1 78,0 % contra FAISS 82,5 % (falla el recall).
* T2' híbrido (`@buscar`): recall@1 79,6 % = FAISS 79,6 %, rg 68,5 %.
* T2' bola pura: 13,0 %.

El ranking pre-registrado **no se cambió** después de eso (solo se cambió velocidad: TTL, vectorizar
sin reservar memoria; los aciertos no dependen de eso). El veredicto pre-registrado sigue siendo T1'/T2'.

**Diseño post-hoc** (después de ver el agregado de T1'): `@similar-hibrido/<doc>` = ancla = vecino
TF‑IDF más cercano (coseno, precalculado al montar para n ≤ 5000: 50 vecinos por doc); posiciones 2–10
= esos 50 candidatos ordenados por distancia hiperbólica a la semilla. Como se diseñó después de ver
T1', **no se juzga en T1'**: se juzga en **T1''** (`t1pp-semillas.tsv`, 200 semillas, splitmix64 semilla
20261023, disjuntas de T1 s7/s11/s23 y de T1'; script `generar_heldout2.py`), fijado y commiteado en este
commit, antes de cualquier medición de `@similar-hibrido`.

sha256 4b5cd775374fa498e918b706d3e90baffbec78e97a26a36f95f6403e8c65e19e  generar_heldout2.py
sha256 28d7cad3ff86d4b949350c60959d9da117157aae9468bc646ce6202e3c5a3334  t1pp-semillas.tsv
