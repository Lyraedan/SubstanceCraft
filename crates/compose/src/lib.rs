//! SubstanceCraft CPU compositor.
//!
//! Evaluates a texture set's layer stack on the CPU. It is intentionally simple and
//! obviously correct: the GPU compositor's output is validated against it, and it backs
//! headless export and baking of flattened results.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
