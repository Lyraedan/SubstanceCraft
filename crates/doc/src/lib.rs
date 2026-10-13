//! SubstanceCraft document model.
//!
//! The project, its texture sets, the per-set layer stacks (fill / paint / effect /
//! group), masks, generators, anchors and channel configuration. This is pure data:
//! no GPU, no UI, no I/O.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use serde::{Deserialize, Serialize};
use substancecraft_geom::Mesh;

pub mod layers;
pub mod procedural;
pub mod project;
pub mod shader;

pub use layers::{BlendMode, Layer, LayerKind, LayerStack, Mask};
pub use procedural::{Effect, Filter, Generator};
pub use project::{Channel, Project, TextureSet, UvTile};
pub use shader::{Shader, ShaderParams};

/// A PBR material description. At M1 this is the flat metal/rough parameter set; it grows into
/// the full channel/shader description in later milestones.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Material {
    /// The material name from the source file.
    pub name: String,
    /// Linear base colour (albedo), RGBA.
    pub base_color: [f32; 4],
    /// Metalness in `0..=1`.
    pub metallic: f32,
    /// Roughness in `0..=1`.
    pub roughness: f32,
}

impl Default for Material {
    fn default() -> Self {
        Self { name: "default".to_string(), base_color: [0.8, 0.8, 0.8, 1.0], metallic: 0.0, roughness: 0.5 }
    }
}

/// The imported geometry and its materials: the canvas a project paints on.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Scene {
    /// The mesh (one combined mesh with per-material submeshes).
    pub mesh: Mesh,
    /// The materials referenced by the mesh's submeshes.
    pub materials: Vec<Material>,
}

impl Scene {
    /// A scene holding a single mesh with one default material.
    pub fn from_mesh(mesh: Mesh) -> Self {
        let mut mesh = mesh;
        mesh.complete_attributes();
        Self { mesh, materials: vec![Material::default()] }
    }

    /// Whether the scene has drawable geometry.
    pub fn is_empty(&self) -> bool {
        self.mesh.is_empty()
    }

    /// The material for a submesh, falling back to the first material (or a default).
    pub fn material_for(&self, index: u32) -> Material {
        self.materials.get(index as usize).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_mesh_completes_attributes_and_adds_a_default_material() {
        let mesh = substancecraft_geom::primitives::uv_sphere(1.0, 8, 12).expect("sphere");
        let scene = Scene::from_mesh(mesh);
        assert!(!scene.is_empty());
        assert_eq!(scene.materials.len(), 1);
        assert_eq!(scene.mesh.tangents.len(), scene.mesh.vertex_count());
    }

    #[test]
    fn missing_material_falls_back_to_default() {
        let scene = Scene::default();
        assert_eq!(scene.material_for(7).name, "default");
    }
}
