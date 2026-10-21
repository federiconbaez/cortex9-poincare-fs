"""Línea base FAISS (IndexFlatIP exacto, 1 hilo) para el re-intento v0.3, sobre los
MISMOS vectores TF-IDF 1024d exportados por `poincare-fs valor-directo`. Misma forma
de medir que bench/valor_faiss.py: una consulta por llamada, sin vectorizar la consulta.
Uso: python bench/valor_faiss_directo.py DIR [reps]"""
import sys, time
import numpy as np
import faiss

d = sys.argv[1]
reps = int(sys.argv[2]) if len(sys.argv) > 2 else 5
faiss.omp_set_num_threads(1)
X = np.fromfile(f"{d}/docs.f32", dtype="<f4").reshape(-1, 1024)
temas = open(f"{d}/temas.txt").read().split()
tam = {t: temas.count(t) for t in set(temas)}
idx = faiss.IndexFlatIP(1024)
idx.add(X)
print(f"faiss {faiss.__version__}, IndexFlatIP, {len(temas)} vectores, 1 hilo")


def pct(v, p):
    v = sorted(v)
    return 1e3 * v[round(p / 100 * (len(v) - 1))]


def correr(conj, consultas, excluir):
    ok = 0; p10 = 0.0; frio = []; ts = []
    for rep in range(reps):
        for qv, t, s in consultas:
            t0 = time.perf_counter()
            _, I = idx.search(qv, 11 if excluir else 10)
            dt = time.perf_counter() - t0
            (frio if rep == 0 else ts).append(dt)
            lista = [i for i in I[0] if i != s][:10]
            if rep == 0:
                ok += temas[lista[0]] == t
                tope = max(1, min(10, tam[t] - (1 if excluir else 0)))
                p10 += sum(temas[i] == t for i in lista) / tope
    m = len(consultas)
    print(f"| {conj} | faiss-flatip | {m} | {100*ok/m:.1f} % ({ok}/{m}) | {p10/m:.3f} | {pct(ts,50):.4f} | {pct(ts,99):.4f} | {pct(frio,50):.4f} |")


print("| conjunto | método | n | recall@1 | P@10 | p50 caliente (ms) | p99 caliente (ms) | p50 frío (ms) |")
print("|---|---|---:|---:|---:|---:|---:|---:|")
for linea in open(f"{d}/conjuntos.tsv").read().splitlines():
    tarea, conj, f = linea.split("\t")
    if tarea == "T1":
        sem = [int(x) for x in open(f"{d}/{f}.txt").read().split()]
        correr(conj, [(X[s:s + 1], temas[s], s) for s in sem], True)
    else:
        Q = np.fromfile(f"{d}/{f}.f32", dtype="<f4").reshape(-1, 1024)
        meta = [l.split("\t") for l in open(f"{d}/{f}.txt").read().splitlines()]
        correr(conj, [(Q[i:i + 1], meta[i][0], -1) for i in range(len(meta))], False)
