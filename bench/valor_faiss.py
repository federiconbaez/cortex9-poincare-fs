"""Línea base FAISS (IndexFlatIP, búsqueda exacta) sobre los MISMOS vectores TF-IDF
1024d que exporta `poincare-fs valor --exportar DIR`. Mide una consulta por llamada
(como los demás métodos). No incluye vectorizar la consulta (viene precalculada).
Uso: python bench/valor_faiss.py DIR [reps]"""
import sys, time
import numpy as np
import faiss

d = sys.argv[1]
reps = int(sys.argv[2]) if len(sys.argv) > 2 else 5
faiss.omp_set_num_threads(1)
X = np.fromfile(f"{d}/docs.f32", dtype="<f4").reshape(-1, 1024)
temas = open(f"{d}/temas.txt").read().split()
n = len(temas)
tam = {t: temas.count(t) for t in set(temas)}
t0 = time.perf_counter()
idx = faiss.IndexFlatIP(1024)
idx.add(X)
print(f"faiss {faiss.__version__}: índice IndexFlatIP con {n} vectores en {1e3*(time.perf_counter()-t0):.3f} ms (1 hilo)")


def p50p99(v):
    v = sorted(v)
    q = lambda p: v[round(p / 100 * (len(v) - 1))]
    return 1e3 * q(50), 1e3 * q(99)


def correr(nombre, consultas, excluir):
    ok = 0; p10 = 0.0; ts = []
    for rep in range(reps):
        for qv, t, s in consultas:
            t0 = time.perf_counter()
            _, I = idx.search(qv, 11 if excluir else 10)
            ts.append(time.perf_counter() - t0)
            lista = [i for i in I[0] if i != s][:10]
            if rep == 0:
                ok += temas[lista[0]] == t
                tope = max(1, min(10, tam[t] - (1 if excluir else 0)))
                p10 += sum(temas[i] == t for i in lista) / tope
    a, b = p50p99(ts)
    m = len(consultas)
    print(f"| {nombre} | FAISS IndexFlatIP (TF-IDF 1024d) | {m} | {100*ok/m:.1f} % ({ok}/{m}) | {p10/m:.3f} | {a:.3f} | {b:.3f} | 0.0 |")


sem = [int(x) for x in open(f"{d}/semillas.txt").read().split()]
print("| tarea | método | n | éxito@1 | P@10 | p50 (ms) | p99 (ms) | acciones |")
print("|---|---|---:|---:|---:|---:|---:|---:|")
correr("T1", [(X[s:s + 1], temas[s], s) for s in sem], True)
try:
    Q = np.fromfile(f"{d}/consultas.f32", dtype="<f4").reshape(-1, 1024)
    meta = [l.split("\t") for l in open(f"{d}/consultas.txt").read().splitlines()]
    for tipo in ["K", "D", None]:
        cs = [(Q[i:i + 1], meta[i][0], -1) for i in range(len(meta)) if tipo is None or meta[i][1] == tipo]
        correr(f"T2-{tipo or 'todas'}", cs, False)
except FileNotFoundError:
    pass
