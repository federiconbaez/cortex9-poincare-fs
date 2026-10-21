# Auditoría de Research: re-intento v0.3 del sistema 2 (HEAD `3d09884`)

Auditora: Federica Research Sentinel. Fecha: 21/10/2026, 00:52–01:05 (hora de Buenos Aires, UTC−3).
Umbral #2 literal (`7c3912d`): latencia ≤ 0,70× ripgrep **y** ≤ 0,70× FAISS, con recall@1 ≥ el de cada línea base.
El pre-registro `e45e51e` lo fija así: **mediana caliente**, en T1' **y** en T2'.

## 1. Orden y conjuntos (verificado)

| commit | autor = committer (UTC) | hora BA | contenido |
|---|---|---|---|
| `e45e51e` | 03:19:00 | 00:19 | PREREGISTRO, `t1p-semillas.tsv`, `t2p-consultas.tsv`, `generar_heldout.py`, MANIFEST |
| `e483cfb` | 03:30:12 | 00:30 | ADENDA, `t1pp-semillas.tsv`, `generar_heldout2.py` |
| `7196986` | 03:32:04 | 00:32 | código |
| `3d09884` | 03:50:21 | 00:50 | resultados, VALOR, modo `PFS_INTERCALAR` |

- Los archivos de consultas tienen **un solo commit cada uno** (`e45e51e` y `e483cfb`). Sus sha256 en HEAD coinciden con los de MANIFEST y ADENDA.
- **Regeneración independiente:** regeneré el corpus desde cero con `poincare-fs generar stdlib 1000` y es idéntico a `/tmp/pfs-valor/c1k` (`diff -r`). Los dos scripts reproducen los tres TSV con los mismos sha256, así que son deterministas con semilla fija (20261021, 20261022 y 20261023).
- **Disjunción:**
  - T1'' no comparte semillas con T1' ni con T1 s7/s11/s23 (0 en común) y tiene 200 semillas.
  - T2' no repite ninguna consulta de las 54 escritas a mano.
  - **Observación:** T1' es disjunto de T1 s7, que es lo que prometía el pre-registro. Pero comparte 46 semillas con s11 y 45 con s23 (77 distintas). Esos conjuntos de referencia los agregó `7196986`, después del pre-registro. El veredicto de T1' no cambia, porque su ranking quedó congelado (ver §3).
- El tarball `/tmp/02-poincare-fs-v03.tar.gz` tiene sha256 `6b964bc4…8487fe`, igual al reportado, y su árbol es idéntico al del repo.

## 2. Modo y línea base pre-registrados

`PREREGISTRO.md` fija la **mediana caliente** contra rg y contra FAISS IndexFlatIP, en T1' **y** T2'. Fija además que FAISS no incluye vectorizar la consulta. **El modo intercalado no está pre-registrado:** es un control de robustez agregado después y no rige el veredicto. Lo informo igual, porque es el escenario más parecido a un uso real.

## 3. Desvíos

- **Corrida accidental:**
  - `bench/heldout/desvio-corrida-accidental.tsv` está versionado. Su sha `7738c350…` coincide con el declarado y con `/tmp/pfs-valor/accidental-run1.tsv`.
  - Hubo **una sola** exposición de T1'/T2'. `ref-try.tsv` (03:24 UTC) es byte a byte igual al accidental. ref-try2 a ref-try5 no contienen T1'/T2'/T1''.
  - Ninguna corrida midió T1'' ni `@similar-hibrido` sobre un conjunto reservado antes de `e483cfb`.
- **«Solo velocidad»:**
  - Comparé consulta por consulta (ok1, p10, top1) las 19.780 filas en común entre la corrida accidental y cada corrida final: **0 diferencias**. Lo mismo en las tres corridas mías.
  - El test `tf_rapido_igual_a_tf` (`assert_eq!` entre `vectorizar_rapido` y `vectorizar`) existe y pasa.
- **Corrida intercalada descartada (por el `find /`):**
  - **No quedó su crudo.** `/tmp/pfs-valor/inter.txt` es la corrida que se conservó (mismo directorio anon 1927323) y `exp3-i` fue sobrescrito. **No se puede verificar.**
  - Igual no hubo selección favorable que importe: mi corrida intercalada da peor (0,90×) que la conservada (0,83×), y las dos fallan.
- **Código no commiteado al medir:**
  - El modo intercalado se midió con código que todavía no estaba commiteado. Está declarado y solo toca el arnés.
  - Lo mismo pasó con la corrida accidental (antes de `7196986`). El ranking es idéntico (ver arriba).

## 4. Re-corrida en clon aparte (`/workspace/audit-clones/02-v03` @ `3d09884`)

- `cargo clippy --all-targets -- -D warnings`: limpio. `cargo test --release`: **68/68** (62+2+3+1).
- Benchmark: 2 corridas en caliente y 1 intercalada, con load 3,2–4,2. Crudo en `bench/auditoria/valor3-corridaaudit{1,2,I}.{txt,tsv}`.
- Los aciertos coinciden consulta por consulta con la corrida 1 de Apex (0 diferencias en 33.780 filas).
- Recalculé desde los TSV (`recalcular_v03.py` → `recalculo-v03.txt`): mediana de las repeticiones 2 a 5 y recall@1 de la repetición 1.

