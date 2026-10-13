//! SubstanceCraft viewport renderer.
//!
//! Real-time PBR shading of the mesh from the scene's channels: image-based-style lighting from
//! a hemisphere ambient plus a key light, metal/rough workflow, display modes and tone mapping.
//! Shading follows the public PBR/OpenPBR literature. This crate owns raw wgpu work and never
//! depends on egui or winit.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod camera;
pub mod headless;
pub mod renderer;
pub mod settings;

pub use camera::OrbitCamera;
pub use headless::{Headless, render_rgba, render_rgba_with};
pub use renderer::{RenderError, ViewportRenderer};
pub use settings::{DisplayMode, RenderSettings};

pub use substancecraft_color::ToneMap;
