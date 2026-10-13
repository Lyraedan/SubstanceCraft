//! SubstanceCraft's egui shell.
//!
//! The UI is thin and data-driven: it renders state and dispatches commands; it never owns
//! document semantics. All colours and radii come from [`theme::Tokens`]; nothing is
//! hard-coded in a view.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod app;
pub mod icons;
pub mod menu;
pub mod panels;
pub mod state;
pub mod theme;
pub mod tool_rail;
pub mod toolbar;
pub mod viewport;

pub use app::StudioApp;
pub use state::UiState;
pub use theme::Tokens;
