//! SubstanceCraft brush engine.
//!
//! Turns pointer samples into stroke geometry: dabs, spacing, pressure/tilt response,
//! jitter, symmetry and the projection maths that maps screen strokes onto a mesh's
//! texture space.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
