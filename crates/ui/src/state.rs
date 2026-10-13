//! Serializable UI state.
//!
//! View state lives here (never in the document) so the shell can persist and restore its
//! layout. The runtime source of truth for panel visibility and the dock selection is the
//! session's `Workspace` (so the control channel can drive it); this type is the on-disk form.

use serde::{Deserialize, Serialize};
use substancecraft_engine::workspace::Workspace;

pub use substancecraft_engine::workspace::RightTab;

/// Which theme to apply.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ThemeChoice {
    /// The default dark studio theme.
    #[default]
    Dark,
    /// The neutral studio-gray theme.
    Gray,
}

impl ThemeChoice {
    /// Every theme, in menu order.
    pub const ALL: [ThemeChoice; 2] = [ThemeChoice::Dark, ThemeChoice::Gray];

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::Dark => "Dark",
            ThemeChoice::Gray => "Gray",
        }
    }

    /// The palette for this theme.
    pub fn tokens(self) -> crate::theme::Tokens {
        match self {
            ThemeChoice::Dark => crate::theme::Tokens::DARK,
            ThemeChoice::Gray => crate::theme::Tokens::GRAY,
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

impl UiState {
    /// The persisted layout as session workspace state.
    pub fn to_workspace(&self) -> Workspace {
        Workspace {
            texture_sets: self.show_texture_sets,
            layers: self.show_layer_stack,
            assets: self.show_shelf,
            tool_properties: self.show_tool_properties,
            tool_rail: self.show_tool_rail,
            right_tab: self.right_tab,
        }
    }

    /// Update the persisted layout from session workspace state.
    pub fn apply_workspace(&mut self, workspace: &Workspace) {
        self.show_texture_sets = workspace.texture_sets;
        self.show_layer_stack = workspace.layers;
        self.show_shelf = workspace.assets;
        self.show_tool_properties = workspace.tool_properties;
        self.show_tool_rail = workspace.tool_rail;
        self.right_tab = workspace.right_tab;
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
    fn workspace_round_trips_through_ui_state() {
        let mut state = UiState::default();
        let mut workspace = state.to_workspace();
        workspace.set_visible(substancecraft_engine::workspace::Panel::Assets, false);
        workspace.right_tab = RightTab::Settings;
        state.apply_workspace(&workspace);
        assert_eq!(state.to_workspace(), workspace);
    }

    #[test]
    fn right_tab_ids_are_stable() {
        assert_eq!(RightTab::Layers.id(), "layers");
        assert_eq!(RightTab::Settings.id(), "settings");
    }
}
