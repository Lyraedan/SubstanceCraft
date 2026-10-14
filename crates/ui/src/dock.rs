//! The dockable workspace: a Painter-style arrangement of tabbed, draggable panels.
//!
//! Panels are identified by [`DockPanel`] and arranged by a `craft_ui` docking [`Layout`]. The
//! viewport is a protected panel (it renders without a tab and cannot be moved); every other panel
//! can be dragged, tabbed together, split or floated. The layout is persisted in the UI state and
//! mirrored to the session's workspace commands so the control channel can show/hide panels.

use craft_ui::docking::{Layout, Node, Permissions};
use craft_ui::layout::{SplitAxis, SplitSize};
use serde::{Deserialize, Serialize};

/// A dockable workspace panel.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DockPanel {
    /// The 3D viewport (protected: fixed, tab-less).
    Viewport,
    /// The texture set tree.
    TextureSetList,
    /// The layer stack.
    Layers,
    /// The active texture set's channels/resolution/shader.
    Settings,
    /// Properties — paint (material/mask previews, brush).
    Properties,
    /// The assets shelf.
    Assets,
    /// Bake settings.
    Bake,
    /// Post-effects settings.
    Post,
}

impl DockPanel {
    /// Every panel.
    pub const ALL: [DockPanel; 8] = [
        DockPanel::Viewport,
        DockPanel::TextureSetList,
        DockPanel::Layers,
        DockPanel::Settings,
        DockPanel::Properties,
        DockPanel::Assets,
        DockPanel::Bake,
        DockPanel::Post,
    ];

    /// A stable id (for state and menus).
    pub fn id(self) -> &'static str {
        match self {
            DockPanel::Viewport => "viewport",
            DockPanel::TextureSetList => "texture_set_list",
            DockPanel::Layers => "layers",
            DockPanel::Settings => "settings",
            DockPanel::Properties => "properties",
            DockPanel::Assets => "assets",
            DockPanel::Bake => "bake",
            DockPanel::Post => "post",
        }
    }

    /// The tab label.
    pub fn label(self) -> &'static str {
        match self {
            DockPanel::Viewport => "Viewport",
            DockPanel::TextureSetList => "Texture Set List",
            DockPanel::Layers => "Layers",
            DockPanel::Settings => "Texture Set Settings",
            DockPanel::Properties => "Properties",
            DockPanel::Assets => "Assets",
            DockPanel::Bake => "Bake",
            DockPanel::Post => "Post",
        }
    }

    /// Docking permissions: the viewport is protected, the rest are fully dockable.
    pub fn permissions(self) -> Permissions {
        match self {
            DockPanel::Viewport => Permissions::PROTECTED,
            _ => Permissions::default(),
        }
    }
}

/// The default arrangement, mirroring Substance 3D Painter: viewport center-left, the texture set
/// list over the Layers/Settings tabs over Properties on the right, and the assets shelf along the
/// bottom.
pub fn default_layout() -> Layout<DockPanel> {
    let right = Node::Split {
        axis: SplitAxis::Vertical,
        size: SplitSize::FixedFirst(170.0),
        first: Box::new(Node::Tabs { panels: vec![DockPanel::TextureSetList], active: 0 }),
        second: Box::new(Node::Split {
            axis: SplitAxis::Vertical,
            size: SplitSize::FixedFirst(220.0),
            first: Box::new(Node::Tabs { panels: vec![DockPanel::Layers, DockPanel::Settings], active: 0 }),
            second: Box::new(Node::Tabs { panels: vec![DockPanel::Properties], active: 0 }),
        }),
    };
    let main = Node::Split {
        axis: SplitAxis::Horizontal,
        size: SplitSize::FixedSecond(330.0),
        first: Box::new(Node::Tabs { panels: vec![DockPanel::Viewport], active: 0 }),
        second: Box::new(right),
    };
    let root = Node::Split {
        axis: SplitAxis::Vertical,
        size: SplitSize::FixedSecond(180.0),
        first: Box::new(main),
        second: Box::new(Node::Tabs { panels: vec![DockPanel::Assets, DockPanel::Bake, DockPanel::Post], active: 0 }),
    };
    Layout { root: Some(root), floating: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_layout_is_valid_and_holds_every_panel_once() {
        let layout = default_layout();
        assert!(layout.validate().is_ok());
        for panel in DockPanel::ALL {
            assert!(layout.contains(&panel), "{} missing", panel.id());
        }
    }

    #[test]
    fn panel_ids_are_unique() {
        let mut ids: Vec<&str> = DockPanel::ALL.iter().map(|panel| panel.id()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), DockPanel::ALL.len());
    }
}
