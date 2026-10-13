//! SubstanceCraft native project format (`.sccraft`).
//!
//! Lossless, crash-safe serialization of the document model, written through
//! `craft-atomic`. The format deliberately fails to compile when `substancecraft-doc`
//! gains a field so nothing is silently dropped from a save.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]
