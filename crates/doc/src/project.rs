//! The project: the scene plus its texture sets.
//!
//! A project splits the imported mesh into **texture sets** by material id (and, for UDIM
//! projects, by UV tile). Each texture set owns a resolution, a set of channels to paint and a
//! list of UV tiles. This is pure data: no GPU, no UI.

use std::collections::BTreeSet;

use substancecraft_geom::Mesh;

use crate::Scene;
use crate::layers::LayerStack;
use crate::shader::{Shader, ShaderParams};

/// The default texture resolution for a new set.
pub const DEFAULT_RESOLUTION: u32 = 2048;

/// A surface channel a texture set can hold. The set mirrors the OpenPBR channel family; the
/// metal/rough workflow is the default.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Channel {
    /// Linear base colour (albedo).
    BaseColor,
    /// Metalness.
    Metallic,
    /// Roughness.
    Roughness,
    /// Tangent-space normal.
    Normal,
    /// Displacement / height.
    Height,
    /// Ambient occlusion.
    AmbientOcclusion,
    /// Emissive colour.
    Emissive,
}

impl Channel {
    /// Every channel, in UI order.
    pub const ALL: [Channel; 7] =
        [Channel::BaseColor, Channel::Metallic, Channel::Roughness, Channel::Normal, Channel::Height, Channel::AmbientOcclusion, Channel::Emissive];

    /// A stable id for commands, files and menus.
    pub fn id(self) -> &'static str {
        match self {
            Channel::BaseColor => "base_color",
            Channel::Metallic => "metallic",
            Channel::Roughness => "roughness",
            Channel::Normal => "normal",
            Channel::Height => "height",
            Channel::AmbientOcclusion => "ambient_occlusion",
            Channel::Emissive => "emissive",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Channel::BaseColor => "Base Color",
            Channel::Metallic => "Metallic",
            Channel::Roughness => "Roughness",
            Channel::Normal => "Normal",
            Channel::Height => "Height",
            Channel::AmbientOcclusion => "Ambient Occlusion",
            Channel::Emissive => "Emissive",
        }
    }

    /// Parse a channel from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        Channel::ALL.into_iter().find(|channel| channel.id() == id)
    }

    /// The default metal/rough channel set.
    pub fn metal_rough_default() -> Vec<Channel> {
        vec![Channel::BaseColor, Channel::Metallic, Channel::Roughness, Channel::Normal, Channel::Height, Channel::AmbientOcclusion]
    }
}

/// A UDIM tile coordinate. Tile `(0, 0)` is UV `0..1`; UDIM `1001 + u + 10·v`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UvTile {
    /// Tile column.
    pub u: u32,
    /// Tile row.
    pub v: u32,
}

impl UvTile {
    /// The base tile.
    pub const DEFAULT: Self = Self { u: 0, v: 0 };

    /// The UDIM number (1001 + u + 10·v).
    pub fn udim(self) -> u32 {
        1001 + self.u + self.v.saturating_mul(10)
    }

    /// Parse a UDIM number (must be ≥ 1001 and well-formed).
    pub fn from_udim(udim: u32) -> Option<Self> {
        let n = udim.checked_sub(1001)?;
        let u = n % 10;
        let v = n / 10;
        (v < 10).then_some(Self { u, v })
    }

    /// The UV-space offset of the tile's lower-left corner.
    pub fn offset(self) -> [f32; 2] {
        [self.u as f32, self.v as f32]
    }
}

/// A texture set: one paintable material configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct TextureSet {
    /// Display name (also used at export).
    pub name: String,
    /// The original material name from the source file, for a "reset name" action.
    pub source_name: Option<String>,
    /// The scene material ids this set paints.
    pub materials: Vec<u32>,
    /// Working resolution in pixels.
    pub resolution: u32,
    /// The active channels, in UI order.
    pub channels: Vec<Channel>,
    /// The UV tiles this set covers.
    pub uv_tiles: Vec<UvTile>,
    /// The non-destructive layer stack.
    pub layers: LayerStack,
    /// The shading model assigned to this set.
    pub shader: Shader,
    /// Editable shader parameters.
    pub shader_params: ShaderParams,
}

