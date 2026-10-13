//! SubstanceCraft pen tablet input.
//!
//! winit drops pen pressure and tilt, so this isolated crate reads them from the native
//! platform APIs. It is the only crate permitted to use `unsafe` (denied crate-wide,
//! allowed with a `SAFETY:` comment in the platform module); it deliberately does not
//! inherit the workspace `unsafe_code = "forbid"` lint.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
