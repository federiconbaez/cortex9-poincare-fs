# Auditoría de Research — experimento de VALOR (sistema 2)

- **Auditado:** HEAD `849983a` (sobre `d69f8cb`), árbol limpio. 21/10/2026 ~00:10 (Buenos Aires, UTC−3), en la box (8 núcleos, load 7–9).
- **Tests:** `cargo test --release` → 54 + 3 + 1 = 58 en verde. `cargo clippy --all-targets -- -D warnings` → 0 avisos.
- **Re-corrida independiente:** mismos pasos que `bench/valor.sh`, pero en `/tmp/pfs-audit` (corpus regenerado, idéntico byte a byte a `/tmp/pfs-valor/c1k`) y sin pisar los TSV versionados. Salida: [`bench/auditoria/rerun-research-2026-10-21.txt`](bench/auditoria/rerun-research-2026-10-21.txt).

## Coincidencia con la evidencia de Apex

Todos los aciertos (éxito@1 y P@10) coinciden **exactamente** en T1 (18 y 1000 docs) y T2 (K, D y total), igual que el alcance (127/200 sin llegar; 73/200 y 83/200; BFS 651/1000). Los tiempos caen dentro del ±20 % declarado, salvo T2 rg/TF-IDF/híbrido, que dieron un 20–60 % más lentos en esta corrida (por carga). Ningún ratio cambia de lado del umbral.

| ratio (mediana caliente, 1000 docs) | Apex | re-corrida |
|---|---:|---:|
| T1 FS / rg | 0,15× | 0,14× |
| T1 FS / FAISS | 7,5× | 6,7× |
| T2 agente FS / rg | 0,13× | 0,12× |
| T2 agente FS / FAISS | 7,1× | 7,3× |
| T1 bola 16d / FAISS | 0,73× | 0,74× |

## Veredicto contra el umbral #2 (aplicado literal)

**(a) poincare-fs como FS navegable: FALLA.** Le gana en tiempo a ripgrep, pero pierde en recall@1 (T1 31,5 % < 48,5 %; T2 22,2 % < 77,8 %). Contra FAISS plano pierde en tiempo (~7×) y en recall@1 (31,5 % < 84 %; 22,2 % < 88,9 %). Falla con las dos líneas base, se lea el umbral como «y» o como «o».

**(b) Bola 16d con acceso directo (no es parte del FS): FALLA contra FAISS, por tiempo** (0,73–0,74× > 0,70×). El recall@1 empata (85,0 % contra 84,0 %, 2/200 de diferencia, no significativo). Contra ripgrep pasaría (0,015×, 85 % ≥ 48,5 %), pero FAISS plano es la línea base más fuerte y el sistema no expone esta interfaz. No se mide T2 para la bola. Queda registrado como **indicio, no como valor demostrado**.

**Resultado: valor no demostrado.**

## Caveats
- La máquina estaba compartida y con carga. Se hizo una sola re-corrida, con 5 repeticiones y una sola semilla de muestreo, así que los tiempos son ruidosos, pero los aciertos son deterministas.
- FAISS se mide desde Python, lo que suma overhead de llamada a la línea base y favorece a la bola: aun así, la bola no llega al 0,70×.
- No se midió en `host-de-pruebas`, no hay tiempo humano y el corpus es solo el stdlib, no documentos del usuario.
