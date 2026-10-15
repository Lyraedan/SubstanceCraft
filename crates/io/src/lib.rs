//! SubstanceCraft import/export.
//!
//! Mesh import (glTF 2.0, OBJ) into the document model, texture import/export, export
//! presets, channel naming conventions and OpenPBR material mapping. All formats are
//! parsed from their public specifications.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod collada;
pub mod export;
pub mod fbx;
pub mod mesh_formats;
pub mod mesh_import;
pub mod texture_maps;

pub use collada::load_collada;
pub use export::{ExportFormat, ExportPreset, ExportSettings, Packing, export_set, export_set_ctx, preset, presets};
pub use fbx::load_fbx;
pub use mesh_formats::{load_ply, load_stl};
pub use mesh_import::{load_gltf, load_obj, load_scene};
pub use texture_maps::{import_texture_set, match_channel};

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
    /// The STL parser rejected the file.
    #[error("invalid STL: {0}")]
    Stl(String),
    /// The PLY parser rejected the file.
    #[error("invalid PLY: {0}")]
    Ply(String),
    /// The FBX parser rejected the file.
    #[error("invalid FBX: {0}")]
    Fbx(String),
    /// The Collada parser rejected the file.
    #[error("invalid Collada: {0}")]
    Collada(String),
    /// The mesh failed a geometry invariant.
    #[error("invalid geometry: {0}")]
    Geometry(#[from] substancecraft_geom::GeometryError),
    /// The file extension is not a supported mesh format.
    #[error("unsupported mesh format: {0}")]
    UnsupportedFormat(String),
    /// The file contained no drawable geometry.
    #[error("the file contains no geometry")]
    Empty,
    /// An image codec failed.
    #[error("image codec error: {0}")]
    Codec(#[from] substancecraft_codecs::CodecError),
}

/// A small procedural demo model (a torus) so the viewport shows something with no assets.
pub fn demo_scene() -> Scene {
    match substancecraft_geom::primitives::torus(1.0, 0.35, 128, 64) {
        Ok(mesh) => Scene::from_mesh(mesh),
        Err(_) => Scene::default(),
    }
}
