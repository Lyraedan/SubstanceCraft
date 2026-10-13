//! UV-derived tangent generation (per-triangle accumulation with Gram-Schmidt orthogonalisation).
//!
//! This is the standard Lengyel method. It is deterministic and dependency-free; a MikkTSpace
//! conformance pass can replace it later without changing the API.

use glam::{Vec3, Vec4};

use crate::GeometryError;
use crate::mesh::Mesh;

/// Fill `mesh.tangents` from positions, normals, UVs and indices. Normals must already be present
/// and unit length; callers use [`crate::Mesh::generate_tangents`], which arranges this.
pub fn generate(mesh: &mut Mesh) -> Result<(), GeometryError> {
    let count = mesh.positions.len();
    let mut tan1 = vec![Vec3::ZERO; count];
    let mut tan2 = vec![Vec3::ZERO; count];

    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [i0, i1, i2] = *triangle;
        let (i0, i1, i2) = (i0 as usize, i1 as usize, i2 as usize);
        let (Some(&p0), Some(&p1), Some(&p2)) = (mesh.positions.get(i0), mesh.positions.get(i1), mesh.positions.get(i2)) else {
            continue;
        };
        let (Some(&uv0), Some(&uv1), Some(&uv2)) = (mesh.uvs.get(i0), mesh.uvs.get(i1), mesh.uvs.get(i2)) else {
            continue;
        };

        let edge1 = p1 - p0;
        let edge2 = p2 - p0;
        let delta1 = uv1 - uv0;
        let delta2 = uv2 - uv0;

        let determinant = delta1.x * delta2.y - delta2.x * delta1.y;
        if determinant.abs() < 1e-12 {
            continue;
        }
        let r = 1.0 / determinant;
        let sdir = (edge1 * delta2.y - edge2 * delta1.y) * r;
        let tdir = (edge2 * delta1.x - edge1 * delta2.x) * r;

        for index in [i0, i1, i2] {
            if let (Some(t1), Some(t2)) = (tan1.get_mut(index), tan2.get_mut(index)) {
                *t1 += sdir;
                *t2 += tdir;
            }
        }
    }

    let mut result = vec![Vec4::new(1.0, 0.0, 0.0, 1.0); count];
    for index in 0..count {
        let normal = mesh.normals.get(index).copied().unwrap_or(Vec3::Z);
        let tangent = tan1.get(index).copied().unwrap_or(Vec3::X);
        let bitangent = tan2.get(index).copied().unwrap_or(Vec3::Z);

        let orthogonal = tangent - normal * normal.dot(tangent);
        let normalized = orthogonal.normalize_or_zero();
        let unit = if normalized.length_squared() > 0.0 { normalized } else { fallback_tangent(normal) };

        let handedness = if normal.cross(unit).dot(bitangent) < 0.0 { -1.0 } else { 1.0 };
        if let Some(slot) = result.get_mut(index) {
            *slot = Vec4::new(unit.x, unit.y, unit.z, handedness);
        }
    }

    mesh.tangents = result;
    Ok(())
}

/// A tangent perpendicular to `normal`, for vertices with no usable UV contribution.
fn fallback_tangent(normal: Vec3) -> Vec3 {
    let reference = if normal.z.abs() < 0.9 { Vec3::Z } else { Vec3::Y };
    let tangent = reference.cross(normal);
    tangent.normalize_or_zero()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{Mesh, Submesh};
    use glam::Vec2;

    #[test]
    fn planar_quad_tangent_is_orthonormal_and_u_aligned() {
        let mut mesh = Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::new(1.0, 1.0, 0.0), Vec3::Y],
            normals: vec![Vec3::Z; 4],
            uvs: vec![Vec2::ZERO, Vec2::X, Vec2::ONE, Vec2::Y],
            tangents: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 6 }],
        };
        mesh.generate_tangents().expect("tangents");
        for tangent in &mesh.tangents {
            let t = tangent.truncate();
            assert!((t.length() - 1.0).abs() < 1e-4, "tangent not unit: {t:?}");
            assert!(t.dot(Vec3::Z).abs() < 1e-4, "tangent not perpendicular to normal");
            assert!((t.x - 1.0).abs() < 1e-4, "tangent should follow +U: {t:?}");
        }
    }

    #[test]
    fn degenerate_uv_triangles_do_not_panic() {
        let mut mesh = Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y],
            normals: vec![Vec3::Z; 3],
            uvs: vec![Vec2::ZERO; 3],
            tangents: Vec::new(),
            indices: vec![0, 1, 2],
            submeshes: Vec::new(),
        };
        mesh.generate_tangents().expect("no panic");
        assert_eq!(mesh.tangents.len(), 3);
        for tangent in &mesh.tangents {
            assert!(tangent.truncate().length() >= 0.0);
        }
    }
}
