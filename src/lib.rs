//! # poincare-fs
//!
//! Sistema de archivos FUSE donde los directorios son proyecciones dinámicas
//! sobre el disco de Poincaré y `cd` aplica una transformación de Möbius.
pub mod ajuste;
pub mod bola;
pub mod embed;
pub mod es;
pub mod fs;
pub mod generar;
pub mod hiper;
pub mod proyeccion;
