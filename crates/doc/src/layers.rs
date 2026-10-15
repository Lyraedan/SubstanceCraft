//! The non-destructive layer stack (pure data).
//!
//! Each texture set owns a [`LayerStack`]. Index `0` is the **bottom** layer; compositing walks
//! the stack bottom-to-top (the UI shows it top-first). A layer has a visibility flag, an opacity,
//! a blend mode, an optional mask and a kind (fill / paint / effect / group).

use serde::{Deserialize, Serialize};
use substancecraft_raster::RasterImage;

use crate::procedural::{Effect, Filter, Generator};
use crate::project::Channel;

/// The default fill strength.
fn default_strength() -> f32 {
    1.0
}

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
    /// Brighten the backdrop by the source (`base / (1 - src)`).
    ColorDodge,
    /// Darken the backdrop by the source (`1 - (1 - base) / src`).
    ColorBurn,
    /// `base + src - 1`.
    LinearBurn,
    /// Overlay driven by the source (hard light).
    HardLight,
    /// A gentler hard light (W3C soft light).
    SoftLight,
    /// A dodge/burn split at the source midpoint.
    VividLight,
    /// `base + 2·src - 1`.
    LinearLight,
    /// Replace with dark or light depending on the source.
    PinLight,
    /// Posterised combination (0 or 1).
    HardMix,
    /// `|base - src|`.
    Difference,
    /// `base + src - 2·base·src`.
    Exclusion,
    /// `base / src`.
    Divide,
    /// The source's hue with the backdrop's saturation and luminosity.
    Hue,
    /// The source's saturation with the backdrop's hue and luminosity.
    Saturation,
    /// The source's hue and saturation with the backdrop's luminosity.
    Color,
    /// The source's luminosity with the backdrop's hue and saturation.
    Luminosity,
    /// The darker of the two colours, by luminosity.
    DarkerColor,
    /// The lighter of the two colours, by luminosity.
    LighterColor,
}

impl BlendMode {
    /// Every mode, in UI order.
    pub const ALL: [BlendMode; 26] = [
        BlendMode::Normal,
        BlendMode::Multiply,
        BlendMode::Screen,
        BlendMode::Overlay,
        BlendMode::Add,
        BlendMode::Subtract,
        BlendMode::Darken,
        BlendMode::Lighten,
        BlendMode::ColorDodge,
        BlendMode::ColorBurn,
        BlendMode::LinearBurn,
        BlendMode::HardLight,
        BlendMode::SoftLight,
        BlendMode::VividLight,
        BlendMode::LinearLight,
        BlendMode::PinLight,
        BlendMode::HardMix,
        BlendMode::Difference,
        BlendMode::Exclusion,
        BlendMode::Divide,
        BlendMode::Hue,
        BlendMode::Saturation,
        BlendMode::Color,
        BlendMode::Luminosity,
        BlendMode::DarkerColor,
        BlendMode::LighterColor,
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
            BlendMode::ColorDodge => "color_dodge",
            BlendMode::ColorBurn => "color_burn",
            BlendMode::LinearBurn => "linear_burn",
            BlendMode::HardLight => "hard_light",
            BlendMode::SoftLight => "soft_light",
            BlendMode::VividLight => "vivid_light",
            BlendMode::LinearLight => "linear_light",
            BlendMode::PinLight => "pin_light",
            BlendMode::HardMix => "hard_mix",
            BlendMode::Difference => "difference",
            BlendMode::Exclusion => "exclusion",
            BlendMode::Divide => "divide",
            BlendMode::Hue => "hue",
            BlendMode::Saturation => "saturation",
            BlendMode::Color => "color",
            BlendMode::Luminosity => "luminosity",
            BlendMode::DarkerColor => "darker_color",
            BlendMode::LighterColor => "lighter_color",
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
            BlendMode::ColorDodge => "Color Dodge",
            BlendMode::ColorBurn => "Color Burn",
            BlendMode::LinearBurn => "Linear Burn",
            BlendMode::HardLight => "Hard Light",
            BlendMode::SoftLight => "Soft Light",
            BlendMode::VividLight => "Vivid Light",
            BlendMode::LinearLight => "Linear Light",
            BlendMode::PinLight => "Pin Light",
            BlendMode::HardMix => "Hard Mix",
            BlendMode::Difference => "Difference",
            BlendMode::Exclusion => "Exclusion",
            BlendMode::Divide => "Divide",
            BlendMode::Hue => "Hue",
            BlendMode::Saturation => "Saturation",
            BlendMode::Color => "Color",
            BlendMode::Luminosity => "Luminosity",
            BlendMode::DarkerColor => "Darker Color",
            BlendMode::LighterColor => "Lighter Color",
        }
    }

    /// Parse a blend mode from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        BlendMode::ALL.into_iter().find(|mode| mode.id() == id)
    }
}

