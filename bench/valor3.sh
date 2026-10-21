#!/bin/sh
# Re-intento del VALOR v0.3 (ver VALOR.md): acceso directo por FUSE, cobertura, conjuntos reservados.
# Uso: sh bench/valor3.sh [corrida]   -> bench/valor3-corrida<N>.txt y bench/valor3-corrida<N>.tsv
set -eu
cd "$(dirname "$0")/.."
cargo build --release -q
N="${1:-1}"
B="$PWD/target/release/poincare-fs"
W=/tmp/pfs-valor
PY="${PY:-/tmp/pfs-faiss-venv/bin/python}"
[ -x "$PY" ] || { python3 -m venv /tmp/pfs-faiss-venv && /tmp/pfs-faiss-venv/bin/pip install -q faiss-cpu numpy; }
[ -d "$W/c1k" ] || "$B" generar stdlib 1000 "$W/c1k"
O="bench/valor3-corrida$N.txt"
{
  echo "## commit $(git rev-parse --short HEAD); $(rg --version | head -1); kernel $(uname -r); $(nproc) núcleos; $(date -Is)"
  "$B" valor-directo "$W/c1k" --reps 5 --tsv "bench/valor3-corrida$N.tsv" --exportar "$W/exp3-$N"
  echo
  echo "## FAISS"
  "$PY" bench/valor_faiss_directo.py "$W/exp3-$N" 5
} > "$O" 2>&1
cat "$O"
