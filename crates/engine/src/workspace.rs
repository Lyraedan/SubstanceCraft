//! Workspace layout view state: which panels are visible and which tab the right dock shows.
//!
//! This is view state rather than document state, but it lives on the [`crate::Session`] so the
//! control channel, CLI and MCP can drive the workspace with commands — the same way the camera
//! and render settings already live on the session.

use serde::{Deserialize, Serialize};

/// A dockable region of the workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Panel {
    /// The right dock's texture set list.
    TextureSets,
    /// The right dock's layers/settings section.
    Layers,
    /// The bottom assets shelf.
    Assets,
    /// The top tool properties bar.
    ToolProperties,
    /// The left icon tool rail.
    ToolRail,
}

impl Panel {
    /// Every panel, in workspace order.
    pub const ALL: [Panel; 5] = [Panel::TextureSets, Panel::Layers, Panel::Assets, Panel::ToolProperties, Panel::ToolRail];

    /// A stable id for commands and files.
    pub fn id(self) -> &'static str {
        match self {
            Panel::TextureSets => "texture_sets",
            Panel::Layers => "layers",
            Panel::Assets => "assets",
            Panel::ToolProperties => "tool_properties",
            Panel::ToolRail => "tool_rail",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Panel::TextureSets => "Texture Set List",
            Panel::Layers => "Layers & Settings",
            Panel::Assets => "Assets Shelf",
            Panel::ToolProperties => "Tool Properties",
            Panel::ToolRail => "Tool Rail",
        }
    }

    /// Parse a panel from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        Panel::ALL.into_iter().find(|panel| panel.id() == id)
    }
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
    /// Every tab, in order.
    pub const ALL: [RightTab; 2] = [RightTab::Layers, RightTab::Settings];

    /// A stable id for commands and files.
    pub fn id(self) -> &'static str {
        match self {
            RightTab::Layers => "layers",
            RightTab::Settings => "settings",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            RightTab::Layers => "Layers",
            RightTab::Settings => "Texture Set Settings",
        }
    }

    /// Parse a tab from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        RightTab::ALL.into_iter().find(|tab| tab.id() == id)
    }
}

/// Panel visibility and the dock selection.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Workspace {
    /// Whether the texture set list is visible.
    pub texture_sets: bool,
    /// Whether the layers/settings section is visible.
    pub layers: bool,
    /// Whether the assets shelf is visible.
    pub assets: bool,
    /// Whether the tool properties bar is visible.
    pub tool_properties: bool,
    /// Whether the tool rail is visible.
    pub tool_rail: bool,
    /// Which tab the right-middle dock shows.
    pub right_tab: RightTab,
}

impl Default for Workspace {
    fn default() -> Self {
        Self { texture_sets: true, layers: true, assets: true, tool_properties: true, tool_rail: true, right_tab: RightTab::Layers }
    }
}

impl Workspace {
    /// Whether a panel is currently visible.
    pub fn is_visible(&self, panel: Panel) -> bool {
        match panel {
            Panel::TextureSets => self.texture_sets,
            Panel::Layers => self.layers,
            Panel::Assets => self.assets,
            Panel::ToolProperties => self.tool_properties,
            Panel::ToolRail => self.tool_rail,
        }
    }

    /// Show or hide a panel.
    pub fn set_visible(&mut self, panel: Panel, visible: bool) {
        match panel {
            Panel::TextureSets => self.texture_sets = visible,
            Panel::Layers => self.layers = visible,
            Panel::Assets => self.assets = visible,
            Panel::ToolProperties => self.tool_properties = visible,
            Panel::ToolRail => self.tool_rail = visible,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_ids_round_trip() {
        for panel in Panel::ALL {
            assert_eq!(Panel::from_id(panel.id()), Some(panel));
        }
        assert_eq!(Panel::from_id("nope"), None);
    }

    #[test]
    fn right_tab_ids_round_trip() {
        for tab in RightTab::ALL {
            assert_eq!(RightTab::from_id(tab.id()), Some(tab));
        }
        assert_eq!(RightTab::from_id("nope"), None);
    }

    #[test]
    fn set_visible_toggles_each_panel() {
        let mut workspace = Workspace::default();
        for panel in Panel::ALL {
            workspace.set_visible(panel, false);
            assert!(!workspace.is_visible(panel));
            workspace.set_visible(panel, true);
            assert!(workspace.is_visible(panel));
        }
    }
}