/// One effect in a mask's per-texel stack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum MaskEffect {
    /// A constant mask fill.
    Fill {
        /// Coverage in `0..=1`.
        value: f32,
    },
    /// A procedural generator fill.
    Generator(Generator),
    /// A filter over the mask above it.
    Filter(Filter),
    /// A projected image mask.
    Image {
        /// The image.
        image: RasterImage,
        /// How the image maps onto the surface.
        projection: Projection,
        /// Invert the sampled value.
        invert: bool,
    },
    /// A baked mesh-map mask (cavity, curvature, thickness, …).
    MeshMap {
        /// Which map.
        kind: MeshMapKind,
        /// Invert the sampled value.
        invert: bool,
    },
}

impl MaskEffect {
    /// A stable id for commands, files and menus.
    pub fn id(&self) -> &'static str {
        match self {
            MaskEffect::Fill { .. } => "fill",
            MaskEffect::Generator(generator) => generator.id(),
            MaskEffect::Filter(filter) => filter.id(),
            MaskEffect::Image { .. } => "image",
            MaskEffect::MeshMap { .. } => "mesh_map",
        }
    }

    /// A human-readable label.
    pub fn label(&self) -> String {
        match self {
            MaskEffect::Fill { .. } => "Fill".to_string(),
            MaskEffect::Generator(generator) => generator.label().to_string(),
            MaskEffect::Filter(filter) => filter.label().to_string(),
            MaskEffect::Image { .. } => "Image".to_string(),
            MaskEffect::MeshMap { kind, .. } => kind.label().to_string(),
        }
    }
}

/// A layer mask: either a constant coverage or a per-texel effect stack.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Mask {
    /// A constant coverage multiplier in `0..=1`.
    Constant(f32),
    /// A per-texel stack of mask effects (generators, filters, image fills, painted content).
    Stack {
        /// The effects, applied in order over a black base.
        effects: Vec<MaskEffect>,
    },
}

impl Mask {
    /// The scalar coverage (1.0 for a per-texel stack; the stack is applied per texel).
    pub fn coverage(&self) -> f32 {
        match self {
            Mask::Constant(value) => value.clamp(0.0, 1.0),
            Mask::Stack { .. } => 1.0,
        }
    }

    /// Whether the mask varies per texel.
    pub fn is_dynamic(&self) -> bool {
        matches!(self, Mask::Stack { .. })
    }
}

/// A baked mesh map a fill can sample.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum MeshMapKind {
    /// Object-space position.
    Position,
    /// World/object-space normal.
    WorldNormal,
    /// Ambient occlusion.
    AmbientOcclusion,
    /// Thickness.
    Thickness,
    /// Curvature.
    Curvature,
    /// Per-material colour ID.
    MaterialId,
}

impl MeshMapKind {
    /// Every settle map, in UI order.
    pub const ALL: [MeshMapKind; 6] = [
        MeshMapKind::Position,
        MeshMapKind::WorldNormal,
        MeshMapKind::AmbientOcclusion,
        MeshMapKind::Thickness,
        MeshMapKind::Curvature,
        MeshMapKind::MaterialId,
    ];

    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            MeshMapKind::Position => "position",
            MeshMapKind::WorldNormal => "world_normal",
            MeshMapKind::AmbientOcclusion => "ambient_occlusion",
            MeshMapKind::Thickness => "thickness",
            MeshMapKind::Curvature => "curvature",
            MeshMapKind::MaterialId => "material_id",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            MeshMapKind::Position => "Position",
            MeshMapKind::WorldNormal => "World Normal",
            MeshMapKind::AmbientOcclusion => "Ambient Occlusion",
            MeshMapKind::Thickness => "Thickness",
            MeshMapKind::Curvature => "Curvature",
            MeshMapKind::MaterialId => "Material ID",
        }
    }

    /// Parse a mesh-map id.
    pub fn from_id(id: &str) -> Option<Self> {
        MeshMapKind::ALL.into_iter().find(|kind| kind.id() == id)
    }
}

/// Where a fill layer's pixels come from.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub enum FillSource {
    /// A constant colour / scalar value.
    #[default]
    Uniform,
    /// An imported image.
    Image {
        /// The decoded image (RGBA, linear for colour channels).
        image: RasterImage,
    },
    /// A procedural generator.
    Procedural {
        /// The generator.
        generator: Generator,
    },
    /// A baked mesh map.
    MeshMap {
        /// Which map.
        kind: MeshMapKind,
    },
}

