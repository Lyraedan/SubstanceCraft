//! SubstanceCraft GPU compositor.
//!
//! Evaluates the layer stack on the GPU with wgpu: render-target-backed layers, masks,
//! blend modes, channel packing and mip generation. This crate owns raw wgpu work; it
//! does not depend on egui or winit.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
