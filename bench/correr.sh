#!/bin/sh
# Benchmark reproducible: genera los corpus, indexa y mide ls/cd/cat montado.
# Uso: sh bench/correr.sh [dir_de_trabajo]   (por defecto /tmp/poincare-bench-corpus)
set -eu
cd "$(dirname "$0")/.."
cargo build --release -q
B="$PWD/target/release/poincare-fs"
W="${1:-/tmp/poincare-bench-corpus}"
mkdir -p "$W"
[ -d "$W/c18" ] || cp -r corpus "$W/c18"
[ -d "$W/stdlib-1k" ] || "$B" generar stdlib 1000 "$W/stdlib-1k"
[ -d "$W/sint-10k" ] || "$B" generar sintetico 10000 "$W/sint-10k"
for c in c18 stdlib-1k sint-10k; do
  "$B" indexar "$W/$c" "$W/$c.idx"
done
echo "carga antes: $(cat /proc/loadavg)"
for c in c18 stdlib-1k sint-10k; do
  for rep in 1 2 3; do
    echo "## $c cache=4096 repetición $rep"
    "$B" bench "$W/$c" --pasos 2000 --cache 4096 --semilla "$rep" --indice "$W/$c.idx" 2>/dev/null
  done
  pasos=2000; [ "$c" = sint-10k ] && pasos=300
  echo "## $c cache=0 (sin LRU)"
  "$B" bench "$W/$c" --pasos "$pasos" --cache 0 --semilla 1 --indice "$W/$c.idx" 2>/dev/null
done
echo "carga después: $(cat /proc/loadavg)"