impl FillSource {
    /// A short type id for UI and commands.
    pub fn id(&self) -> &'static str {
        match self {
            FillSource::Uniform => "uniform",
            FillSource::Image { .. } => "image",
            FillSource::Procedural { .. } => "procedural",
            FillSource::MeshMap { .. } => "mesh_map",
        }
    }
}

/// How a fill's source is mapped onto the surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProjectionKind {
    /// Sample by the mesh UVs.
    #[default]
    Uv,
    /// Project along an axis in object space.
    Planar,
    /// Blend three axis projections by the surface normal.
    TriPlanar,
}

impl ProjectionKind {
    /// Every projection, in UI order.
    pub const ALL: [ProjectionKind; 3] = [ProjectionKind::Uv, ProjectionKind::Planar, ProjectionKind::TriPlanar];

    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            ProjectionKind::Uv => "uv",
            ProjectionKind::Planar => "planar",
            ProjectionKind::TriPlanar => "triplanar",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            ProjectionKind::Uv => "UV",
            ProjectionKind::Planar => "Planar",
            ProjectionKind::TriPlanar => "Tri-planar",
        }
    }

    /// Parse a projection id.
    pub fn from_id(id: &str) -> Option<Self> {
        ProjectionKind::ALL.into_iter().find(|kind| kind.id() == id)
    }
}

/// A fill's projection: how tiling, rotation and offset map a source onto the surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Projection {
    /// The projection kind.
    pub kind: ProjectionKind,
    /// Tiling scale (X, Y).
    pub scale: [f32; 2],
    /// Rotation in degrees about the projection centre.
    pub rotation: f32,
    /// Offset (X, Y).
    pub offset: [f32; 2],
    /// Planar/tri-planar axis (0 = X, 1 = Y, 2 = Z).
    pub axis: u8,
}

impl Default for Projection {
    fn default() -> Self {
        Self { kind: ProjectionKind::Uv, scale: [1.0, 1.0], rotation: 0.0, offset: [0.0, 0.0], axis: 1 }
    }
}

impl Projection {
    /// Apply tiling, rotation and offset to a UV coordinate.
    pub fn transform_uv(&self, uv: [f32; 2]) -> [f32; 2] {
        let scaled = [uv[0] * self.scale[0], uv[1] * self.scale[1]];
        let (sin, cos) = self.rotation.to_radians().sin_cos();
        let centred = [scaled[0] - 0.5, scaled[1] - 0.5];
        let rotated = [centred[0] * cos - centred[1] * sin, centred[0] * sin + centred[1] * cos];
        [rotated[0] + 0.5 + self.offset[0], rotated[1] + 0.5 + self.offset[1]]
    }

    /// Project an object-space point and normal to a source UV (`uv` is the mesh UV).
    pub fn project(&self, uv: [f32; 2], position: [f32; 3], normal: [f32; 3]) -> [f32; 2] {
        match self.kind {
            ProjectionKind::Uv => self.transform_uv(uv),
            ProjectionKind::Planar => self.transform_uv(planar_uv(self.axis, position)),
            ProjectionKind::TriPlanar => self.transform_uv(planar_uv(dominant_axis(normal), position)),
        }
    }
}

/// The planar UV of an object-space point along an axis (X → YZ, Y → XZ, Z → XY).
pub fn planar_uv(axis: u8, position: [f32; 3]) -> [f32; 2] {
    match axis {
        0 => [position[1], position[2]],
        2 => [position[0], position[1]],
        _ => [position[0], position[2]],
    }
}

/// The dominant axis (0 = X, 1 = Y, 2 = Z) of a normal.
pub fn dominant_axis(normal: [f32; 3]) -> u8 {
    let x = normal[0].abs();
    let y = normal[1].abs();
    let z = normal[2].abs();
    if x >= y && x >= z {
        0
    } else if y >= z {
        1
    } else {
        2
    }
}

/// The content of a layer.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum LayerKind {
    /// A fill layer: `color`/`value` for a uniform fill, or a projected `source`.
    Fill {
        /// The fill colour (RGBA) used by colour channels.
        color: [f32; 4],
        /// The scalar value used by scalar channels (metallic, roughness, height, AO).
        value: f32,
        /// Where the pixels come from.
        #[serde(default)]
        source: FillSource,
        /// How the source maps onto the surface.
        #[serde(default)]
        projection: Projection,
        /// The per-channel strength multiplier.
        #[serde(default = "default_strength")]
        strength: f32,
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
    /// Per-channel opacity multipliers (1.0 when absent), applied on top of `opacity`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub channel_opacity: Vec<(Channel, f32)>,
    /// The connected parts (mesh pieces) this layer is restricted to (`None` = every part).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parts: Option<Vec<u32>>,
    /// The layer's content.
    pub kind: LayerKind,
}

