//! Serializable UI state.
//!
//! View state lives here (never in the document) so the shell can persist and restore its
//! layout. It mirrors the Substance-3D-Painter-style workspace: a menu bar, a contextual tool
//! properties bar, a left tool rail, a right dock (texture set list plus a tabbed
//! layers/settings section), a bottom assets shelf and a status bar.

use serde::{Deserialize, Serialize};

/// Which theme to apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    /// The default dark studio theme.
    #[default]
    Dark,
}

/// Which section the right-middle dock shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RightTab {
    /// The layer stack.
    #[default]
    Layers,
    /// The active texture set's channels, resolution and shader.
    Settings,
}

impl RightTab {
    /// A stable id for state and commands.
    pub fn id(self) -> &'static str {
        match self {
            RightTab::Layers => "layers",
            RightTab::Settings => "settings",
        }
    }

    /// The tab label.
    pub fn label(self) -> &'static str {
        match self {
            RightTab::Layers => "Layers",
            RightTab::Settings => "Texture Set Settings",
        }
    }
}

/// Everything the shell remembers between runs.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UiState {
    /// The chosen theme.
    pub theme: ThemeChoice,
    /// Whether the right dock's texture set list is visible.
    pub show_texture_sets: bool,
    /// Whether the right dock's layers/settings section is visible.
    pub show_layer_stack: bool,
    /// Whether the bottom assets shelf is visible.
    pub show_shelf: bool,
    /// Whether the contextual tool properties bar is visible.
    pub show_tool_properties: bool,
    /// Whether the left icon tool rail is visible.
    pub show_tool_rail: bool,
    /// Which tab the right-middle dock shows.
    pub right_tab: RightTab,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::Dark,
            show_texture_sets: true,
            show_layer_stack: true,
            show_shelf: true,
            show_tool_properties: true,
            show_tool_rail: true,
            right_tab: RightTab::Layers,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_all_panels_visible() {
        let state = UiState::default();
        assert!(state.show_texture_sets && state.show_layer_stack && state.show_shelf);
        assert!(state.show_tool_properties && state.show_tool_rail);
        assert_eq!(state.right_tab, RightTab::Layers);
    }

    #[test]
    fn round_trips_through_json() {
        let state = UiState { show_shelf: false, right_tab: RightTab::Settings, ..UiState::default() };
        let json = serde_json::to_string(&state).expect("serialize");
        let back: UiState = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(state, back);
    }

    #[test]
    fn missing_fields_fall_back_to_defaults() {
        // A pre-existing config without the newer keys must still load.
        let back: UiState = serde_json::from_str("{\"show_shelf\":false}").expect("deserialize");
        assert!(!back.show_shelf);
        assert!(back.show_tool_properties && back.show_tool_rail);
        assert_eq!(back.right_tab, RightTab::Layers);
    }

    #[test]
    fn right_tab_ids_are_stable() {
        assert_eq!(RightTab::Layers.id(), "layers");
        assert_eq!(RightTab::Settings.id(), "settings");
    }
}
