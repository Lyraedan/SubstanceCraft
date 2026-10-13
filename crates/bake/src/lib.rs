//! SubstanceCraft mesh-map bakers.
//!
//! Ray-based bakers that project model information onto a low-poly UV layout: position, world and
//! tangent-space normal, ambient occlusion, thickness, curvature and material ID. Picking uses the
//! `geom` BVH, so baking is fast even on dense meshes.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::f32::consts::TAU;

use substancecraft_geom::glam::{Vec2, Vec3, Vec4};
use substancecraft_geom::{Bvh, Mesh, Ray};
use substancecraft_raster::RasterImage;

/// The mesh maps that can be baked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BakeMap {
    /// Object-space position, normalised to the mesh bounds.
    Position,
    /// World/object-space normal.
    WorldNormal,
    /// Tangent-space normal from a high-poly mesh (needs a high-poly).
    TangentNormal,
    /// Ambient occlusion.
    AmbientOcclusion,
    /// Thickness (inverse distance to the back face).
    Thickness,
    /// Curvature derived from the normal field.
    Curvature,
    /// Per-material colour ID.
    MaterialId,
}

impl BakeMap {
    /// Every map, in UI order.
    pub const ALL: [BakeMap; 7] = [
        BakeMap::Position,
        BakeMap::WorldNormal,
        BakeMap::TangentNormal,
        BakeMap::AmbientOcclusion,
        BakeMap::Thickness,
        BakeMap::Curvature,
        BakeMap::MaterialId,
    ];

    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            BakeMap::Position => "position",
            BakeMap::WorldNormal => "world_normal",
            BakeMap::TangentNormal => "tangent_normal",
            BakeMap::AmbientOcclusion => "ambient_occlusion",
            BakeMap::Thickness => "thickness",
            BakeMap::Curvature => "curvature",
            BakeMap::MaterialId => "material_id",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            BakeMap::Position => "Position",
            BakeMap::WorldNormal => "World Normal",
            BakeMap::TangentNormal => "Normal",
            BakeMap::AmbientOcclusion => "Ambient Occlusion",
            BakeMap::Thickness => "Thickness",
            BakeMap::Curvature => "Curvature",
            BakeMap::MaterialId => "Material ID",
        }
    }

    /// Parse a map from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        BakeMap::ALL.into_iter().find(|map| map.id() == id)
    }

    /// Whether this map needs a high-poly mesh.
    pub fn needs_high_poly(self) -> bool {
        matches!(self, BakeMap::TangentNormal)
    }
}

/// Bake parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BakeSettings {
    /// Output edge length in pixels.
    pub resolution: u32,
    /// Hemisphere samples for ambient occlusion.
    pub samples: u32,
    /// Cage / frontal distance as a fraction of the mesh radius.
    pub max_distance: f32,
    /// Occlusion ray length as a fraction of the mesh radius.
    pub ao_distance: f32,
    /// Random seed for the AO sample rotation.
    pub seed: u32,
}

impl Default for BakeSettings {
    fn default() -> Self {
        Self { resolution: 256, samples: 16, max_distance: 0.05, ao_distance: 0.1, seed: 0 }
    }
}

/// A configured baker for a low-poly (and optional high-poly) mesh.
pub struct Baker<'a> {
    low: &'a Mesh,
    high: Option<&'a Mesh>,
    low_bvh: Bvh,
    high_bvh: Option<Bvh>,
    materials: Vec<u32>,
    radius: f32,
}

impl<'a> Baker<'a> {
    /// Build a baker, indexing the low and optional high mesh.
    pub fn new(low: &'a Mesh, high: Option<&'a Mesh>) -> Self {
        let low_bvh = Bvh::build(low);
        let high_bvh = high.map(Bvh::build);
        let materials = triangle_materials(low);
        let radius = low.bounds().radius().max(1e-4);
        Self { low, high, low_bvh, high_bvh, materials, radius }
    }

