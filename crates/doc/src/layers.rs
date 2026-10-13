//! The non-destructive layer stack (pure data).
//!
//! Each texture set owns a [`LayerStack`]. Index `0` is the **bottom** layer; compositing walks
//! the stack bottom-to-top (the UI shows it top-first). A layer has a visibility flag, an opacity,
//! a blend mode, an optional mask and a kind (fill / paint / effect / group).

use serde::{Deserialize, Serialize};
use substancecraft_raster::RasterImage;

use crate::procedural::Effect;

/// How a layer combines with the accumulated result beneath it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlendMode {
    /// Replace the backdrop (weighted by coverage).
    #[default]
    Normal,
    /// `base · src`.
    Multiply,
    /// `1 - (1 - base)(1 - src)`.
    Screen,
    /// Per-channel contrast-preserving overlay.
    Overlay,
    /// `base + src`.
    Add,
    /// `base - src`.
    Subtract,
    /// `min(base, src)`.
    Darken,
    /// `max(base, src)`.
    Lighten,
}

impl BlendMode {
    /// Every mode, in UI order.
    pub const ALL: [BlendMode; 8] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Subtract,
        BlendMode::Darken,
        BlendMode::Lighten,
    ];

    /// A stable id for commands, files and menus.
    pub fn id(self) -> &'static str {
        match self {
            BlendMode::Normal => "normal",
            BlendMode::Multiply => "multiply",
            BlendMode::Screen => "screen",
            BlendMode::Overlay => "overlay",
            BlendMode::Add => "add",
            BlendMode::Subtract => "subtract",
            BlendMode::Darken => "darken",
            BlendMode::Lighten => "lighten",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            BlendMode::Normal => "Normal",
            BlendMode::Multiply => "Multiply",
            BlendMode::Screen => "Screen",
            BlendMode::Overlay => "Overlay",
            BlendMode::Add => "Add",
            BlendMode::Subtract => "Subtract",
            BlendMode::Darken => "Darken",
            BlendMode::Lighten => "Lighten",
        }
    }

    /// Parse a blend mode from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        BlendMode::ALL.into_iter().find(|mode| mode.id() == id)
    }
}

/// A layer mask. For now only a constant coverage; painted masks arrive with the brush engine.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mask {
    /// A constant coverage multiplier in `0..=1`.
    Constant(f32),
}

impl Mask {
    /// The mask's coverage at any point.
    pub fn coverage(self) -> f32 {
        match self {
            Mask::Constant(value) => value.clamp(0.0, 1.0),
        }
    }
}

/// The content of a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    /// A constant fill: `color` for RGB channels, `value` for scalar channels.
    Fill {
        /// The fill colour (RGBA) used by colour channels.
        color: [f32; 4],
        /// The scalar value used by scalar channels (metallic, roughness, height, AO).
        value: f32,
    },
    /// A paint layer backed by an image. `None` is an unpainted (transparent) layer.
    Paint {
        /// The painted RGBA content.
        image: Option<RasterImage>,
    },
    /// A group of child layers.
    Group {
        /// The children, composited in order.
        children: LayerStack,
    },
    /// An effect applied to the layers below (a generator fill or a filter).
    Effect {
        /// The generator or filter.
        effect: Effect,
    },
}

impl LayerKind {
    /// A short type id for UI and commands.
    pub fn type_id(&self) -> &'static str {
        match self {
            LayerKind::Fill { .. } => "fill",
            LayerKind::Paint { .. } => "paint",
            LayerKind::Group { .. } => "group",
            LayerKind::Effect { .. } => "effect",
        }
    }
}

/// One layer in a stack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layer {
    /// Display name.
    pub name: String,
    /// Whether the layer contributes.
    pub visible: bool,
    /// Opacity in `0..=1`.
    pub opacity: f32,
    /// Blend mode.
    pub blend: BlendMode,
    /// Optional mask.
    pub mask: Option<Mask>,
    /// The layer's content.
    pub kind: LayerKind,
}

impl Layer {
    /// A fill layer.
    pub fn fill(name: impl Into<String>, color: [f32; 4], value: f32) -> Self {
        Self { name: name.into(), visible: true, opacity: 1.0, blend: BlendMode::Normal, mask: None, kind: LayerKind::Fill { color, value } }
    }

