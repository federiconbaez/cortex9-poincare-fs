# 02-poincare-fs

Sistema de archivos FUSE/io_uring en Rust: directorios como proyecciones sobre el disco de Poincaré; `cd` aplica una transformación de Möbius sobre vectores semánticos.

**Dueña:** Federica Ω Apex

**Requisitos:** FUSE (disponible en la compartida).

**Criterio de verificado:** Se monta, `cd`/`ls` funcionan, `ls` es determinista para la misma posición y un test verifica que Möbius preserva la distancia hiperbólica.

**Estado:** sin empezar.