    /// Bake one map at the requested resolution.
    pub fn bake(&self, map: BakeMap, settings: &BakeSettings) -> RasterImage {
        let resolution = settings.resolution.clamp(16, 2048);
        match map {
            BakeMap::Position => self.bake_position(resolution),
            BakeMap::WorldNormal => self.bake_world_normal(resolution),
            BakeMap::TangentNormal => self.bake_tangent_normal(resolution, settings),
            BakeMap::AmbientOcclusion => self.bake_ao(resolution, settings),
            BakeMap::Thickness => self.bake_thickness(resolution, settings),
            BakeMap::MaterialId => self.bake_material_id(resolution),
            BakeMap::Curvature => self.bake_curvature(resolution),
        }
    }

    fn bake_position(&self, resolution: u32) -> RasterImage {
        let bounds = self.low.bounds();
        let size = bounds.size();
        let inv = Vec3::new(1.0 / size.x.max(1e-6), 1.0 / size.y.max(1e-6), 1.0 / size.z.max(1e-6));
        let mut image = RasterImage::filled(resolution, resolution, [0.0, 0.0, 0.0, 1.0]);
        self.rasterize(resolution, |x, y, sample| {
            let normalized = (sample.position - bounds.min) * inv;
            image.set_pixel(x, y, [normalized.x.clamp(0.0, 1.0), normalized.y.clamp(0.0, 1.0), normalized.z.clamp(0.0, 1.0), 1.0]);
        });
        image
    }

    fn bake_world_normal(&self, resolution: u32) -> RasterImage {
        let mut image = RasterImage::filled(resolution, resolution, [0.5, 0.5, 1.0, 1.0]);
        self.rasterize(resolution, |x, y, sample| {
            let n = sample.normal;
            image.set_pixel(x, y, [n.x * 0.5 + 0.5, n.y * 0.5 + 0.5, n.z * 0.5 + 0.5, 1.0]);
        });
        image
    }

    fn bake_tangent_normal(&self, resolution: u32, settings: &BakeSettings) -> RasterImage {
        let distance = settings.max_distance * self.radius;
        let mut image = RasterImage::filled(resolution, resolution, [0.5, 0.5, 1.0, 1.0]);
        if self.high_bvh.is_none() {
            return image;
        }
        let high = self.high;
        let high_bvh = self.high_bvh.as_ref();
        self.rasterize(resolution, |x, y, sample| {
            let Some(high_mesh) = high else { return };
            let Some(bvh) = high_bvh else { return };
            let normal = sample.normal;
            let origin = sample.position + normal * distance * 0.5;
            let ray = Ray { origin: origin - normal * distance, direction: normal };
            let Some(hit) = bvh.intersect(high_mesh, &ray) else { return };
            let (tangent, bitangent) = tangent_frame(normal, sample.tangent);
            let high_normal = hit.normal;
            let tangent_normal = Vec3::new(high_normal.dot(tangent), high_normal.dot(bitangent), high_normal.dot(normal));
            image.set_pixel(x, y, [tangent_normal.x * 0.5 + 0.5, tangent_normal.y * 0.5 + 0.5, tangent_normal.z * 0.5 + 0.5, 1.0]);
        });
        image
    }

    fn bake_ao(&self, resolution: u32, settings: &BakeSettings) -> RasterImage {
        let samples = settings.samples.clamp(1, 256);
        let distance = settings.ao_distance * self.radius;
        let epsilon = self.radius * 1e-3;
        let mut image = RasterImage::filled(resolution, resolution, [1.0, 1.0, 1.0, 1.0]);
        self.rasterize(resolution, |x, y, sample| {
            let (tangent, bitangent) = tangent_frame(sample.normal, sample.tangent);
            let mut occluded = 0.0f32;
            for index in 0..samples {
                let (u1, u2) = hammersley(index, samples, settings.seed);
                let local = cosine_hemisphere(u1, u2);
                let direction = tangent * local.x + bitangent * local.y + sample.normal * local.z;
                let ray = Ray { origin: sample.position + sample.normal * epsilon, direction };
                if self.low_bvh.occluded(self.low, &ray, distance) {
                    occluded += 1.0;
                }
            }
            let value = (1.0 - occluded / samples as f32).clamp(0.0, 1.0);
            image.set_pixel(x, y, [value, value, value, 1.0]);
        });
        image
    }