impl TextureSet {
    /// A new set with the default resolution, metal/rough channels, the base UV tile, an empty
    /// layer stack and the default OpenPBR shader.
    pub fn new(name: impl Into<String>, materials: Vec<u32>) -> Self {
        Self {
            name: name.into(),
            source_name: None,
            materials,
            resolution: DEFAULT_RESOLUTION,
            channels: Channel::metal_rough_default(),
            uv_tiles: vec![UvTile::DEFAULT],
            layers: LayerStack::new(),
            shader: Shader::default(),
            shader_params: ShaderParams::default(),
        }
    }

    /// Whether the set paints this channel.
    pub fn has_channel(&self, channel: Channel) -> bool {
        self.channels.contains(&channel)
    }

    /// Enable or disable a channel, keeping UI order.
    pub fn set_channel(&mut self, channel: Channel, enabled: bool) {
        if enabled {
            if !self.channels.contains(&channel) {
                self.channels.push(channel);
                self.channels.sort();
            }
        } else {
            self.channels.retain(|existing| *existing != channel);
        }
    }

    /// Whether this set paints any of the given materials.
    pub fn paints(&self, material: u32) -> bool {
        self.materials.contains(&material)
    }
}

/// The open project: a scene and its texture sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Project {
    /// Project name.
    pub name: String,
    /// The imported geometry.
    pub scene: Scene,
    /// The texture sets, at least one.
    pub texture_sets: Vec<TextureSet>,
    /// The selected texture set index.
    pub active_texture_set: usize,
}

impl Default for Project {
    fn default() -> Self {
        Self::new("untitled")
    }
}

impl Project {
    /// A new, empty project with a single default texture set.
    pub fn new(name: impl Into<String>) -> Self {
        Self::from_scene(name, Scene::default())
    }

    /// Build a project from a scene, splitting it into one texture set per material id used.
    pub fn from_scene(name: impl Into<String>, scene: Scene) -> Self {
        let texture_sets = split_texture_sets(&scene);
        Self { name: name.into(), scene, texture_sets, active_texture_set: 0 }
    }

    /// Re-derive the texture sets from the current scene, keeping the name. Used after import.
    pub fn rebuild_texture_sets(&mut self) {
        self.texture_sets = split_texture_sets(&self.scene);
        if self.active_texture_set >= self.texture_sets.len() {
            self.active_texture_set = 0;
        }
    }

    /// The active texture set, if any.
    pub fn active_set(&self) -> Option<&TextureSet> {
        self.texture_sets.get(self.active_texture_set)
    }

    /// The active texture set, mutably.
    pub fn active_set_mut(&mut self) -> Option<&mut TextureSet> {
        self.texture_sets.get_mut(self.active_texture_set)
    }

    /// Select a set by index; returns false if out of range.
    pub fn select(&mut self, index: usize) -> bool {
        if index < self.texture_sets.len() {
            self.active_texture_set = index;
            true
        } else {
            false
        }
    }

    /// The number of triangles a set covers (its material submeshes).
    pub fn triangles_for_set(&self, index: usize) -> usize {
        let Some(set) = self.texture_sets.get(index) else { return 0 };
        self.scene.mesh.submeshes.iter().filter(|submesh| set.paints(submesh.material)).map(|submesh| submesh.index_count as usize / 3).sum()
    }

    /// The material ids painted by the active set, for viewport isolation.
    pub fn active_materials(&self) -> Vec<u32> {
        self.active_set().map(|set| set.materials.clone()).unwrap_or_default()
    }
}

/// Split a scene into texture sets, one per material id referenced by a submesh.
fn split_texture_sets(scene: &Scene) -> Vec<TextureSet> {
    let mut material_ids: Vec<u32> = Vec::new();
    for submesh in &scene.mesh.submeshes {
        if !material_ids.contains(&submesh.material) {
            material_ids.push(submesh.material);
        }
    }
    if material_ids.is_empty() {
        material_ids.push(0);
    }

    material_ids
        .into_iter()
        .map(|material| {
            let name = scene
                .materials
                .get(material as usize)
                .map(|m| m.name.clone())
                .filter(|name| !name.is_empty())
                .unwrap_or_else(|| format!("texture_set_{material}"));
            let mut set = TextureSet::new(name.clone(), vec![material]);
            set.source_name = Some(name);
            set.uv_tiles = tiles_for_material(&scene.mesh, material);
            set
        })
        .collect()
}

