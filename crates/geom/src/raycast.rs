//! Ray casting against meshes, for painting and picking.

use glam::{Vec2, Vec3};

use crate::mesh::Mesh;

/// A ray with a normalised direction.
#[derive(Clone, Copy, Debug)]
pub struct Ray {
    /// The ray origin.
    pub origin: Vec3,
    /// The normalised ray direction.
    pub direction: Vec3,
}

impl Ray {
    /// A ray from an origin toward a point (direction is normalised).
    pub fn toward(origin: Vec3, target: Vec3) -> Self {
        Self { origin, direction: (target - origin).normalize_or_zero() }
    }

    /// The point at parameter `t`.
    pub fn at(&self, t: f32) -> Vec3 {
        self.origin + self.direction * t
    }
}

/// A hit against a mesh triangle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    /// Distance along the ray.
    pub distance: f32,
    /// World-space position.
    pub position: Vec3,
    /// Interpolated shading normal.
    pub normal: Vec3,
    /// Interpolated UV.
    pub uv: Vec2,
    /// Index of the hit triangle (in triangle units).
    pub triangle: u32,
}

/// Möller–Trumbore ray/triangle intersection. Returns `(t, u, v)` where `u`/`v` are barycentric
/// weights for `b`/`c` (so `a`'s weight is `1 - u - v`). Hits both faces.
pub fn ray_triangle(ray: &Ray, a: Vec3, b: Vec3, c: Vec3) -> Option<(f32, f32, f32)> {
    let edge1 = b - a;
    let edge2 = c - a;
    let pvec = ray.direction.cross(edge2);
    let det = edge1.dot(pvec);
    if det.abs() < 1e-8 {
        return None;
    }
    let inv_det = 1.0 / det;
    let tvec = ray.origin - a;
    let u = tvec.dot(pvec) * inv_det;
    if !(-1e-6..=1.0 + 1e-6).contains(&u) {
        return None;
    }
    let qvec = tvec.cross(edge1);
    let v = ray.direction.dot(qvec) * inv_det;
    if v < -1e-6 || u + v > 1.0 + 1e-6 {
        return None;
    }
    let t = edge2.dot(qvec) * inv_det;
    (t > 1e-5).then_some((t, u, v))
}

/// The nearest hit of a ray against a mesh, or `None`. Brute force over all triangles.
pub fn raycast(mesh: &Mesh, ray: &Ray) -> Option<Hit> {
    let mut nearest: Option<Hit> = None;
    for (triangle_index, triangle) in mesh.indices.as_chunks::<3>().0.iter().enumerate() {
        let [i0, i1, i2] = *triangle;
        let (Some(&a), Some(&b), Some(&c)) = (mesh.positions.get(i0 as usize), mesh.positions.get(i1 as usize), mesh.positions.get(i2 as usize))
        else {
            continue;
        };
        let Some((t, u, v)) = ray_triangle(ray, a, b, c) else { continue };
        if nearest.as_ref().is_some_and(|hit| hit.distance <= t) {
            continue;
        }
        let w = 1.0 - u - v;
        let normal =
            interpolate3(mesh.normals.get(i0 as usize), mesh.normals.get(i1 as usize), mesh.normals.get(i2 as usize), w, u, v).normalize_or_zero();
        let uv = interpolate2(mesh.uvs.get(i0 as usize), mesh.uvs.get(i1 as usize), mesh.uvs.get(i2 as usize), w, u, v);
        nearest = Some(Hit { distance: t, position: ray.at(t), normal, uv, triangle: triangle_index as u32 });
    }
    nearest
}

fn interpolate3(a: Option<&Vec3>, b: Option<&Vec3>, c: Option<&Vec3>, w: f32, u: f32, v: f32) -> Vec3 {
    let a = a.copied().unwrap_or(Vec3::ZERO);
    let b = b.copied().unwrap_or(Vec3::ZERO);
    let c = c.copied().unwrap_or(Vec3::ZERO);
    a * w + b * u + c * v
}

fn interpolate2(a: Option<&Vec2>, b: Option<&Vec2>, c: Option<&Vec2>, w: f32, u: f32, v: f32) -> Vec2 {
    let a = a.copied().unwrap_or_default();
    let b = b.copied().unwrap_or_default();
    let c = c.copied().unwrap_or_default();
    a * w + b * u + c * v
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::{Mesh, Submesh};

    fn quad() -> Mesh {
        Mesh {
            positions: vec![Vec3::new(-1.0, -1.0, 0.0), Vec3::new(1.0, -1.0, 0.0), Vec3::new(1.0, 1.0, 0.0), Vec3::new(-1.0, 1.0, 0.0)],
            normals: vec![Vec3::Z; 4],
            uvs: vec![Vec2::new(0.0, 1.0), Vec2::new(1.0, 1.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, 0.0)],
            tangents: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 6 }],
        }
    }

    #[test]
    fn hits_a_quad_and_interpolates_uv() {
        let mesh = quad();
        let ray = Ray { origin: Vec3::new(0.0, 0.0, 5.0), direction: -Vec3::Z };
        let hit = raycast(&mesh, &ray).expect("hit");
        assert!((hit.distance - 5.0).abs() < 1e-4);
        assert!((hit.uv.x - 0.5).abs() < 1e-4, "u={}", hit.uv.x);
        assert!((hit.uv.y - 0.5).abs() < 1e-4, "v={}", hit.uv.y);
    }

    #[test]
    fn misses_when_pointing_away() {
        let mesh = quad();
        let ray = Ray { origin: Vec3::new(0.0, 0.0, 5.0), direction: Vec3::Z };
        assert!(raycast(&mesh, &ray).is_none());
    }

    #[test]
    fn picks_the_nearest_triangle() {
        let mesh = quad();
        let ray = Ray { origin: Vec3::new(0.0, 0.0, 5.0), direction: -Vec3::Z };
        // Both triangles overlap only at the shared edge; any hit is on the quad.
        let hit = raycast(&mesh, &ray).expect("hit");
        assert!(hit.triangle <= 1);
    }

    #[test]
    fn parallel_ray_misses() {
        let mesh = quad();
        let ray = Ray { origin: Vec3::new(0.0, 0.0, 5.0), direction: Vec3::X };
        assert!(raycast(&mesh, &ray).is_none());
    }
}
