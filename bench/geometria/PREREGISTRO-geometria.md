# Pre-registro: ¿la geometría hiperbólica aporta a ESTE FS con datos jerárquicos?

Fecha: 21/10/2026, ~01:10 (hora de Buenos Aires, UTC−3). Este commit **precede** a cualquier
entrenamiento o medición del experimento. Sobre HEAD `42bb6ce` (auditoría de Research del 2 v0.3,
que encontró que la ablación euclídea 16d iguala o supera a la hiperbólica en stdlib‑1000).

Condiciones vinculantes de Research (sala, 21/10): (1) el criterio de valor es la **tarea del FS**
(`@similar`: recall@1 y P@10), no la distorsión del árbol, que ya está publicada (Nickel & Kiela 2017,
Sala et al. 2018) y se informa solo como confirmación; (2) las dos geometrías se entrenan **igual**
(mismo objetivo, mismas épocas, misma grilla y presupuesto de hiperparámetros; la euclídea se entrena
de forma nativa, no es una proyección de la hiperbólica); (3) margen, semillas y conjuntos reservados
nuevos quedan fijados en este commit.

## 1. Corpus (deterministas, sin red al construir)

Script: `construir_corpus.py` (dos ejecuciones dan los mismos sha256; ver `MANIFEST-geometria.txt`).
Entradas fijadas:

| entrada | sha256 |
|---|---|
| `nltk_data/corpora/wordnet.zip` (WordNet 3.0, de `raw.githubusercontent.com/nltk/nltk_data/gh-pages/packages/corpora/wordnet.zip`; el descargador de nltk rechaza el proxy del box, se bajó con curl) | `cbda5ea6eef7f36a97a43d4a75f85e07fccbb4f23657d27b4ccbc93e2646ab59` |
| `linux-6.12.tar.xz` (cdn.kernel.org; coincide con `sha256sums.asc` oficial) | `b1a2562be56e42afb3f8489d4c2a7ac472ac23098f1ef1c1e40da601f54625eb` |

**(a) WordNet, sustantivos** (`wordnet-manifiesto.tsv`).
* Camino canónico de cada synset: el más corto de `hypernym_paths()`, empate por lista de nombres.
* Universo: synsets sustantivos con profundidad ≥ 7 (61 325). Muestra: **4000** (semilla 20261021).
* Documento = lemas (con `_`→espacio) + `. ` + glosa. Nada más (sin ejemplos, sin hiperónimos).
* **Relevancia: mismo ancestro a profundidad 5** del camino canónico (entity = 0). 426 clases,
  252 con ≥ 2 miembros; 3826 documentos elegibles como consulta. Profundidad de los docs: 7–18.
* Árbol para distorsión: trie de los caminos canónicos (columna `camino`).

**(b) Árbol real de archivos: `linux-6.12/Documentation`** (`kernel-manifiesto.tsv`).
* Todos los `*.rst`, excluido `translations/` (duplicados en otros idiomas); solo directorios con ≥ 2 `.rst`.
* Documento = primeros 8192 bytes del archivo (UTF‑8 con reemplazo).
* **3071** documentos, **relevancia: mismo directorio** (233 directorios, todos con ≥ 2). Profundidad 1–6.
* Árbol para distorsión: el árbol de directorios (columna `camino`).

En ambos, los nombres son anónimos (`wnNNNNN`, `kdNNNNN`) en orden permutado (semilla 20261023),
para que ni el nombre ni el desempate por índice filtren la clase. El texto de los corpus no se
versiona (se regenera); se versiona el manifiesto con el sha256 de cada documento.

**Conjuntos reservados nuevos** (semilla 20261024 / 20261025, entre los elegibles): por corpus,
`*-val.tsv` = **500** consultas (solo para elegir hiperparámetros) y `*-test.tsv` = **1000**
consultas (solo para el veredicto), disjuntos.

## 2. Tarea (criterio de valor)

`ls -U @similar/<q>` del FS: los k = 10 vecinos de `x_q` en el embedding, sin `q`.
* **recall@1** = % de consultas cuyo primer vecino es relevante. **P@10** = media de relevantes/10.
* Hiperbólica: exactamente el camino de `@similar` (`directo::IndiceBola::knn`, distancia de Poincaré).
* Euclídea: k vecinos exactos por distancia euclídea (f64), desempate por índice.
* **FS montado**: para cada corpus se monta el FS con la configuración hiperbólica elegida en d = 2 y
  d = 16 (semilla 1 de la lista) y, para las 50 primeras consultas de test, `ls -U @similar/<q>` tiene
  que dar exactamente los 10 del arnés. Si no coincide, el experimento no vale.