    fn bake_thickness(&self, resolution: u32, settings: &BakeSettings) -> RasterImage {
        let distance = (settings.max_distance * 4.0).max(0.2) * self.radius;
        let epsilon = self.radius * 1e-3;
        let mut image = RasterImage::filled(resolution, resolution, [1.0, 1.0, 1.0, 1.0]);
        self.rasterize(resolution, |x, y, sample| {
            let ray = Ray { origin: sample.position - sample.normal * epsilon, direction: -sample.normal };
            let value = match self.low_bvh.intersect(self.low, &ray) {
                Some(hit) if hit.distance <= distance => (hit.distance / distance).clamp(0.0, 1.0),
                _ => 1.0,
            };
            image.set_pixel(x, y, [value, value, value, 1.0]);
        });
        image
    }

    fn bake_material_id(&self, resolution: u32) -> RasterImage {
        let mut image = RasterImage::filled(resolution, resolution, [0.0, 0.0, 0.0, 1.0]);
        self.rasterize(resolution, |x, y, sample| {
            let color = material_color(sample.material);
            image.set_pixel(x, y, [color.x, color.y, color.z, 1.0]);
        });
        image
    }

    fn bake_curvature(&self, resolution: u32) -> RasterImage {
        // Bake world normals, then take the divergence of the normal field.
        let normals = self.bake_world_normal(resolution);
        let mut image = RasterImage::filled(resolution, resolution, [0.5, 0.5, 0.5, 1.0]);
        let sample_channel = |x: u32, y: u32, c: usize| -> f32 { normals.pixel(x.min(resolution - 1), y.min(resolution - 1))[c] };
        for y in 0..resolution {
            for x in 0..resolution {
                let left = sample_channel(x.saturating_sub(1), y, 0);
                let right = sample_channel(x + 1, y, 0);
                let up = sample_channel(x, y.saturating_sub(1), 1);
                let down = sample_channel(x, y + 1, 1);
                let curvature = ((right - left) + (down - up)) * 4.0;
                let value = (0.5 + curvature).clamp(0.0, 1.0);
                image.set_pixel(x, y, [value, value, value, 1.0]);
            }
        }
        image
    }

    /// Rasterise every low-poly triangle and visit covered texels with interpolated attributes.
    fn rasterize(&self, resolution: u32, mut visit: impl FnMut(u32, u32, TexelSample)) {
        let triangles = self.low.indices.as_chunks::<3>().0;
        for (triangle_index, triangle) in triangles.iter().enumerate() {
            let [i0, i1, i2] = *triangle;
            let (Some(p0), Some(p1), Some(p2)) = (position(self.low, i0), position(self.low, i1), position(self.low, i2)) else {
                continue;
            };
            let (Some(uv0), Some(uv1), Some(uv2)) = (uv(self.low, i0), uv(self.low, i1), uv(self.low, i2)) else {
                continue;
            };
            let material = self.materials.get(triangle_index).copied().unwrap_or(0);

            let a = Vec2::new(uv0.x * resolution as f32, uv0.y * resolution as f32);
            let b = Vec2::new(uv1.x * resolution as f32, uv1.y * resolution as f32);
            let c = Vec2::new(uv2.x * resolution as f32, uv2.y * resolution as f32);
            let area = edge(a, b, c);
            if area.abs() < 1e-9 {
                continue;
            }
            let min_x = a.x.min(b.x).min(c.x).floor().max(0.0) as u32;
            let max_x = (a.x.max(b.x).max(c.x).ceil() as i64).clamp(0, resolution as i64 - 1) as u32;
            let min_y = a.y.min(b.y).min(c.y).floor().max(0.0) as u32;
            let max_y = (a.y.max(b.y).max(c.y).ceil() as i64).clamp(0, resolution as i64 - 1) as u32;

            for y in min_y..=max_y {
                for x in min_x..=max_x {
                    let point = Vec2::new(x as f32 + 0.5, y as f32 + 0.5);
                    let w0 = edge(b, c, point) / area;
                    let w1 = edge(c, a, point) / area;
                    let w2 = edge(a, b, point) / area;
                    if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                        continue;
                    }
                    let normal = (normal(self.low, i0) * w0 + normal(self.low, i1) * w1 + normal(self.low, i2) * w2).normalize_or_zero();
                    let sample = TexelSample {
                        position: p0 * w0 + p1 * w1 + p2 * w2,
                        normal,
                        tangent: tangent(self.low, i0) * w0 + tangent(self.low, i1) * w1 + tangent(self.low, i2) * w2,
                        material,
                    };
                    visit(x, y, sample);
                }
            }
        }
    }
}