/// The distinct UDIM tiles used by the submeshes of one material.
fn tiles_for_material(mesh: &Mesh, material: u32) -> Vec<UvTile> {
    let mut tiles = BTreeSet::new();
    for submesh in mesh.submeshes.iter().filter(|submesh| submesh.material == material) {
        let start = submesh.first_index as usize;
        let end = start.saturating_add(submesh.index_count as usize).min(mesh.indices.len());
        for &vertex in &mesh.indices[start..end] {
            if let Some(uv) = mesh.uvs.get(vertex as usize) {
                // Subtract a hair so a UV exactly on a tile boundary stays in the lower tile, and
                // guard NaN by folding it to 0.
                let u = (uv.x - 1e-5).max(0.0).floor() as u32;
                let v = (uv.y - 1e-5).max(0.0).floor() as u32;
                if u < 10 && v < 10 {
                    tiles.insert(UvTile { u, v });
                }
            }
        }
    }
    if tiles.is_empty() { vec![UvTile::DEFAULT] } else { tiles.into_iter().collect() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_geom::primitives::uv_sphere;

    fn scene_with_materials() -> Scene {
        let mut scene = Scene::from_mesh(uv_sphere(1.0, 8, 12).expect("sphere"));
        scene.materials =
            vec![crate::Material { name: "metal".into(), ..Default::default() }, crate::Material { name: "plastic".into(), ..Default::default() }];
        scene
    }

    #[test]
    fn splits_one_set_per_material() {
        let mut scene = scene_with_materials();
        // Give the mesh two submeshes referencing different materials.
        let total = scene.mesh.indices.len() as u32;
        scene.mesh.submeshes = vec![
            substancecraft_geom::Submesh { material: 0, first_index: 0, index_count: total / 2 },
            substancecraft_geom::Submesh { material: 1, first_index: total / 2, index_count: total - total / 2 },
        ];
        let project = Project::from_scene("test", scene);
        assert_eq!(project.texture_sets.len(), 2);
        assert_eq!(project.texture_sets[0].name, "metal");
        assert_eq!(project.texture_sets[1].name, "plastic");
        assert_eq!(project.texture_sets[0].materials, vec![0]);
    }

    #[test]
    fn uv_tiles_derive_from_uvs() {
        let mut mesh = uv_sphere(1.0, 8, 12).expect("sphere");
        // Shift half the UVs into tile (1, 0).
        for uv in &mut mesh.uvs {
            if uv.x > 0.5 {
                uv.x += 1.0;
            }
        }
        let scene = Scene::from_mesh(mesh);
        let project = Project::from_scene("test", scene);
        let tiles = &project.texture_sets[0].uv_tiles;
        assert!(tiles.contains(&UvTile::DEFAULT));
        assert!(tiles.contains(&UvTile { u: 1, v: 0 }));
    }

    #[test]
    fn channel_toggle_keeps_order_and_dedupes() {
        let mut set = TextureSet::new("s", vec![0]);
        assert!(set.has_channel(Channel::BaseColor));
        set.set_channel(Channel::BaseColor, false);
        assert!(!set.has_channel(Channel::BaseColor));
        set.set_channel(Channel::Metallic, true);
        set.set_channel(Channel::Metallic, true);
        assert_eq!(set.channels.iter().filter(|c| **c == Channel::Metallic).count(), 1);
    }

    #[test]
    fn udim_round_trips() {
        for tile in [UvTile::DEFAULT, UvTile { u: 3, v: 2 }, UvTile { u: 9, v: 9 }] {
            assert_eq!(UvTile::from_udim(tile.udim()), Some(tile));
        }
        assert_eq!(UvTile::from_udim(1000), None);
        assert_eq!(UvTile::DEFAULT.udim(), 1001);
    }

    #[test]
    fn uv_at_tile_boundary_stays_in_the_base_tile() {
        let mut mesh = uv_sphere(1.0, 6, 8).expect("sphere");
        if let Some(uv) = mesh.uvs.first_mut() {
            uv.x = 1.0;
            uv.y = 1.0;
        }
        let project = Project::from_scene("test", Scene::from_mesh(mesh));
        assert_eq!(project.texture_sets[0].uv_tiles, vec![UvTile::DEFAULT]);
    }

    #[test]
    fn triangles_for_set_counts_its_submeshes() {
        let project = Project::from_scene("test", Scene::from_mesh(uv_sphere(1.0, 8, 12).expect("sphere")));
        let total = project.scene.mesh.triangle_count();
        assert_eq!(project.triangles_for_set(0), total);
    }
}