| celda | R@1 FS / rg / FAISS | t/rg (Apex c1, c2; Research a1, a2) | t/FAISS caliente (Apex c1, c2; Research a1, a2) | t/FAISS intercalado (Apex; Research) | veredicto literal |
|---|---|---|---|---|---|
| **T1' `@similar` bola pura (pre-reg.)** | 78,0 / 50,0 / **82,5** | 0,007; 0,007; 0,008; 0,006 | 0,34; 0,31; 0,39; 0,33 | 0,62; 0,63 | **FALLA** (recall contra FAISS, −4,5 pts) |
| **T2' `@buscar` híbrido (pre-reg.)** | 79,6 / 68,5 / 79,6 | 0,008; 0,010; 0,011; 0,009 | 0,38; 0,48; **0,58**; 0,45 | 0,83; **0,90** ✘ | **PASA** en el modo pre-registrado (caliente). Falla intercalado (no pre-reg.) |
| T2' `@buscar-bola` | 13,0 / 68,5 / 79,6 | ✔ | 0,41; 0,52; 0,43; 0,53 | 0,84; 0,91 | FALLA |
| T1'' `@similar-hibrido` (post-hoc) | 81,5 / 50,5 / 81,5 | 0,006; 0,006; 0,005; 0,007 | 0,29; 0,27; 0,28; **0,35** | 0,60; 0,63 | pasa, **exploratorio**, no confirmatorio |

**Umbral #2 tal como lo pre-registró `e45e51e` (T1' y T2'): NO PASA**, porque falla T1'.

Contra ripgrep, todas las celdas pasan el tiempo por dos órdenes de magnitud (≤ 0,023×, incluso intercalado). En recall@1 superan a rg todas salvo `@buscar-bola` (13,0 contra 68,5).

## 5. ¿Geometría o interfaz? (ablación sobre las mismas coordenadas 16d)

Código en `bench/auditoria/ablacion_auditoria.rs`, salida en `ablacion-v03.txt` y `top1-hibrido-vs-faiss-v03.txt`. Las variantes hiperbólicas reproducen exactamente los números del arnés, lo que valida la ablación.

- **El empate de recall@1 es por construcción.** El top-1 del híbrido es **idéntico** al top-1 de FAISS en 200/200 (T1'), 200/200 (T1'') y 54/54 (T2'). La bola no interviene en el recall@1 de los híbridos.
- **La métrica hiperbólica no aporta frente a la euclídea en 16d** sobre las mismas coordenadas, y la euclídea es igual o mejor en todos los casos:

  | caso | hiperbólica | euclídea 16d | TF-IDF solo |
  |---|---|---|---|
  | T1' bola pura, R@1 | 78,0 | 80,0 | — |
  | T1' bola pura, P@10 | 0,677 | 0,695 | — |
  | T1'' bola pura, R@1 | 78,0 | 78,0 | — |
  | T1'' bola pura, P@10 | 0,637 | 0,659 | — |
  | T1' híbrido, P@10 | 0,689 | 0,695 | 0,608 |
  | T1'' híbrido, P@10 | 0,652 | 0,669 | 0,582 |
  | T2' híbrido, P@10 | 0,357 | 0,361 | 0,337 |

  La mejora de P@10 viene del **embedding de baja dimensión** (reordenar 49 candidatos), no de la curvatura.
- **El tiempo** se mide en proceso, en p50:

  | operación | p50 |
  |---|---|
  | knn hiperbólico 16d | 0,012 ms |
  | knn euclídeo 16d | 0,007 ms |
  | fuerza bruta 1024d en Rust | 0,57 ms |
  | lectura de vecinos TF-IDF precalculados | 0,0008 ms |
  | FAISS 1024d | 0,13 ms |

  La ventaja es la dimensión (16 contra 1024) y, en `@similar-hibrido`, el **precálculo al montar**: el top-10 léxico sale de una tabla. FAISS con la misma tabla precalculada costaría lo mismo. Por eso la comparación de T1'' es caché contra búsqueda, no algoritmo contra algoritmo. El resto del tiempo del FS (~0,03–0,04 ms) es el recorrido de FUSE.

## 6. Discrepancias con lo reportado por Apex

1. Ninguna en los números de recall ni en el orden de los commits. Los aciertos coinciden bit a bit.
2. Los rangos de tiempo son más anchos en mi re-corrida: T2' caliente 0,45–0,58× (Apex 0,38–0,48×), T1'' 0,28–0,35× (Apex 0,27–0,29×) y T2' intercalado 0,90× (Apex 0,83×). No cambian ningún veredicto.
3. El reporte dice «el intercalado falla». Es correcto, pero **no es el modo pre-registrado**: por el pre-registro, T2' **pasa**, y el veredicto global falla solo por T1'.
4. Las 77 semillas de T1' que se comparten con las referencias s11/s23 no están declaradas. No tienen efecto sobre el veredicto.
5. El crudo de la corrida intercalada descartada no se conservó.
6. `PREREGISTRO.md` dice «00:2x» hora BA, pero el commit es de las 00:19 BA. Es cosmético.

## Conclusión

- **Valor demostrado (confianza alta):** la interfaz. Hay acceso directo, el alcance es 1000/1000 y la latencia es mucho menor que la de rg y menor que la de FAISS en caliente. Con eso, T2' `@buscar` pasa el umbral literal en el modo pre-registrado.
- **No demostrado (confianza alta):** que la geometría hiperbólica aporte. La bola pura falla T1'. Los híbridos empatan por construcción. Sobre las mismas coordenadas, la euclídea 16d iguala o supera a la hiperbólica en R@1 y P@10, y además es más rápida.
- **Veredicto del sistema 2 contra el umbral #2 pre-registrado: FALLA (parcial).** Pasa T2' y falla T1'. T1'' queda como resultado exploratorio a confirmar.
