//! SubstanceCraft mesh-map bakers.
//!
//! Ray-based baking of model information onto a low-poly UV layout: tangent/world-space
//! normal, ambient occlusion, curvature, position, thickness and material ID, plus cage
//! generation for high-to-low projection.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
