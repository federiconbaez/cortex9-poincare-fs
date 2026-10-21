#!/usr/bin/env python3
"""Construye, de forma determinista y sin red, los dos corpus jerárquicos del
experimento de geometría (PREREGISTRO-geometria.md) y sus conjuntos reservados.

Entradas fijadas (sha256 en el pre-registro):
  * WordNet 3.0 de nltk_data (`~/nltk_data/corpora/wordnet.zip`, ya descomprimido).
  * linux-6.12.tar.xz de kernel.org (solo `Documentation/`, extraído en --kernel).

Uso: construir_corpus.py --kernel /tmp/geo/linux-6.12/Documentation --salida /tmp/geo
Escribe:  <salida>/corpus-wordnet/*.txt, <salida>/corpus-kernel/*.txt  (texto, NO versionado)
          ./wordnet-manifiesto.tsv, ./kernel-manifiesto.tsv            (versionado)
          ./wordnet-val.tsv, ./wordnet-test.tsv, ./kernel-val.tsv, ./kernel-test.tsv
"""
import argparse, collections, hashlib, os, random, sys

SEMILLA_MUESTRA = 20261021   # muestra de synsets
SEMILLA_NOMBRES = 20261023   # permutación de nombres anónimos
SEMILLA_HELDOUT = 20261024   # val / test
N_WORDNET = 4000
NIVEL_WN = 5                 # clase = ancestro a profundidad 5 del camino canónico
PROF_MIN_WN = NIVEL_WN + 2   # docs al menos 2 niveles por debajo de la clase
MAX_BYTES_KERNEL = 8192
N_VAL, N_TEST = 500, 1000


def sha(b):
    return hashlib.sha256(b).hexdigest()


def camino_canonico(s):
    ps = s.hypernym_paths()
    ps.sort(key=lambda p: (len(p), [x.name() for x in p]))
    return ps[0]


def wordnet():
    from nltk.corpus import wordnet as wn
    assert wn.get_version() == "3.0", wn.get_version()
    syn = sorted(wn.all_synsets("n"), key=lambda s: s.name())
    caminos = {s.name(): camino_canonico(s) for s in syn}
    univ = [s for s in syn if len(caminos[s.name()]) - 1 >= PROF_MIN_WN]
    muestra = random.Random(SEMILLA_MUESTRA).sample(univ, N_WORDNET)
    muestra.sort(key=lambda s: s.name())
    docs = []
    for s in muestra:
        lemas = " ".join(l.name().replace("_", " ") for l in s.lemmas())
        texto = f"{lemas}. {s.definition()}\n"
        camino = [x.name() for x in caminos[s.name()]]
        docs.append((s.name(), camino[NIVEL_WN], "/".join(camino), texto))
    return docs


def kernel(raiz):
    docs = []
    for d, subdirs, archivos in os.walk(raiz):
        subdirs.sort()
        rel = os.path.relpath(d, raiz)
        if rel == "translations" or rel.startswith("translations" + os.sep):
            continue
        for a in sorted(archivos):
            if a.endswith(".rst"):
                docs.append(os.path.normpath(os.path.join(rel, a)))
    por_dir = collections.Counter(os.path.dirname(p) for p in docs)
    out = []
    for p in sorted(docs):
        dirn = os.path.dirname(p) or "."
        if por_dir[os.path.dirname(p)] < 2:
            continue
        with open(os.path.join(raiz, p), "rb") as f:
            b = f.read(MAX_BYTES_KERNEL)
        texto = b.decode("utf-8", errors="replace")
        out.append((p, dirn, "Documentation/" + p, texto))
    return out


def escribir(nombre, prefijo, docs, salida, aqui):
    dirc = os.path.join(salida, f"corpus-{nombre}")
    os.makedirs(dirc, exist_ok=True)
    for f in os.listdir(dirc):
        os.remove(os.path.join(dirc, f))
    perm = list(range(len(docs)))
    random.Random(SEMILLA_NOMBRES).shuffle(perm)
    filas = []
    for k, i in enumerate(perm):
        ident, clase, camino, texto = docs[i]
        anon = f"{prefijo}{k:05d}"
        b = texto.encode("utf-8")
        with open(os.path.join(dirc, anon + ".txt"), "wb") as f:
            f.write(b)
        filas.append((anon, clase, camino, ident, sha(b)))
    filas.sort()
    man = "nombre\tclase\tcamino\torigen\tsha256\n" + "".join("\t".join(r) + "\n" for r in filas)
    with open(os.path.join(aqui, f"{nombre}-manifiesto.tsv"), "w") as f:
        f.write(man)
    tam = collections.Counter(r[1] for r in filas)
    elegibles = [r[0] for r in filas if tam[r[1]] >= 2]
    rng = random.Random(SEMILLA_HELDOUT + (0 if nombre == "wordnet" else 1))
    rng.shuffle(elegibles)
    val, test = sorted(elegibles[:N_VAL]), sorted(elegibles[N_VAL:N_VAL + N_TEST])
    assert len(test) == N_TEST and not set(val) & set(test)
    for parte, lista in (("val", val), ("test", test)):
        with open(os.path.join(aqui, f"{nombre}-{parte}.tsv"), "w") as f:
            f.write("nombre\n" + "".join(x + "\n" for x in lista))
    print(f"{nombre}: {len(filas)} docs, {len(tam)} clases, {sum(1 for v in tam.values() if v >= 2)} con ≥2, "
          f"elegibles {len(tam) and sum(v for v in tam.values() if v >= 2)}, val {len(val)}, test {len(test)}, "
          f"prof. camino {min(r[2].count('/') for r in filas)}–{max(r[2].count('/') for r in filas)}", file=sys.stderr)


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--kernel", required=True)
    ap.add_argument("--salida", required=True)
    a = ap.parse_args()
    aqui = os.path.dirname(os.path.abspath(__file__))
    escribir("wordnet", "wn", wordnet(), a.salida, aqui)
    escribir("kernel", "kd", kernel(a.kernel), a.salida, aqui)


if __name__ == "__main__":
    main()
