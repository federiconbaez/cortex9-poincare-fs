# Resultados: geometría hiperbólica vs euclídea en datos jerárquicos

Fecha: 21/10/2026 (hora de Buenos Aires, UTC−3).  
Pre-registro: commit `1403985` (`PREREGISTRO-geometria.md`).  
Código + mediciones: este commit (posterior).

## Veredicto global

**La geometría hiperbólica no aporta a la tarea del FS** (`@similar`: recall@1) en ninguno de
los dos corpus × dimensiones 2/4/8/16. En las 8 celdas el veredicto es **sin diferencia
demostrada** (regla: H gana solo si Δ≥+2 pp y IC95>0 en las 3 semillas). En varias celdas
la mediana de E es igual o mayor. Consecuencia acordada: el sistema 2 queda como
**«interfaz con valor»**; la geometría hiperbólica se cae como aportación demostrada.

La confirmación publicada (entrenar sobre aristas del árbol) sí reproduce el resultado
conocido: H distorsiona menos que E en baja dimensión (p. ej. WordNet d=2: D 0,24 contra 0,44).
Eso confirma la teoría; **no** cuenta para el valor del FS.

## Baseline TF‑IDF (test, 1000 consultas)

| corpus | R@1 | P@10 |
|---|---:|---:|
| WordNet | 45,40 | 0,330 |
| Kernel Documentation | 59,30 | 0,417 |

## Tabla principal (configuración elegida en val; test una sola vez)

Δ = R@1_H − R@1_E (pp) por semilla; IC95 bootstrap pareado (10 000, semilla 20261026).

### WordNet (4000 docs, clase = ancestro profundidad 5)

| geom | dim | lr | r | R@1 test (3 semillas) | mediana R@1 | mediana P@10 | D | ρ | Δ (pp) | IC95 Δ | veredicto |
|---|---:|---:|---:|---|---:|---:|---:|---:|---|---|---|
| H | 2 | 0,03 | 0,95 | 30,7 / 33,5 / 32,7 | 32,7 | 0,266 | 0,407 | 0,034 | +0,40 / +0,50 / −1,80 | cruza 0 | sin diferencia demostrada |
| E | 2 | 0,03 | 0,999 | 32,5 / 33,0 / 32,3 | 32,5 | 0,270 | 0,467 | 0,084 | — | — | — |
| H | 4 | 0,03 | 0,95 | 38,3 / 37,9 / 37,5 | 37,9 | 0,296 | 0,266 | 0,067 | −1,50 / −2,30 / −0,60 | cruza 0 | sin diferencia demostrada |
| E | 4 | 0,03 | 0,95 | 38,9 / 40,2 / 39,0 | 39,0 | 0,303 | 0,325 | 0,105 | — | — | — |
| H | 8 | 0,03 | 0,95 | 39,6 / 41,2 / 42,0 | 41,2 | 0,322 | 0,241 | 0,068 | +0,90 / −1,70 / −2,50 | cruza 0 | sin diferencia demostrada |
| E | 8 | 0,03 | 0,95 | 42,1 / 42,9 / 41,1 | 42,1 | 0,326 | 0,261 | 0,106 | — | — | — |
| H | 16 | 0,3 | 0,95 | 43,0 / 41,7 / 42,1 | 42,1 | 0,322 | 0,227 | 0,061 | −1,00 / −2,30 / −0,50 | cruza 0 | sin diferencia demostrada |
| E | 16 | 0,03 | 0,95 | 43,5 / 44,0 / 43,1 | 43,5 | 0,345 | 0,234 | 0,132 | — | — | — |

### Kernel Documentation (3071 docs, clase = directorio)

