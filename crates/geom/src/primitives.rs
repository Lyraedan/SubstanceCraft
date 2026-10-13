//! Procedural primitive meshes, useful as default content and in tests.

use std::f32::consts::{PI, TAU};

use glam::{Vec2, Vec3};

use crate::GeometryError;
use crate::mesh::{Mesh, Submesh};

/// A UV sphere centred at the origin.
///
/// `rings` is the number of latitude bands (≥ 2) and `sectors` the number of longitude bands (≥ 3).
pub fn uv_sphere(radius: f32, rings: u32, sectors: u32) -> Result<Mesh, GeometryError> {
    if radius <= 0.0 {
        return Err(GeometryError::Parameter("sphere radius must be positive"));
    }
    if rings < 2 {
        return Err(GeometryError::Parameter("sphere needs at least 2 rings"));
    }
    if sectors < 3 {
        return Err(GeometryError::Parameter("sphere needs at least 3 sectors"));
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for ring in 0..=rings {
        let v = ring as f32 / rings as f32;
        let phi = PI * v;
        for sector in 0..=sectors {
            let u = sector as f32 / sectors as f32;
            let theta = TAU * u;
            let normal = Vec3::new(phi.sin() * theta.cos(), phi.cos(), phi.sin() * theta.sin());
            normals.push(normal);
            positions.push(normal * radius);
            uvs.push(Vec2::new(u, v));
        }
    }

    let mut indices = Vec::new();
    let stride = sectors + 1;
    for ring in 0..rings {
        for sector in 0..sectors {
            let a = ring * stride + sector;
            let b = a + stride;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }

    let mut mesh =
        Mesh { positions, normals, uvs, tangents: Vec::new(), indices, submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 0 }] };
    finish(&mut mesh);
    Ok(mesh)
}

/// A torus centred at the origin, lying in the XZ plane.
pub fn torus(major_radius: f32, minor_radius: f32, major_segments: u32, minor_segments: u32) -> Result<Mesh, GeometryError> {
    if major_radius <= 0.0 || minor_radius <= 0.0 {
        return Err(GeometryError::Parameter("torus radii must be positive"));
    }
    if major_segments < 3 || minor_segments < 3 {
        return Err(GeometryError::Parameter("torus needs at least 3 major and minor segments"));
    }

    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();

    for i in 0..=major_segments {
        let u = i as f32 / major_segments as f32;
        let phi = TAU * u;
        for j in 0..=minor_segments {
            let v = j as f32 / minor_segments as f32;
            let theta = TAU * v;
            let normal = Vec3::new(phi.cos() * theta.cos(), theta.sin(), phi.sin() * theta.cos());
            let center = Vec3::new(phi.cos(), 0.0, phi.sin()) * major_radius;
            positions.push(center + normal * minor_radius);
            normals.push(normal);
            uvs.push(Vec2::new(u, v));
        }
    }

    let mut indices = Vec::new();
    let stride = minor_segments + 1;
    for i in 0..major_segments {
        for j in 0..minor_segments {
            let a = i * stride + j;
            let b = a + stride;
            indices.extend_from_slice(&[a, b, a + 1, a + 1, b, b + 1]);
        }
    }

    let mut mesh =
        Mesh { positions, normals, uvs, tangents: Vec::new(), indices, submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 0 }] };
    finish(&mut mesh);
    Ok(mesh)
}

/// Set the single submesh's index count and generate tangents.
fn finish(mesh: &mut Mesh) {
    if let Some(submesh) = mesh.submeshes.first_mut() {
        submesh.index_count = mesh.indices.len() as u32;
    }
    let _ = mesh.generate_tangents();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sphere_is_valid_and_unit_radius() {
        let mesh = uv_sphere(2.0, 16, 24).expect("sphere");
        mesh.validate().expect("valid");
        assert!(mesh.tangents.len() == mesh.vertex_count());
        for position in &mesh.positions {
            assert!((position.length() - 2.0).abs() < 1e-4);
        }
    }

    #[test]
    fn torus_is_valid() {
        let mesh = torus(1.0, 0.25, 24, 12).expect("torus");
        mesh.validate().expect("valid");
        assert_eq!(mesh.tangents.len(), mesh.vertex_count());
    }

    #[test]
    fn bad_parameters_are_errors_not_panics() {
        assert!(uv_sphere(0.0, 8, 8).is_err());
        assert!(uv_sphere(1.0, 1, 8).is_err());
        assert!(torus(1.0, 1.0, 2, 8).is_err());
    }

    #[test]
    fn normals_are_unit() {
        let mesh = uv_sphere(1.0, 12, 16).expect("sphere");
        for normal in &mesh.normals {
            assert!((normal.length() - 1.0).abs() < 1e-4);
        }
    }
}
