//! A bounding-volume hierarchy for fast ray queries.
//!
//! Built once per mesh with a median split on the largest axis; traversal uses an explicit stack
//! (no recursion) so deep or degenerate meshes cannot overflow the stack.

use glam::Vec3;

use crate::bounds::Bounds;
use crate::mesh::Mesh;
use crate::raycast::{Hit, Ray, ray_triangle};

/// The maximum build recursion depth; deeper ranges become leaves.
const MAX_DEPTH: u32 = 32;
/// The largest leaf size before splitting.
const MAX_LEAF: usize = 4;

#[derive(Clone, Copy, Debug)]
struct Node {
    min: Vec3,
    max: Vec3,
    /// For a leaf: the first index in `order`. For an internal node: the left child index.
    first: u32,
    /// The number of triangles in a leaf (0 for an internal node).
    count: u32,
    /// The right child index (internal nodes only).
    right: u32,
}

/// A binary BVH over a mesh's triangles.
#[derive(Clone, Debug)]
pub struct Bvh {
    nodes: Vec<Node>,
    order: Vec<u32>,
    bounds: Bounds,
}

impl Bvh {
    /// Build a BVH for a mesh. An empty mesh yields an empty tree.
    pub fn build(mesh: &Mesh) -> Self {
        let triangle_count = mesh.indices.len() / 3;
        let mut order: Vec<u32> = (0..triangle_count as u32).collect();
        let mut nodes: Vec<Node> = Vec::with_capacity(triangle_count.max(1) * 2);
        let bounds = if triangle_count == 0 {
            Bounds::EMPTY
        } else {
            let root = build_range(&mut nodes, &mut order, mesh, 0, triangle_count, 0);
            let _ = root;
            triangle_bounds(mesh, &order)
        };
        Self { nodes, order, bounds }
    }

    /// The bounds of the whole tree.
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// The number of nodes (for diagnostics/tests).
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Whether the tree is empty.
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The nearest hit along a ray, or `None`. Hits both faces.
    pub fn intersect(&self, mesh: &Mesh, ray: &Ray) -> Option<Hit> {
        if self.nodes.is_empty() {
            return None;
        }
        let inverse = Vec3::new(safe_inverse(ray.direction.x), safe_inverse(ray.direction.y), safe_inverse(ray.direction.z));
        let mut best: Option<Hit> = None;
        let mut best_distance = f32::INFINITY;
        let mut stack: Vec<u32> = Vec::with_capacity(64);
        stack.push(0);

        while let Some(node_index) = stack.pop() {
            let Some(node) = self.nodes.get(node_index as usize) else { continue };
            if !aabb_hit(node.min, node.max, ray, inverse, best_distance) {
                continue;
            }
            if node.count > 0 {
                for &triangle in self.order.get(node.first as usize..(node.first + node.count) as usize).unwrap_or(&[]) {
                    if let Some(hit) = intersect_triangle(mesh, ray, triangle)
                        && hit.distance < best_distance
                    {
                        best_distance = hit.distance;
                        best = Some(hit);
                    }
                }
            } else {
                stack.push(node.first);
                stack.push(node.right);
            }
        }
        best
    }

    /// Whether any geometry is hit along a ray within `max_distance` (shadow/occlusion test).
    pub fn occluded(&self, mesh: &Mesh, ray: &Ray, max_distance: f32) -> bool {
        self.intersect(mesh, ray).is_some_and(|hit| hit.distance <= max_distance)
    }
}

fn build_range(nodes: &mut Vec<Node>, order: &mut [u32], mesh: &Mesh, start: usize, end: usize, depth: u32) -> u32 {
    let index = nodes.len() as u32;
    let bounds = range_bounds(mesh, order, start, end).unwrap_or(Bounds::EMPTY);
    nodes.push(Node { min: bounds.min, max: bounds.max, first: 0, count: 0, right: 0 });

    let count = end.saturating_sub(start);
    if count <= MAX_LEAF || depth >= MAX_DEPTH {
        nodes[index as usize].first = start as u32;
        nodes[index as usize].count = count as u32;
        return index;
    }

    let extent = bounds.size();
    let axis = if extent.x >= extent.y && extent.x >= extent.z {
        0
    } else if extent.y >= extent.z {
        1
    } else {
        2
    };
    order[start..end].sort_by(|a, b| {
        let ca = triangle_centroid(mesh, *a)[axis];
        let cb = triangle_centroid(mesh, *b)[axis];
        ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
    });
    let mid = start + count / 2;
    let left = build_range(nodes, order, mesh, start, mid, depth + 1);
    let right = build_range(nodes, order, mesh, mid, end, depth + 1);
    nodes[index as usize].first = left;
    nodes[index as usize].right = right;
    nodes[index as usize].count = 0;
    index
}

fn range_bounds(mesh: &Mesh, order: &[u32], start: usize, end: usize) -> Option<Bounds> {
    let mut bounds = Bounds::EMPTY;
    for &triangle in order.get(start..end)? {
        for index in triangle_indices(mesh, triangle) {
            if let Some(&position) = mesh.positions.get(index as usize) {
                bounds = bounds.include_point(position);
            }
        }
    }
    (!bounds.is_empty()).then_some(bounds)
}

