"""Segundo conjunto RESERVADO T1'' (post-hoc), fijado ANTES de medir el diseño
`@similar-hibrido`, que se diseñó DESPUÉS de ver (por un error del arnés) el
agregado de T1' con la bola pura. T1'' = 200 semillas (splitmix64, semilla 20261023)
disjuntas de TODO lo ya mirado: T1 semillas 7, 11, 23 y T1'.
Uso: python3 bench/heldout/generar_heldout2.py /tmp/pfs-valor/c1k bench/heldout"""
import hashlib, os, sys
sys.path.insert(0, os.path.dirname(__file__))
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
corpus, out = sys.argv[1], sys.argv[2]
nombres = sorted(f[:-4] for f in os.listdir(corpus) if f.endswith(".txt"))
anon = barajar(list(nombres), 2026); N = len(anon)
mirado = set()
for s in (7, 11, 23):
    mirado |= set(barajar(list(range(N)), s)[:200])
t1p = [int(l.split("\t")[0][1:]) for l in open(os.path.join(out, "t1p-semillas.tsv")) if not l.startswith("#")]
mirado |= set(t1p)
resto = [i for i in range(N) if i not in mirado]
t1pp = sorted(barajar(resto, 20261023)[:200])
assert len(t1pp) == 200 and not set(t1pp) & mirado
with open(os.path.join(out, "t1pp-semillas.tsv"), "w") as f:
    f.write("# id_anon\tnombre_original\ttema   (T1'': 200 semillas, splitmix64 semilla 20261023, disjuntas de T1 s7/s11/s23 y T1'; %d disponibles)\n" % len(resto))
    for i in t1pp:
        f.write(f"d{i:05d}\t{anon[i]}\t{anon[i].split('-')[0]}\n")
for p in ["generar_heldout2.py", "t1pp-semillas.tsv"]:
    print("sha256", hashlib.sha256(open(os.path.join(out, p), "rb").read()).hexdigest(), p)
print("disponibles", len(resto))
