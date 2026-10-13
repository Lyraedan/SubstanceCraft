//! SubstanceCraft geometry foundation.
//!
//! Meshes, tangent-space generation, bounding volumes and spatial acceleration
//! structures shared by baking, painting and rendering. Nothing here depends on a
//! GPU or a UI.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod bounds;
pub mod mesh;
pub mod primitives;
pub mod tangents;

pub use bounds::Bounds;
pub use glam;
pub use mesh::{Mesh, Submesh};

use thiserror::Error;

/// Errors from geometry operations. Every one is recoverable and actionable; nothing panics.
#[derive(Debug, Error, PartialEq, Eq)]
pub enum GeometryError {
    /// A mesh with no vertices cannot be rendered or baked.
    #[error("mesh has no vertices")]
    Empty,
    /// Per-vertex attribute arrays must all be as long as the position array.
    #[error("mesh has {positions} positions but {attribute} has {count}")]
    AttributeMismatch {
        /// Number of positions.
        positions: usize,
        /// The offending attribute name.
        attribute: &'static str,
        /// The offending attribute length.
        count: usize,
    },
    /// An index points past the end of the vertex arrays.
    #[error("index {index} is out of range for {vertices} vertices")]
    IndexOutOfRange {
        /// The offending index.
        index: u32,
        /// The vertex count.
        vertices: usize,
    },
    /// The index buffer length is not a multiple of three.
    #[error("index count {0} is not a multiple of 3")]
    BadIndexCount(usize),
    /// A generation parameter was rejected (e.g. zero segments).
    #[error("invalid geometry parameter: {0}")]
    Parameter(&'static str),
}