fn triangle_bounds(mesh: &Mesh, order: &[u32]) -> Bounds {
    range_bounds(mesh, order, 0, order.len()).unwrap_or(Bounds::EMPTY)
}

fn triangle_indices(mesh: &Mesh, triangle: u32) -> [u32; 3] {
    let start = triangle as usize * 3;
    let slice = mesh.indices.get(start..start + 3);
    match slice {
        Some([a, b, c]) => [*a, *b, *c],
        _ => [0, 0, 0],
    }
}

fn triangle_centroid(mesh: &Mesh, triangle: u32) -> Vec3 {
    let [a, b, c] = triangle_indices(mesh, triangle);
    let pa = mesh.positions.get(a as usize).copied().unwrap_or(Vec3::ZERO);
    let pb = mesh.positions.get(b as usize).copied().unwrap_or(Vec3::ZERO);
    let pc = mesh.positions.get(c as usize).copied().unwrap_or(Vec3::ZERO);
    (pa + pb + pc) / 3.0
}

fn intersect_triangle(mesh: &Mesh, ray: &Ray, triangle: u32) -> Option<Hit> {
    let [i0, i1, i2] = triangle_indices(mesh, triangle);
    let a = mesh.positions.get(i0 as usize).copied()?;
    let b = mesh.positions.get(i1 as usize).copied()?;
    let c = mesh.positions.get(i2 as usize).copied()?;
    let (t, u, v) = ray_triangle(ray, a, b, c)?;
    let w = 1.0 - u - v;
    let normal = (mesh.normals.get(i0 as usize).copied().unwrap_or(Vec3::ZERO) * w
        + mesh.normals.get(i1 as usize).copied().unwrap_or(Vec3::ZERO) * u
        + mesh.normals.get(i2 as usize).copied().unwrap_or(Vec3::ZERO) * v)
        .normalize_or_zero();
    let uv = mesh.uvs.get(i0 as usize).copied().unwrap_or_default() * w
        + mesh.uvs.get(i1 as usize).copied().unwrap_or_default() * u
        + mesh.uvs.get(i2 as usize).copied().unwrap_or_default() * v;
    Some(Hit { distance: t, position: ray.at(t), normal, uv, triangle })
}

fn aabb_hit(min: Vec3, max: Vec3, ray: &Ray, inverse: Vec3, max_distance: f32) -> bool {
    let t0 = (min - ray.origin) * inverse;
    let t1 = (max - ray.origin) * inverse;
    let tmin = t0.min(t1).max_element().max(0.0);
    let tmax = t0.max(t1).min_element();
    tmax >= tmin && tmin <= max_distance
}

fn safe_inverse(value: f32) -> f32 {
    if value.abs() < 1e-9 { 1e9 } else { 1.0 / value }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mesh::Submesh;
    use glam::Vec2;

    fn grid() -> Mesh {
        // A 4x4 grid of quads in the z=0 plane.
        let mut positions = Vec::new();
        let mut normals = Vec::new();
        let mut uvs = Vec::new();
        let mut indices = Vec::new();
        let n = 4;
        for y in 0..=n {
            for x in 0..=n {
                positions.push(Vec3::new(x as f32, y as f32, 0.0));
                normals.push(Vec3::Z);
                uvs.push(Vec2::new(x as f32 / n as f32, y as f32 / n as f32));
            }
        }
        let stride = n + 1;
        for y in 0..n {
            for x in 0..n {
                let a = y * stride + x;
                indices.extend_from_slice(&[a, a + 1, a + stride + 1, a, a + stride + 1, a + stride]);
            }
        }
        let index_count = indices.len() as u32;
        Mesh { positions, normals, uvs, tangents: Vec::new(), indices, submeshes: vec![Submesh { material: 0, first_index: 0, index_count }] }
    }

    #[test]
    fn bvh_matches_brute_force() {
        let mesh = grid();
        let bvh = Bvh::build(&mesh);
        for (ox, oy) in [(0.5, 0.5), (2.3, 1.7), (3.9, 3.9), (-1.0, 2.0)] {
            let ray = Ray { origin: Vec3::new(ox, oy, 5.0), direction: -Vec3::Z };
            let brute = crate::raycast::raycast(&mesh, &ray);
            let fast = bvh.intersect(&mesh, &ray);
            match (brute, fast) {
                (Some(b), Some(f)) => assert!((b.distance - f.distance).abs() < 1e-4),
                (None, None) => {}
                other => panic!("bvh/brute mismatch: {other:?}"),
            }
        }
    }

    #[test]
    fn empty_mesh_is_safe() {
        let bvh = Bvh::build(&Mesh::default());
        let ray = Ray { origin: Vec3::Z, direction: -Vec3::Z };
        assert!(bvh.intersect(&Mesh::default(), &ray).is_none());
        assert!(bvh.is_empty());
    }

    #[test]
    fn occlusion_respects_max_distance() {
        let mesh = grid();
        let bvh = Bvh::build(&mesh);
        let ray = Ray { origin: Vec3::new(1.0, 1.0, 5.0), direction: -Vec3::Z };
        assert!(bvh.occluded(&mesh, &ray, 10.0));
        assert!(!bvh.occluded(&mesh, &ray, 1.0));
    }
}
