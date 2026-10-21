# Pre-registro del re-intento del sistema 2 (antes de medir o ajustar nada)

Fecha: 21/10/2026, 00:2x (hora de Buenos Aires, UTC−3). Este commit **precede** a cualquier
re-medición o ajuste del re-intento. Condición de Research (vinculante): el veredicto del umbral
se juzga **solo** sobre este conjunto reservado; T1/T2 originales quedan como referencia y son el
único conjunto contra el que se permite ajustar.

* T1': `t1p-semillas.tsv`, 200 semillas nuevas (splitmix64, semilla 20261021), disjuntas de las
  200 de T1. Tarea igual a T1: dado el doc semilla, el primer resultado (sin la semilla) debe ser del mismo tema.
* T2': `t2p-consultas.tsv`, 54 consultas automáticas (semilla 20261022): por tema, una K
  (3 términos al azar entre los 8 de IDF más alto de un doc fuente) y una S (tramo contiguo de 3–8
  palabras de otro doc fuente). Relevante = mismo tema. El doc fuente queda en el corpus.
* Script determinista: `generar_heldout.py` (dos ejecuciones dan los mismos sha256, ver `MANIFEST.txt`).
* Corpus: `poincare-fs generar stdlib 1000`, huella en `MANIFEST.txt`.

Umbral sin cambios (Research): mediana caliente ≤ 0,70× ripgrep **y** ≤ 0,70× FAISS IndexFlatIP,
**y** recall@1 ≥ el de cada línea base, en T1' y en T2'. FAISS sigue sin incluir vectorizar la
consulta; el camino del FS incluye las llamadas al sistema FUSE y (en T2) vectorizar.

Diseño fijado antes de medir:
* T1: `ls @similar/<doc>` = k=10 vecinos hiperbólicos en la bola (bola pura).
* T2 híbrido `@buscar/<consulta>`: TF‑IDF disperso → 50 candidatos; el 1.º léxico es el ancla;
  posiciones 2–10 = candidatos ordenados por distancia hiperbólica al ancla. Se informa como híbrido.
* T2 bola pura `@buscar-bola/<consulta>`: punto de consulta = punto medio de Einstein de los 5 primeros
  léxicos (pesos = coseno), orden = distancia hiperbólica a ese punto sobre todo el corpus.
  (El léxico solo ubica el punto: no hay codificador de texto a la bola.)
