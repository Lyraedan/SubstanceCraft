//! SubstanceCraft import/export.
//!
//! Mesh import (glTF 2.0, OBJ) into the document model, texture import/export, export
//! presets, channel naming conventions and OpenPBR material mapping. All formats are
//! parsed from their public specifications.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod mesh_import;

pub use mesh_import::{load_gltf, load_obj, load_scene};

use substancecraft_doc::Scene;
use thiserror::Error;

/// Errors from import/export. Everything is recoverable and reported as a message.
#[derive(Debug, Error)]
pub enum IoError {
    /// The operating system refused a read or write.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The OBJ parser rejected the file.
    #[error("invalid OBJ: {0}")]
    Obj(String),
    /// The glTF parser rejected the file.
    #[error("invalid glTF: {0}")]
    Gltf(String),
    /// The mesh failed a geometry invariant.
    #[error("invalid geometry: {0}")]
    Geometry(#[from] substancecraft_geom::GeometryError),
    /// The file extension is not a supported mesh format.
    #[error("unsupported mesh format: {0}")]
    UnsupportedFormat(String),
    /// The file contained no drawable geometry.
    #[error("the file contains no geometry")]
    Empty,
}

/// A small procedural demo model (a torus) so the viewport shows something with no assets.
pub fn demo_scene() -> Scene {
    match substancecraft_geom::primitives::torus(1.0, 0.35, 128, 64) {
        Ok(mesh) => Scene::from_mesh(mesh),
        Err(_) => Scene::default(),
    }
}