impl Layer {
    /// A fill layer.
    pub fn fill(name: impl Into<String>, color: [f32; 4], value: f32) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            mask: None,
            channel_opacity: Vec::new(),
            parts: None,
            kind: LayerKind::Fill { color, value, source: FillSource::default(), projection: Projection::default(), strength: 1.0 },
        }
    }

    /// A fill layer with an explicit source and projection.
    pub fn fill_with_source(name: impl Into<String>, color: [f32; 4], value: f32, source: FillSource, projection: Projection) -> Self {
        let mut layer = Self::fill(name, color, value);
        layer.kind = LayerKind::Fill { color, value, source, projection, strength: 1.0 };
        layer
    }

    /// A group layer with no children yet.
    pub fn group(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            mask: None,
            channel_opacity: Vec::new(),
            parts: None,
            kind: LayerKind::Group { children: LayerStack::default() },
        }
    }

    /// A paint layer (empty until painted).
    pub fn paint(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            mask: None,
            channel_opacity: Vec::new(),
            parts: None,
            kind: LayerKind::Paint { image: None },
        }
    }

    /// An effect layer (a generator fill or a filter).
    pub fn effect(name: impl Into<String>, effect: Effect) -> Self {
        Self {
            name: name.into(),
            visible: true,
            opacity: 1.0,
            blend: BlendMode::Normal,
            mask: None,
            channel_opacity: Vec::new(),
            parts: None,
            kind: LayerKind::Effect { effect },
        }
    }

    /// The effect of an effect layer, if any.
    pub fn effect_kind(&self) -> Option<Effect> {
        match &self.kind {
            LayerKind::Effect { effect } => Some(*effect),
            _ => None,
        }
    }

    /// The fill source, if this is a fill layer.
    pub fn fill_source(&self) -> Option<&FillSource> {
        match &self.kind {
            LayerKind::Fill { source, .. } => Some(source),
            _ => None,
        }
    }

    /// The fill strength, if this is a fill layer.
    pub fn fill_strength(&self) -> f32 {
        match &self.kind {
            LayerKind::Fill { strength, .. } => *strength,
            _ => 1.0,
        }
    }

    /// The effective coverage of this layer at a point: opacity × mask.
    pub fn coverage(&self) -> f32 {
        if !self.visible {
            return 0.0;
        }
        let base = self.opacity.clamp(0.0, 1.0);
        match &self.mask {
            Some(mask) => base * mask.coverage(),
            None => base,
        }
    }

    /// The per-channel opacity multiplier (1.0 when unset).
    pub fn channel_opacity(&self, channel: Channel) -> f32 {
        self.channel_opacity.iter().find(|(stored, _)| *stored == channel).map(|(_, value)| value.clamp(0.0, 1.0)).unwrap_or(1.0)
    }

    /// The effective coverage for one channel: opacity × mask × per-channel opacity.
    pub fn coverage_for(&self, channel: Channel) -> f32 {
        self.coverage() * self.channel_opacity(channel)
    }

    /// Set a channel's opacity multiplier (clamped), inserting or updating it.
    pub fn set_channel_opacity(&mut self, channel: Channel, value: f32) {
        let value = value.clamp(0.0, 1.0);
        if let Some(slot) = self.channel_opacity.iter_mut().find(|(stored, _)| *stored == channel) {
            slot.1 = value;
        } else {
            self.channel_opacity.push((channel, value));
        }
    }

    /// Remove a channel's opacity override, returning it to 1.0.
    pub fn clear_channel_opacity(&mut self, channel: Channel) {
        self.channel_opacity.retain(|(stored, _)| *stored != channel);
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
    fn per_channel_opacity_scales_coverage_for_that_channel() {
        let mut layer = Layer::fill("f", [1.0; 4], 0.5);
        assert_eq!(layer.channel_opacity(Channel::Roughness), 1.0);
        layer.set_channel_opacity(Channel::Roughness, 0.25);
        assert!((layer.coverage_for(Channel::Roughness) - 0.25).abs() < 1e-6);
        assert!((layer.coverage_for(Channel::BaseColor) - 1.0).abs() < 1e-6);
        layer.clear_channel_opacity(Channel::Roughness);
        assert_eq!(layer.channel_opacity(Channel::Roughness), 1.0);
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
