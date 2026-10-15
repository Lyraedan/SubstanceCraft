//! Built-in material presets, smart materials and smart masks.
//!
//! These are original, procedural content (measured metal reflectances plus noise- and mesh-map-
//! driven wear), never copied from a proprietary library. A smart material is a small layer
//! subtree; a smart mask is a mask-effect stack.

use substancecraft_doc::{Filter, Generator, Layer, Mask, MaskEffect, MeshMapKind};

/// A flat fill-layer material preset.
#[derive(Clone, Copy, Debug)]
pub struct MaterialPreset {
    /// Display name.
    pub name: &'static str,
    /// Base colour (linear RGBA).
    pub color: [f32; 4],
    /// Metallic value.
    pub metallic: f32,
    /// Roughness value.
    pub roughness: f32,
}

/// A smart material: a named layer subtree.
#[derive(Clone, Debug)]
pub struct SmartMaterial {
    /// Display name.
    pub name: &'static str,
    /// The layers to insert (wrapped in a group on apply).
    pub layers: Vec<Layer>,
}

/// A smart mask: a named mask-effect stack.
#[derive(Clone, Debug)]
pub struct SmartMask {
    /// Display name.
    pub name: &'static str,
    /// The effects, applied over a black base.
    pub effects: Vec<MaskEffect>,
}

/// The built-in material presets (measured metal reflectances and common dielectrics).
pub fn material_presets() -> Vec<MaterialPreset> {
    vec![
        MaterialPreset { name: "Gold", color: [1.0, 0.766, 0.336, 1.0], metallic: 1.0, roughness: 0.2 },
        MaterialPreset { name: "Copper", color: [0.955, 0.637, 0.538, 1.0], metallic: 1.0, roughness: 0.3 },
        MaterialPreset { name: "Aluminum", color: [0.913, 0.921, 0.925, 1.0], metallic: 1.0, roughness: 0.25 },
        MaterialPreset { name: "Iron", color: [0.56, 0.57, 0.58, 1.0], metallic: 1.0, roughness: 0.4 },
        MaterialPreset { name: "Plastic Red", color: [0.8, 0.1, 0.1, 1.0], metallic: 0.0, roughness: 0.4 },
        MaterialPreset { name: "Plastic Blue", color: [0.1, 0.2, 0.8, 1.0], metallic: 0.0, roughness: 0.4 },
        MaterialPreset { name: "Rubber", color: [0.05, 0.05, 0.05, 1.0], metallic: 0.0, roughness: 0.8 },
        MaterialPreset { name: "Ceramic", color: [0.9, 0.9, 0.88, 1.0], metallic: 0.0, roughness: 0.15 },
        MaterialPreset { name: "Wood", color: [0.45, 0.28, 0.14, 1.0], metallic: 0.0, roughness: 0.6 },
        MaterialPreset { name: "Concrete", color: [0.6, 0.6, 0.58, 1.0], metallic: 0.0, roughness: 0.8 },
    ]
}

/// Find a material preset by name.
pub fn material_preset(name: &str) -> Option<MaterialPreset> {
    material_presets().into_iter().find(|preset| preset.name == name)
}

/// The built-in smart materials.
pub fn smart_materials() -> Vec<SmartMaterial> {
    vec![
        SmartMaterial { name: "Worn Metal", layers: worn_metal() },
        SmartMaterial { name: "Rusty Iron", layers: rusty_iron() },
        SmartMaterial { name: "Old Wood", layers: old_wood() },
    ]
}

/// Find a smart material by name.
pub fn smart_material(name: &str) -> Option<SmartMaterial> {
    smart_materials().into_iter().find(|material| material.name == name)
}

/// The built-in smart masks.
pub fn smart_masks() -> Vec<SmartMask> {
    vec![
        SmartMask { name: "Edges", effects: vec![MaskEffect::MeshMap { kind: MeshMapKind::Curvature, invert: false }] },
        SmartMask { name: "Cavity", effects: vec![MaskEffect::MeshMap { kind: MeshMapKind::AmbientOcclusion, invert: false }] },
        SmartMask { name: "Thickness", effects: vec![MaskEffect::MeshMap { kind: MeshMapKind::Thickness, invert: true }] },
        SmartMask {
            name: "Dirt",
            effects: vec![
                MaskEffect::Generator(Generator::default_perlin()),
                MaskEffect::Filter(Filter::Levels { in_black: 0.45, in_white: 0.7, gamma: 1.0, out_black: 0.0, out_white: 1.0 }),
            ],
        },
    ]
}

/// Find a smart mask by name.
pub fn smart_mask(name: &str) -> Option<SmartMask> {
    smart_masks().into_iter().find(|mask| mask.name == name)
}

/// A fill layer with a mask-effect stack.
fn masked_fill(name: &str, color: [f32; 4], value: f32, effects: Vec<MaskEffect>) -> Layer {
    let mut layer = Layer::fill(name, color, value);
    layer.mask = Some(Mask::Stack { effects });
    layer
}

fn worn_metal() -> Vec<Layer> {
    let edges = vec![
        MaskEffect::MeshMap { kind: MeshMapKind::Curvature, invert: false },
        MaskEffect::Filter(Filter::Levels { in_black: 0.5, in_white: 0.75, gamma: 1.0, out_black: 0.0, out_white: 1.0 }),
    ];
    vec![
        Layer::fill("Metal Base", [0.55, 0.56, 0.58, 1.0], 0.2),
        Layer::fill("Metal Roughness", [0.0, 0.0, 0.0, 1.0], 0.45),
        masked_fill("Edge Wear", [0.85, 0.85, 0.88, 1.0], 0.15, edges),
    ]
}

fn rusty_iron() -> Vec<Layer> {
    let rust = vec![
        MaskEffect::Generator(Generator::default_perlin()),
        MaskEffect::Filter(Filter::Levels { in_black: 0.4, in_white: 0.6, gamma: 1.0, out_black: 0.0, out_white: 1.0 }),
    ];
    vec![Layer::fill("Iron", [0.4, 0.4, 0.42, 1.0], 0.5), masked_fill("Rust", [0.45, 0.2, 0.08, 1.0], 0.85, rust)]
}

fn old_wood() -> Vec<Layer> {
    let grain = vec![
        MaskEffect::Generator(Generator::default_simplex()),
        MaskEffect::Filter(Filter::Levels { in_black: 0.35, in_white: 0.65, gamma: 1.0, out_black: 0.0, out_white: 1.0 }),
    ];
    vec![Layer::fill("Wood", [0.4, 0.26, 0.13, 1.0], 0.7), masked_fill("Grain", [0.25, 0.15, 0.07, 1.0], 0.75, grain)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_have_unique_names_and_round_trip() {
        for preset in material_presets() {
            assert!(material_preset(preset.name).is_some());
        }
        for material in smart_materials() {
            assert!(!material.layers.is_empty());
            assert!(smart_material(material.name).is_some());
        }
        for mask in smart_masks() {
            assert!(!mask.effects.is_empty());
            assert!(smart_mask(mask.name).is_some());
        }
        assert!(material_preset("Nope").is_none());
    }
}