    /// A group layer with no children yet.
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            mask: None,
            kind: LayerKind::Group { children: LayerStack::default() },
        }
    }

    /// A paint layer (empty until painted).
    pub fn paint(name: impl Into<String>) -> Self {
        Self { name: name.into(), visible: true, opacity: 1.0, blend: BlendMode::Normal, mask: None, kind: LayerKind::Paint { image: None } }
    }

    /// An effect layer (a generator fill or a filter).
    pub fn effect(name: impl Into<String>, effect: Effect) -> Self {
        Self { name: name.into(), visible: true, opacity: 1.0, blend: BlendMode::Normal, mask: None, kind: LayerKind::Effect { effect } }
    }

    /// The effect of an effect layer, if any.
    pub fn effect_kind(&self) -> Option<Effect> {
        match &self.kind {
            LayerKind::Effect { effect } => Some(*effect),
            _ => None,
        }
    }

    /// The effective coverage of this layer at a point: opacity × mask.
    pub fn coverage(&self) -> f32 {
        if !self.visible {
            return 0.0;
        }
        let base = self.opacity.clamp(0.0, 1.0);
        match self.mask {
            Some(mask) => base * mask.coverage(),
            None => base,
        }
    }
}

/// An ordered stack of layers. Index `0` is the bottom.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LayerStack {
    /// The layers, bottom-first.
    pub layers: Vec<Layer>,
}

impl LayerStack {
    /// An empty stack.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of layers.
    pub fn len(&self) -> usize {
        self.layers.len()
    }

    /// Whether the stack is empty.
    pub fn is_empty(&self) -> bool {
        self.layers.is_empty()
    }

    /// A layer by index.
    pub fn get(&self, index: usize) -> Option<&Layer> {
        self.layers.get(index)
    }

    /// A layer by index, mutably.
    pub fn get_mut(&mut self, index: usize) -> Option<&mut Layer> {
        self.layers.get_mut(index)
    }

    /// Append a layer on top.
    pub fn push(&mut self, layer: Layer) {
        self.layers.push(layer);
    }

    /// Total number of layers including group children.
    pub fn total(&self) -> usize {
        self.layers.iter().map(|layer| 1 + layer.child_stack().map_or(0, LayerStack::total)).sum()
    }

    /// Remove a layer by index; returns it if in range.
    pub fn remove(&mut self, index: usize) -> Option<Layer> {
        (index < self.layers.len()).then(|| self.layers.remove(index))
    }

    /// Move a layer one step toward the top; returns false if it cannot move.
    pub fn move_up(&mut self, index: usize) -> bool {
        if index + 1 >= self.layers.len() {
            return false;
        }
        self.layers.swap(index, index + 1);
        true
    }

    /// Move a layer one step toward the bottom; returns false if it cannot move.
    pub fn move_down(&mut self, index: usize) -> bool {
        if index == 0 {
            return false;
        }
        self.layers.swap(index, index - 1);
        true
    }

    /// Wrap the layer at `index` in a new group (returning the group's index).
    pub fn group_layer(&mut self, index: usize, name: impl Into<String>) -> Option<usize> {
        if index >= self.layers.len() {
            return None;
        }
        let layer = self.layers.remove(index);
        let mut group = Layer::group(name);
        if let LayerKind::Group { children } = &mut group.kind {
            children.push(layer);
        }
        self.layers.insert(index, group);
        Some(index)
    }
}

impl Layer {
    /// The child stack, if this layer is a group.
    pub fn child_stack(&self) -> Option<&LayerStack> {
        match &self.kind {
            LayerKind::Group { children } => Some(children),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coverage_includes_opacity_visibility_and_mask() {
        let mut layer = Layer::fill("f", [1.0; 4], 0.5);
        assert_eq!(layer.coverage(), 1.0);
        layer.mask = Some(Mask::Constant(0.5));
        assert_eq!(layer.coverage(), 0.5);
        layer.opacity = 0.5;
        assert_eq!(layer.coverage(), 0.25);
        layer.visible = false;
        assert_eq!(layer.coverage(), 0.0);
    }

    #[test]
    fn reorder_respects_bounds() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("a", [0.0; 4], 0.0));
        stack.push(Layer::fill("b", [1.0; 4], 1.0));
        assert!(!stack.move_up(1));
        assert!(stack.move_up(0));
        assert_eq!(stack.get(1).map(|l| l.name.as_str()), Some("a"));
        assert!(!stack.move_down(0));
    }

    #[test]
    fn grouping_wraps_a_layer() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("a", [0.0; 4], 0.0));
        assert_eq!(stack.group_layer(0, "grp"), Some(0));
        assert_eq!(stack.total(), 2);
        assert!(matches!(stack.get(0).map(|l| &l.kind), Some(LayerKind::Group { .. })));
    }

    #[test]
    fn blend_ids_round_trip() {
        for mode in BlendMode::ALL {
            assert_eq!(BlendMode::from_id(mode.id()), Some(mode));
        }
        assert_eq!(BlendMode::from_id("nope"), None);
    }
}
