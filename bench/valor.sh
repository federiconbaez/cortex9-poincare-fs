#!/bin/sh
# Experimento de VALOR (ver VALOR.md): poincare-fs contra ripgrep, TF-IDF plano y FAISS plano.
# Uso: sh bench/valor.sh [dir_de_trabajo]   (por defecto /tmp/pfs-valor)
# FAISS: PY=/ruta/a/python con faiss-cpu (por defecto /tmp/pfs-faiss-venv/bin/python;
#   se crea con: python3 -m venv /tmp/pfs-faiss-venv && /tmp/pfs-faiss-venv/bin/pip install faiss-cpu numpy)
set -eu
cd "$(dirname "$0")/.."
cargo build --release -q
B="$PWD/target/release/poincare-fs"
W="${1:-/tmp/pfs-valor}"
PY="${PY:-/tmp/pfs-faiss-venv/bin/python}"
mkdir -p "$W"
[ -d "$W/c1k" ] || "$B" generar stdlib 1000 "$W/c1k"
echo "## versiones: $(rg --version | head -1); kernel $(uname -r); $(nproc) núcleos; fecha $(date -Is)"
echo "## corpus de 18 documentos (6 temas), solo T1"
"$B" valor corpus --reps 5 --tsv bench/valor-c18.tsv --exportar "$W/exp-c18"
"$PY" bench/valor_faiss.py "$W/exp-c18" 5
echo
echo "## corpus stdlib 1000 documentos (27 temas), T1 + T2"
"$B" valor "$W/c1k" --consultas bench/consultas-stdlib.tsv --reps 5 --semillas 200 --tsv bench/valor-stdlib1k.tsv --exportar "$W/exp-1k"
"$PY" bench/valor_faiss.py "$W/exp-1k" 5