* Contexto (no entra en la regla): TF‑IDF coseno 1024d (= FAISS IndexFlatIP) sobre las mismas consultas.

## 3. Entrenamiento (idéntico para las dos geometrías)

* Señal: el mismo grafo que usa el FS, 5 vecinos por coseno TF‑IDF (`ajuste::vecinos_tfidf`). Las
  etiquetas de relevancia **no** se usan para entrenar.
* Objetivo: `L = d(i,j) + log Σ_{k∈{j}∪neg} exp(−d(i,k))`, 10 negativos (Nickel & Kiela), mismo
  muestreo, misma inicialización (uniforme ±1e‑3) y mismo flujo del generador splitmix64.
  * Hiperbólica: `d` de Poincaré, SGD Riemanniano (gradiente × `(1−|u|²)²/4`), retracción a `|x| ≤ r`
    (= `ajuste::ajustar` del FS, con `r` como parámetro).
  * Euclídea, nativa: `d = |u−v|`, SGD plano, retracción a `|x| ≤ R` con `R = 2·artanh(r)` (misma
    distancia máxima al origen que la bola).
* **Épocas: 50** para todo (las primeras 10 con lr/10, como el FS). Dimensiones **2, 4, 8, 16**.
* **Grilla (10 configuraciones, igual para ambas):** lr ∈ {0.03, 0.1, 0.3, 1, 3} × r ∈ {0.95, 0.999}
  (euclídea: R ∈ {3.664, 7.601}).
* **Semillas de entrenamiento (3):** `0x5EED_F00D`, `0xA11CE`, `0xB0B`.
* Selección por (corpus, geometría, dim): la configuración con mayor recall@1 medio en **val** sobre las
  3 semillas; empate → mayor P@10 en val → menor lr → menor r. Test se mide una sola vez, con esa configuración.

## 4. Regla de decisión (fijada ahora)

Por **corpus × dim**, por separado, y por semilla s: `Δ_s = R@1_H − R@1_E` (puntos porcentuales) sobre
las 1000 consultas de test; IC95 bootstrap pareado por consulta (10 000 remuestreos, percentil,
semilla 20261026).
* **H GANA** en la celda si en las **3** semillas `Δ_s ≥ +2,0` y el límite inferior del IC95 > 0.
* **E GANA** si en las 3 semillas `Δ_s ≤ −2,0` y el límite superior del IC95 < 0.
* Si no, **sin diferencia demostrada**.
* Se informa cada corpus y cada dim por separado. Si H gana solo en 2–4d y no en 16d, es un resultado
  válido y se dice así: "la geometría aporta en baja dimensión; a 16d (la del FS) no".
* Consecuencia acordada (Apex): si H no gana en ninguna celda, la geometría hiperbólica se cae y el
  sistema 2 queda como "interfaz con valor".
* P@10 y la distorsión se informan con su IC/mediana, pero no deciden.

## 5. Secundario / confirmación (no decide)

* **Distorsión** de cada embedding respecto del árbol: 20 000 pares de docs de test (semilla 20261027),
  `d_T` = distancia en el árbol (`|a|+|b|−2·lcp`), `s = mediana(d_T/d_E)`,
  `D = media |s·d_E − d_T| / d_T`; y Spearman ρ(d_E, d_T).
* **Confirmación publicada**: mismo entrenamiento pero con el grafo = aristas del árbol (nodos internos
  incluidos), misma grilla y semillas, configuración elegida por menor D medio (dentro de muestra:
  mide ajuste, no generalización). Se espera que H gane en baja dimensión; no cuenta para el valor.

## 6. Presupuesto y cosas fijas

* Cómputo acotado (< ~40 min en total). Si no alcanza, se informa qué faltó; no se cambia la grilla.
* Se reportan medianas sobre semillas y los IC; todo el crudo (TSV por corrida, por consulta) se versiona.
* Código del FS: se agrega `r` a `ParamsAjuste` (por defecto 0,95, misma huella si no cambia) y las
  opciones `--lr` y `--radio-ajuste` para poder montar la configuración elegida; los 68 tests siguen en verde.
