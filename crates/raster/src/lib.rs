//! SubstanceCraft raster foundation.
//!
//! Depth-agnostic pixel formats, copy-on-write tile buffers, per-channel PBR data and
//! the blend-mode maths shared by the CPU oracle and the GPU compositor. Never assume a
//! `u8`-only pixel path.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub use bytemuck;
