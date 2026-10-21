#!/bin/sh
# Corre el experimento pre-registrado en PREREGISTRO-geometria.md.
# 1) reconstruye los corpus (deterministas) y verifica los sha256 del pre-registro;
# 2) corre el arnés Rust `geometria` (ejemplo de cargo, implementado en el commit de resultados);
# 3) verifica con el FS montado que `ls -U @similar/<q>` coincide con el arnés.
set -eu
AQUI=$(cd "$(dirname "$0")" && pwd)
RAIZ=$(cd "$AQUI/../.." && pwd)
SAL=${SAL:-/tmp/geo}
KDOC=${KDOC:-$SAL/linux-6.12/Documentation}
python3 "$AQUI/construir_corpus.py" --kernel "$KDOC" --salida "$SAL"
(cd "$AQUI" && sha256sum -c MANIFEST-geometria.txt)
cd "$RAIZ"
cargo build --release --examples --bin poincare-fs
for C in wordnet kernel; do
  ./target/release/examples/geometria "$SAL/corpus-$C" "$AQUI/$C-manifiesto.tsv" \
     "$AQUI/$C-val.tsv" "$AQUI/$C-test.tsv" "$AQUI/resultados/$C"
done
sh "$AQUI/montaje-geometria.sh"
