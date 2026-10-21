#!/bin/sh
set -eu
AQUI=$(cd "$(dirname "$0")" && pwd)
RAIZ=$(cd "$AQUI/../.." && pwd)
SAL=${SAL:-/tmp/geo}
B="$RAIZ/target/release/examples/geometria_check"
cargo build --release --example geometria_check --manifest-path "$RAIZ/Cargo.toml"
# wordnet H d=2/16 y kernel H d=2/16 según elegidas.tsv
python3 - <<'PY' "$AQUI" "$SAL" "$B"
import sys
aqui, sal, binario = sys.argv[1:4]
import subprocess
for corpus in ("wordnet", "kernel"):
    eleg = {}
    for line in open(f"{aqui}/resultados/{corpus}/elegidas.tsv"):
        if line.startswith("geom") or not line.strip():
            continue
        g, d, lr, r = line.split()
        if g == "H" and d in ("2", "16"):
            eleg[d] = (lr, r)
    for d, (lr, r) in eleg.items():
        subprocess.check_call([binario, f"{sal}/corpus-{corpus}", d, lr, r, "5eedf00d",
                               f"{aqui}/{corpus}-test.tsv"])
print("montaje-geometria: OK")
PY
