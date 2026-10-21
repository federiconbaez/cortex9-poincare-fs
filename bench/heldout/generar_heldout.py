"""Conjunto RESERVADO (held-out) para el veredicto del umbral #2, fijado ANTES de
re-medir o ajustar nada (condición de auditoría de Research, 21/10/2026).

Determinista: splitmix64 (mismo generador que src/ajuste.rs::Rng), sin random de Python.
Uso: python3 bench/heldout/generar_heldout.py /tmp/pfs-valor/c1k bench/heldout
Escribe:
  t1p-semillas.tsv  : 200 semillas T1' (disjuntas de las 200 originales), nombre original y id anónimo dNNNNN
  t2p-consultas.tsv : 54 consultas T2' automáticas (27 temas x {K: términos de IDF alto, S: tramo contiguo 3-8 palabras})
  MANIFEST.txt      : semillas, huella del corpus y sha256 de cada archivo
Relevante = mismo tema (prefijo del nombre original). El documento fuente de cada
consulta T2' queda en el corpus (como pidió Research); se registra en la columna fuente."""
import hashlib, os, re, sys

M = (1 << 64) - 1
class Rng:
    def __init__(s, x): s.x = x & M
    def next(s):
        s.x = (s.x + 0x9E3779B97F4A7C15) & M
        z = s.x
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & M
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & M
        return z ^ (z >> 31)
    def indice(s, n): return s.next() % n

def barajar(v, seed):
    r = Rng(seed)
    for i in range(len(v) - 1, 0, -1):
        j = r.indice(i + 1); v[i], v[j] = v[j], v[i]
    return v

STOP = set("""the a an and or of to in on for with by is are be it this that from as at into its their them then than which when where how
what who was were has have had not no but if else can will would should could may might do does did so such these those there here also
using use used about over under between all any each other some more most only own same very just one two new get set we you your our
they he she his her i me my up out via per like way back later none true false self return returns default value values must need needs
because being been after before while until since whether otherwise""".split())

SEMILLA_T1P = 20261021
SEMILLA_T2P = 20261022

def palabras(t): return [p.lower() for p in re.split(r"[^0-9A-Za-z_]+", t) if p]

def main():
    corpus, out = sys.argv[1], sys.argv[2]
    nombres = sorted(f[:-4] for f in os.listdir(corpus) if f.endswith(".txt"))
    huella = hashlib.sha256()
    textos = {}
    for n in nombres:
        b = open(os.path.join(corpus, n + ".txt"), "rb").read()
        huella.update(n.encode() + b"\0" + len(b).to_bytes(8, "little") + b)
        textos[n] = b.decode("utf-8", "replace")
    tema = lambda n: n.split("-")[0]
    # anonimización idéntica a src/valor.rs (Rng(2026)); el índice del corpus = i de dNNNNN
    anon = barajar(list(nombres), 2026)
    pos = {n: i for i, n in enumerate(anon)}
    N = len(anon)
    # semillas T1 originales: Rng(7), 200 primeras, como valor.rs
    orig = sorted(barajar(list(range(N)), 7)[:200])
    resto = [i for i in range(N) if i not in set(orig)]
    t1p = sorted(barajar(resto, SEMILLA_T1P)[:200])
    assert not set(t1p) & set(orig) and len(t1p) == 200
    with open(os.path.join(out, "t1p-semillas.tsv"), "w") as f:
        f.write("# id_anon\tnombre_original\ttema   (T1': 200 semillas, splitmix64 semilla %d, disjuntas de las 200 de T1)\n" % SEMILLA_T1P)
        for i in t1p:
            f.write(f"d{i:05d}\t{anon[i]}\t{tema(anon[i])}\n")
    # T2': por tema, un doc fuente (no es semilla de T1 ni de T1') y dos consultas
    df = {}
    for n in nombres:
        for w in set(palabras(textos[n])): df[w] = df.get(w, 0) + 1
    excl = set(orig) | set(t1p)
    temas = sorted(set(map(tema, nombres)))
    r = Rng(SEMILLA_T2P)
    filas = []
    for t in temas:
        cand = sorted(i for i in range(N) if tema(anon[i]) == t and i not in excl) or sorted(i for i in range(N) if tema(anon[i]) == t)
        # K: 3 términos al azar entre los 8 de IDF más alto (df >= 2, >= 4 letras, sin vacías ni números)
        for intento in range(50):
            src = cand[r.indice(len(cand))]
            ws = sorted({w for w in palabras(textos[anon[src]]) if len(w) >= 4 and w not in STOP and not w.isdigit() and df[w] >= 2},
                        key=lambda w: (df[w], w))[:8]
            if len(ws) >= 3: break
        elegidas = []
        while len(elegidas) < 3:
            w = ws[r.indice(len(ws))]
            if w not in elegidas: elegidas.append(w)
        filas.append((t, "K", " ".join(elegidas), f"d{src:05d}"))
        # S: tramo contiguo de 3 a 8 palabras de otro doc fuente
        for intento in range(50):
            src = cand[r.indice(len(cand))]
            toks = palabras(textos[anon[src]])
            if len(toks) >= 8: break
        largo = 3 + r.indice(6)
        ini = r.indice(len(toks) - largo + 1)
        filas.append((t, "S", " ".join(toks[ini:ini + largo]), f"d{src:05d}"))
    with open(os.path.join(out, "t2p-consultas.tsv"), "w") as f:
        f.write("# tema\ttipo\tconsulta\tfuente   (T2': generadas automáticamente, splitmix64 semilla %d; K = 3 términos de IDF alto, S = tramo contiguo de 3-8 palabras)\n" % SEMILLA_T2P)
        for fl in filas: f.write("\t".join(fl) + "\n")
    sha = lambda p: hashlib.sha256(open(os.path.join(out, p), "rb").read()).hexdigest()
    with open(os.path.join(out, "MANIFEST.txt"), "w") as f:
        f.write(f"corpus: {corpus} ({N} docs, {len(temas)} temas), huella sha256(nombre\\0 largo contenido...) = {huella.hexdigest()}\n")
        f.write(f"semillas: T1' {SEMILLA_T1P}, T2' {SEMILLA_T2P}; anonimización Rng(2026), T1 original Rng(7) (iguales a src/valor.rs)\n")
        f.write(f"T1': {len(t1p)} semillas, intersección con T1 = 0; T2': {len(filas)} consultas\n")
        for p in ["generar_heldout.py", "t1p-semillas.tsv", "t2p-consultas.tsv"]:
            f.write(f"sha256 {sha(p)}  {p}\n")
    print(open(os.path.join(out, "MANIFEST.txt")).read(), end="")
    # control: T1 original coincide con lo exportado por valor.rs
    exp = "/tmp/pfs-valor/exp-1k/semillas.txt"
    if os.path.exists(exp):
        assert [int(x) for x in open(exp).read().split()] == orig, "T1 original no coincide con valor.rs"
        print("control: T1 original == /tmp/pfs-valor/exp-1k/semillas.txt")

main()