| geom | dim | lr | r | R@1 test (3 semillas) | mediana R@1 | mediana P@10 | D | ρ | Δ (pp) | IC95 Δ | veredicto |
|---|---:|---:|---:|---|---:|---:|---:|---:|---|---|---|
| H | 2 | 0,03 | 0,95 | 46,1 / 47,2 / 46,3 | 46,3 | 0,367 | 0,391 | 0,126 | +0,90 / +0,80 / +0,70 | cruza 0 | sin diferencia demostrada |
| E | 2 | 0,03 | 0,999 | 45,4 / 46,4 / 45,4 | 45,4 | 0,369 | 0,452 | 0,158 | — | — | — |
| H | 4 | 0,03 | 0,95 | 53,7 / 53,0 / 51,9 | 53,0 | 0,417 | 0,242 | 0,200 | −0,20 / −0,40 / +1,20 | cruza 0 | sin diferencia demostrada |
| E | 4 | 0,03 | 0,95 | 52,5 / 53,4 / 52,1 | 52,5 | 0,423 | 0,319 | 0,177 | — | — | — |
| H | 8 | 0,1 | 0,95 | 55,8 / 56,2 / 56,2 | 56,2 | 0,424 | 0,218 | 0,157 | +1,10 / +0,80 / +1,30 | cruza 0 (< +2) | sin diferencia demostrada |
| E | 8 | 0,03 | 0,95 | 54,5 / 55,4 / 55,1 | 55,1 | 0,438 | 0,252 | 0,193 | — | — | — |
| H | 16 | 0,1 | 0,95 | 56,7 / 56,8 / 56,5 | 56,7 | 0,427 | 0,215 | 0,164 | −0,20 / +0,60 / −0,50 | cruza 0 | sin diferencia demostrada |
| E | 16 | 0,3 | 0,95 | 57,2 / 56,2 / 56,7 | 56,7 | 0,467 | 0,213 | 0,140 | — | — | — |

Crudo por configuración: `resultados/{wordnet,kernel}/crudo.tsv`.  
Elegidas: `resultados/{wordnet,kernel}/elegidas.tsv`.

## Confirmación publicada (grafo = aristas del árbol; no decide)

| corpus | dim | mediana D (H) | mediana D (E) |
|---|---:|---:|---:|
| WordNet | 2 | 0,239 | 0,439 |
| WordNet | 4 | 0,213 | 0,301 |
| WordNet | 8 | 0,209 | 0,232 |
| WordNet | 16 | 0,203 | 0,204 |
| Kernel | 2 | 0,232 | 0,417 |
| Kernel | 4 | 0,199 | 0,271 |
| Kernel | 8 | 0,187 | 0,214 |
| Kernel | 16 | 0,180 | 0,185 |

H gana claro en distorsión a 2–4d; a 16d empatan. Coincide con Nickel & Kiela 2017 / Sala 2018.

## FS montado

Para H d=2 y d=16 (semilla `0x5EED_F00D`, lr/r elegidos en val) en ambos corpus:
`examples/geometria_check` reportó **50/50** consultas de test con
`@similar/<q>` idéntico a `IndiceBola::knn` (el camino real del FS).

## Cómputo

| etapa | tiempo |
|---|---|
| WordNet (240 entrenamientos + eval) | 94 s |
| Kernel (240 entrenamientos + eval) | 72 s |
| Confirmación WordNet | 236 s |
| Confirmación Kernel | 67 s |
| Montaje / check | ~50 s |
| **Total** | **~520 s (~8,7 min)** |

## Desvíos respecto del pre-registro

1. Ninguno en la regla de decisión, grilla, semillas, épocas ni corpus.
2. El knn del arnés de medición usa `bola::distancia` f64 exacta; el FS usa el prefiltro f32 +
   reorden f64 de `IndiceBola`. La equivalencia top‑10 se verificó montando el FS (50/50).
3. En la confirmación, el grado medio del árbol es 1,3–1,8 (se trunca a ≤5 vecinos como en el FS).

## Tests / clippy

* `cargo test --release`: **72** en verde (64 lib + 2 directo + 2 geometría + 3 montaje + 1 seccomp).
* `cargo clippy --all-targets -- -D warnings`: 0 avisos.
