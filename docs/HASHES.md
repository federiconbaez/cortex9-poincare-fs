# Hashes / Historial reescrito

**ES:** El 2026-10-22 se reescribió todo el historial de este repositorio (git-filter-repo) para quitar datos del host y del usuario (nombre de máquina, usuario, uid, rutas de cgroup, versión de kernel). Los commits cambiaron de hash; los documentos de auditoría citan los hashes viejos. Esta tabla da la equivalencia.

**EN:** On 2026-10-22 this repository's full history was rewritten (git-filter-repo) to remove host and user data. Commit hashes changed; audit documents cite the old hashes. This table maps old → new.

**Nota / Note:** el sha256 del tarball citado en AUDIT-valor-v03.md (`6b964bc4…8487fe`) se calculó sobre el historial previo a la reescritura / was computed on the pre-rewrite history.

| Viejo / Old | Nuevo / New | Fecha (UTC) | Mensaje / Message |
|---|---|---|---|
| `c623319` | `c623319` | 2026-10-21T01:43:08Z | Inicio: alcance, dueña y criterio de verificado |
| `e80db4b` | `e80db4b` | 2026-10-21T01:57:15Z | poincare-fs: FUSE sobre el disco de Poincaré, Möbius en cd, tests de propiedades y de montaje real, README y VERIFICACION |
| `7f80fea` | `7f80fea` | 2026-10-21T02:32:36Z | poincare-fs v0.2: bola d-dimensional con girovectores, embedding RSGD, trayectoria, io_uring con respaldo pread, LRU, generadores de corpus y benchmark |
| `d69f8cb` | `d69f8cb` | 2026-10-21T02:33:45Z | VERIFICACION v0.2 completa (clon nuevo, io_uring/pread, calidad, benchmark, sesión montada); README: io_uring vs pread medido |
| `e3b1ef2` | `255d687` | 2026-10-21T03:07:01Z | Experimento de VALOR: poincare-fs contra ripgrep, TF-IDF plano y FAISS (IndexFlatIP); arnés 'valor', consultas fijas, resultados y VALOR.md (veredicto: no pasa el umbral) |
| `849983a` | `4e5c018` | 2026-10-21T03:07:13Z | VALOR.md: precisión sobre qué consultas falla el agente |
| `20cc898` | `2fa0a92` | 2026-10-21T03:09:38Z | Auditoría de Research del valor: confirma FALLA del umbral #2 |
| `e45e51e` | `104e966` | 2026-10-21T03:19:00Z | Conjunto reservado T1'/T2' + pre-registro (antes de re-medir o ajustar el sistema 2) |
| `e483cfb` | `3e49291` | 2026-10-21T03:30:12Z | Adenda: T1'' reservado para el diseño post-hoc @similar-hibrido (antes de medirlo); declara desvío |
| `7196986` | `b931ee4` | 2026-10-21T03:32:04Z | poincare-fs v0.3: acceso directo por FUSE (@doc, @similar, @similar-hibrido, @buscar, @buscar-bola), árbol de cobertura (alcance completo), índices SoA f32; arnés valor-directo + FAISS; tests nuevos |
| `3d09884` | `a93135a` | 2026-10-21T03:50:21Z | VALOR v0.3: resultados en conjuntos reservados (T1', T1'', T2'), 2 corridas + 1 intercalada, veredicto por celda; README v0.3; modo PFS_INTERCALAR |
| `42bb6ce` | `f43418e` | 2026-10-21T04:02:17Z | Auditoría de Research v0.3: umbral #2 pre-registrado FALLA (T1'); T2' pasa en caliente; ablación euclídea 16d iguala o supera a la hiperbólica; crudo de re-corrida |
| `1403985` | `96634f1` | 2026-10-21T04:08:25Z | Pre-registro geometría: corpus jerárquicos (WordNet 3.0, Documentation de linux-6.12), conjuntos reservados val/test, regla de decisión, antes de entrenar o medir |
| `4ce7080` | `7472db4` | 2026-10-21T04:21:55Z | Geometría jerárquica: H vs E nativa (d=2/4/8/16) en WordNet y Documentation; valor no demostrado |
| `7cd0d9b` | `a7bbcf0` | 2026-10-22 | Licencia Apache-2.0 (mensaje reescrito con formato nuevo / rewritten message) |