/// Interpolated per-texel attributes.
struct TexelSample {
    position: Vec3,
    normal: Vec3,
    tangent: Vec4,
    material: u32,
}

fn position(mesh: &Mesh, index: u32) -> Option<Vec3> {
    mesh.positions.get(index as usize).copied()
}

fn normal(mesh: &Mesh, index: u32) -> Vec3 {
    mesh.normals.get(index as usize).copied().unwrap_or(Vec3::Z)
}

fn tangent(mesh: &Mesh, index: u32) -> Vec4 {
    mesh.tangents.get(index as usize).copied().unwrap_or(Vec4::new(1.0, 0.0, 0.0, 1.0))
}

fn uv(mesh: &Mesh, index: u32) -> Option<Vec2> {
    mesh.uvs.get(index as usize).copied()
}

/// The material id for each triangle, from the mesh's submeshes.
fn triangle_materials(mesh: &Mesh) -> Vec<u32> {
    let mut materials = vec![0u32; mesh.indices.len() / 3];
    for submesh in &mesh.submeshes {
        let first = submesh.first_index as usize / 3;
        let count = submesh.index_count as usize / 3;
        for triangle in first..(first + count).min(materials.len()) {
            materials[triangle] = submesh.material;
        }
    }
    materials
}

fn edge(a: Vec2, b: Vec2, p: Vec2) -> f32 {
    (b.x - a.x) * (p.y - a.y) - (b.y - a.y) * (p.x - a.x)
}

/// An orthonormal tangent frame around a normal, using the mesh tangent when available.
fn tangent_frame(normal: Vec3, mesh_tangent: Vec4) -> (Vec3, Vec3) {
    let n = normal.normalize_or_zero();
    let candidate = mesh_tangent.truncate();
    let mut tangent = candidate - n * n.dot(candidate);
    if tangent.length_squared() < 1e-8 {
        let reference = if n.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
        tangent = reference.cross(n);
    }
    let tangent = tangent.normalize_or_zero();
    let bitangent = n.cross(tangent) * if mesh_tangent.w < 0.0 { -1.0 } else { 1.0 };
    (tangent, bitangent)
}

/// Hammersley point `(i, n)` with a rotation offset.
fn hammersley(i: u32, n: u32, seed: u32) -> (f32, f32) {
    let u1 = (i as f32 + 0.5) / n as f32;
    let u2 = radical_inverse(i.wrapping_add(seed.wrapping_mul(2654435761)));
    (u1, u2)
}

/// Van der Corput radical inverse in base 2.
fn radical_inverse(mut bits: u32) -> f32 {
    bits = bits.reverse_bits();
    bits as f32 * 2.328_306_4e-10
}

/// Cosine-weighted hemisphere direction from two uniform samples.
fn cosine_hemisphere(u1: f32, u2: f32) -> Vec3 {
    let r = u1.sqrt();
    let phi = TAU * u2;
    Vec3::new(r * phi.cos(), r * phi.sin(), (1.0 - u1).sqrt())
}

