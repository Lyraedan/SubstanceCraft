//! Serializable UI state.
//!
//! View state lives here (never in the document) so the control channel can read and drive
//! it. It is intentionally small at M0; panels, dock layout and viewport framing grow here.

use serde::{Deserialize, Serialize};

/// Which theme to apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    /// The default dark studio theme.
    #[default]
    Dark,
}

/// Everything the shell remembers between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    /// The chosen theme.
    pub theme: ThemeChoice,
    /// Whether the left texture-set list is visible.
    pub show_texture_sets: bool,
    /// Whether the bottom layer stack is visible.
    pub show_layer_stack: bool,
    /// Whether the right shelf is visible.
    pub show_shelf: bool,
}

impl Default for UiState {
    fn default() -> Self {
        Self { theme: ThemeChoice::Dark, show_texture_sets: true, show_layer_stack: true, show_shelf: true }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_all_panels_visible() {
        let state = UiState::default();
        assert!(state.show_texture_sets && state.show_layer_stack && state.show_shelf);
    }

    #[test]
    fn round_trips_through_json() {
        let state = UiState { show_shelf: false, ..UiState::default() };
        let json = serde_json::to_string(&state).expect("serialize");
        let back: UiState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, back);
    }
}
