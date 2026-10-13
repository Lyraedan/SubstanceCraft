//! An indexed triangle mesh with PBR-ready attributes.

use glam::{Vec2, Vec3, Vec4};

use crate::GeometryError;
use crate::bounds::Bounds;
use crate::tangents;

/// A contiguous run of indices sharing one material.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Submesh {
    /// Index into the parent scene's material list.
    pub material: u32,
    /// Offset of the first index in the mesh index buffer.
    pub first_index: u32,
    /// Number of indices in this run (a multiple of three).
    pub index_count: u32,
}

/// An indexed triangle mesh. All per-vertex arrays are parallel: index `i` names the same vertex
/// in `positions`, `normals`, `uvs` and `tangents`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mesh {
    /// Vertex positions, object space.
    pub positions: Vec<Vec3>,
    /// Vertex normals, object space (unit length once generated).
    pub normals: Vec<Vec3>,
    /// First texture-coordinate set.
    pub uvs: Vec<Vec2>,
    /// Tangent vectors; `w` is the bitangent sign (MikkTSpace convention).
    pub tangents: Vec<Vec4>,
    /// Triangle indices into the vertex arrays.
    pub indices: Vec<u32>,
    /// Material runs over the index buffer.
    pub submeshes: Vec<Submesh>,
}

impl Mesh {
    /// An empty mesh.
    pub fn new() -> Self {
        Self::default()
    }

    /// The number of vertices.
    pub fn vertex_count(&self) -> usize {
        self.positions.len()
    }

    /// The number of triangles.
    pub fn triangle_count(&self) -> usize {
        self.indices.len() / 3
    }

    /// Whether the mesh has any triangles.
    pub fn is_empty(&self) -> bool {
        self.positions.is_empty() || self.indices.is_empty()
    }

    /// The mesh bounding box over its positions.
    pub fn bounds(&self) -> Bounds {
        Bounds::from_points(&self.positions)
    }

    /// Check structural invariants: a non-empty position array, parallel attributes, in-range
    /// indices and a triangle-multiple index count.
    pub fn validate(&self) -> Result<(), GeometryError> {
        if self.positions.is_empty() {
            return Err(GeometryError::Empty);
        }
        for (attribute, count) in [("normals", self.normals.len()), ("uvs", self.uvs.len()), ("tangents", self.tangents.len())] {
            if count != 0 && count != self.positions.len() {
                return Err(GeometryError::AttributeMismatch { positions: self.positions.len(), attribute, count });
            }
        }
        if !self.indices.len().is_multiple_of(3) {
            return Err(GeometryError::BadIndexCount(self.indices.len()));
        }
        let vertices = self.positions.len();
        for &index in &self.indices {
            if index as usize >= vertices {
                return Err(GeometryError::IndexOutOfRange { index, vertices });
            }
        }
        Ok(())
    }

    /// Compute per-vertex smooth normals by area-weighted triangle averaging, overwriting any
    /// existing normals. Skips degenerate triangles.
    pub fn compute_smooth_normals(&mut self) {
        let mut normals = vec![Vec3::ZERO; self.positions.len()];
        for triangle in self.indices.as_chunks::<3>().0 {
            let [i0, i1, i2] = *triangle;
            let (Some(&p0), Some(&p1), Some(&p2)) =
                (self.positions.get(i0 as usize), self.positions.get(i1 as usize), self.positions.get(i2 as usize))
            else {
                continue;
            };
            let face = (p1 - p0).cross(p2 - p0);
            for index in [i0, i1, i2] {
                if let Some(normal) = normals.get_mut(index as usize) {
                    *normal += face;
                }
            }
        }
        for normal in &mut normals {
            *normal = normal.normalize_or_zero();
        }
        self.normals = normals;
    }

    /// Generate tangents from positions, normals, UVs and indices. Computes smooth normals first
    /// if none are present.
    pub fn generate_tangents(&mut self) -> Result<(), GeometryError> {
        self.validate()?;
        if self.normals.len() != self.positions.len() {
            self.compute_smooth_normals();
        }
        tangents::generate(self)
    }

    /// Append another mesh, offsetting its indices and submesh runs.
    pub fn append(&mut self, other: &Mesh) {
        let base = self.positions.len() as u32;
        let index_offset = self.indices.len() as u32;

        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.uvs.extend_from_slice(&other.uvs);
        self.tangents.extend_from_slice(&other.tangents);
        self.indices.extend(other.indices.iter().map(|index| index.saturating_add(base)));

        self.submeshes.extend(other.submeshes.iter().map(|submesh| Submesh {
            material: submesh.material,
            first_index: submesh.first_index.saturating_add(index_offset),
            index_count: submesh.index_count,
        }));
    }

    /// Fill in defaults for any missing attributes so the mesh is renderable: smooth normals if
    /// absent and unit UVs/tangents if absent.
    pub fn complete_attributes(&mut self) {
        if self.normals.len() != self.positions.len() {
            self.compute_smooth_normals();
        }
        if self.uvs.len() != self.positions.len() {
            self.uvs = vec![Vec2::ZERO; self.positions.len()];
        }
        if self.tangents.len() != self.positions.len() {
            let _ = self.generate_tangents();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> Mesh {
        Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::new(0.0, 1.0, 0.0)],
            normals: vec![Vec3::Z; 3],
            uvs: vec![Vec2::ZERO, Vec2::X, Vec2::Y],
            tangents: Vec::new(),
            indices: vec![0, 1, 2],
            submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 3 }],
        }
    }

    #[test]
    fn validate_accepts_a_simple_triangle() {
        assert_eq!(triangle().validate(), Ok(()));
    }

    #[test]
    fn validate_rejects_out_of_range_index() {
        let mut mesh = triangle();
        mesh.indices[2] = 9;
        assert!(matches!(mesh.validate(), Err(GeometryError::IndexOutOfRange { index: 9, vertices: 3 })));
    }

    #[test]
    fn validate_rejects_mismatched_attributes() {
        let mut mesh = triangle();
        mesh.normals.pop();
        assert!(matches!(mesh.validate(), Err(GeometryError::AttributeMismatch { .. })));
    }

    #[test]
    fn smooth_normals_point_along_the_face_normal() {
        let mut mesh = triangle();
        mesh.compute_smooth_normals();
        for normal in &mesh.normals {
            assert!((normal.z - 1.0).abs() < 1e-5 || normal.length() == 0.0);
        }
    }

    #[test]
    fn append_offsets_indices_and_submeshes() {
        let mut a = triangle();
        a.append(&triangle());
        assert_eq!(a.indices, vec![0, 1, 2, 3, 4, 5]);
        assert_eq!(a.submeshes.len(), 2);
        assert_eq!(a.submeshes[1].first_index, 3);
    }

    #[test]
    fn bounds_are_tight() {
        let bounds = triangle().bounds();
        assert_eq!(bounds.min, Vec3::ZERO);
        assert_eq!(bounds.max, Vec3::new(1.0, 1.0, 0.0));
    }
}