/// A stable distinct colour per material id.
fn material_color(material: u32) -> Vec3 {
    let hue = (material as f32 * 0.618_034).fract() * 6.0;
    let y = hue - hue.floor();
    let (r, g, b) = match hue as u32 {
        0 => (1.0, y, 0.0),
        1 => (1.0 - y, 1.0, 0.0),
        2 => (0.0, 1.0, y),
        3 => (0.0, 1.0 - y, 1.0),
        4 => (y, 0.0, 1.0),
        _ => (1.0, 0.0, 1.0 - y),
    };
    Vec3::new(r * 0.9 + 0.05, g * 0.9 + 0.05, b * 0.9 + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_geom::Submesh;

    fn quad() -> Mesh {
        Mesh {
            positions: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(0.0, 1.0, 0.0)],
            normals: vec![Vec3::Z; 4],
            uvs: vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)],
            tangents: vec![Vec4::new(1.0, 0.0, 0.0, 1.0); 4],
            indices: vec![0, 1, 2, 0, 2, 3],
            submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 6 }],
        }
    }

    #[test]
    fn world_normal_is_z_up() {
        let mesh = quad();
        let baker = Baker::new(&mesh, None);
        let image = baker.bake(BakeMap::WorldNormal, &BakeSettings { resolution: 32, ..Default::default() });
        let center = image.pixel(16, 16);
        assert!((center[0] - 0.5).abs() < 0.05 && (center[1] - 0.5).abs() < 0.05, "expected +Z normal, got {center:?}");
        assert!(center[2] > 0.95);
    }

    #[test]
    fn position_spans_the_bounds() {
        let mesh = quad();
        let baker = Baker::new(&mesh, None);
        let image = baker.bake(BakeMap::Position, &BakeSettings { resolution: 32, ..Default::default() });
        // A corner is near (0,0,0) and the opposite corner near (1,1,0).
        let near_origin = image.pixel(1, 1);
        assert!(near_origin[0] < 0.1 && near_origin[1] < 0.1);
        let far = image.pixel(30, 30);
        assert!(far[0] > 0.85 && far[1] > 0.85);
    }

    #[test]
    fn flat_plane_is_unoccluded() {
        let mesh = quad();
        let baker = Baker::new(&mesh, None);
        let image = baker.bake(BakeMap::AmbientOcclusion, &BakeSettings { resolution: 16, samples: 8, ..Default::default() });
        // A single flat plane occludes very little.
        let center = image.pixel(8, 8);
        assert!(center[0] > 0.75, "flat plane should be mostly unoccluded: {center:?}");
    }

    #[test]
    fn tangent_normal_defaults_without_high_poly() {
        let mesh = quad();
        let baker = Baker::new(&mesh, None);
        let image = baker.bake(BakeMap::TangentNormal, &BakeSettings { resolution: 16, ..Default::default() });
        let center = image.pixel(8, 8);
        assert!((center[2] - 1.0).abs() < 1e-6, "without a high-poly the normal should be flat +Z");
    }

    #[test]
    fn material_id_is_distinct_per_material() {
        let mut mesh = quad();
        let index_count = mesh.indices.len() as u32;
        mesh.submeshes =
            vec![Submesh { material: 0, first_index: 0, index_count: 3 }, Submesh { material: 1, first_index: 3, index_count: index_count - 3 }];
        let baker = Baker::new(&mesh, None);
        let image = baker.bake(BakeMap::MaterialId, &BakeSettings { resolution: 32, ..Default::default() });
        assert_ne!(material_color(0), material_color(1));
        // The lower triangle uses material 0, upper uses 1.
        assert_ne!(image.pixel(16, 4), image.pixel(16, 28));
    }

    #[test]
    fn curvature_of_a_flat_plane_is_mid_grey() {
        let mesh = quad();
        let baker = Baker::new(&mesh, None);
        let image = baker.bake(BakeMap::Curvature, &BakeSettings { resolution: 32, ..Default::default() });
        let center = image.pixel(16, 16);
        assert!((center[0] - 0.5).abs() < 0.1, "flat surface curvature should be ~0.5, got {center:?}");
    }

    #[test]
    fn tangent_frame_is_orthonormal() {
        let (t, b) = tangent_frame(Vec3::Z, Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert!(t.dot(b).abs() < 1e-5 && (t.length() - 1.0).abs() < 1e-4 && (b.length() - 1.0).abs() < 1e-4);
    }
}
