//! Mesh connectivity: connected pieces ("parts") used by the parts workflow.
//!
//! A part is a connected component of the mesh. Vertices are welded by object-space position, so a
//! UV seam (which splits vertices on import) does not split a part.

use std::collections::HashMap;

use crate::mesh::Mesh;

/// A tiny union-find over vertex indices.
struct UnionFind {
    parent: Vec<u32>,
}

impl UnionFind {
    fn new(count: usize) -> Self {
        Self { parent: (0..count as u32).collect() }
    }

    fn find(&mut self, mut index: u32) -> u32 {
        while self.parent[index as usize] != index {
            let parent = self.parent[index as usize];
            self.parent[index as usize] = self.parent[parent as usize];
            index = self.parent[index as usize];
        }
        index
    }

    fn union(&mut self, a: u32, b: u32) {
        let (ra, rb) = (self.find(a), self.find(b));
        if ra != rb {
            self.parent[ra as usize] = rb;
        }
    }
}

/// Assign each vertex a connected-piece id (0-based, in first-seen order).
pub fn mesh_parts(mesh: &Mesh) -> Vec<u32> {
    let count = mesh.positions.len();
    if count == 0 {
        return Vec::new();
    }
    let mut union = UnionFind::new(count);

    // Weld vertices sharing a position so UV seams do not split a part.
    let mut weld: HashMap<[i64; 3], u32> = HashMap::new();
    for (index, position) in mesh.positions.iter().enumerate() {
        let key = [quantize(position.x), quantize(position.y), quantize(position.z)];
        match weld.get(&key) {
            Some(&representative) => union.union(index as u32, representative),
            None => {
                weld.insert(key, index as u32);
            }
        }
    }

    // Connect the vertices of each triangle.
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = *triangle;
        if a as usize >= count || b as usize >= count || c as usize >= count {
            continue;
        }
        union.union(a, b);
        union.union(b, c);
    }

    compact(&mut union, count)
}

/// Assign each vertex a UV-island id (connected components in UV space).
pub fn uv_islands(mesh: &Mesh) -> Vec<u32> {
    let count = mesh.positions.len();
    if count == 0 {
        return Vec::new();
    }
    let mut union = UnionFind::new(count);
    // Weld corners sharing a UV coordinate, so triangles meeting at a UV edge join one island.
    let mut weld: HashMap<[i64; 2], u32> = HashMap::new();
    for (index, uv) in mesh.uvs.iter().enumerate() {
        let key = [quantize(uv.x), quantize(uv.y)];
        match weld.get(&key) {
            Some(&representative) => union.union(index as u32, representative),
            None => {
                weld.insert(key, index as u32);
            }
        }
    }
    // A triangle's three corners are one island.
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = *triangle;
        if a as usize >= count || b as usize >= count || c as usize >= count {
            continue;
        }
        union.union(a, b);
        union.union(b, c);
    }
    compact(&mut union, count)
}

/// Assign each vertex the material id of the submesh its triangle belongs to.
pub fn material_parts(mesh: &Mesh) -> Vec<u32> {
    let mut parts = vec![0u32; mesh.positions.len()];
    if mesh.submeshes.is_empty() {
        return parts;
    }
    for submesh in &mesh.submeshes {
        let start = submesh.first_index as usize;
        let end = start.saturating_add(submesh.index_count as usize).min(mesh.indices.len());
        for &vertex in mesh.indices.get(start..end).unwrap_or(&[]) {
            if let Some(slot) = parts.get_mut(vertex as usize) {
                *slot = submesh.material;
            }
        }
    }
    parts
}

/// Build a mesh containing only the triangles whose part passes `keep`, rebuilding submeshes by
/// material. Vertices are copied lazily and shared by the kept triangles.
pub fn filter_parts(mesh: &Mesh, vertex_parts: &[u32], keep: impl Fn(u32) -> bool) -> Mesh {
    let mut out = Mesh::new();
    let mut remap: Vec<Option<u32>> = vec![None; mesh.positions.len()];
    let ranges: Vec<(u32, u32, u32)> = if mesh.submeshes.is_empty() {
        vec![(0, 0, mesh.indices.len() as u32)]
    } else {
        mesh.submeshes.iter().map(|submesh| (submesh.material, submesh.first_index, submesh.index_count)).collect()
    };

    for (material, first, count) in ranges {
        let start = first as usize;
        let end = start.saturating_add(count as usize).min(mesh.indices.len());
        let submesh_start = out.indices.len() as u32;
        for triangle in mesh.indices.get(start..end).unwrap_or(&[]).as_chunks::<3>().0 {
            let [a, b, c] = *triangle;
            let part = vertex_parts.get(a as usize).copied().unwrap_or(u32::MAX);
            if !keep(part) {
                continue;
            }
            let na = push_vertex(mesh, &mut out, &mut remap, a);
            let nb = push_vertex(mesh, &mut out, &mut remap, b);
            let nc = push_vertex(mesh, &mut out, &mut remap, c);
            out.indices.extend_from_slice(&[na, nb, nc]);
        }
        let index_count = out.indices.len() as u32 - submesh_start;
        if index_count > 0 {
            out.submeshes.push(crate::mesh::Submesh { material, first_index: submesh_start, index_count });
        }
    }
    out
}

/// Copy one vertex into `out` (once) and return its new index.
fn push_vertex(mesh: &Mesh, out: &mut Mesh, remap: &mut [Option<u32>], index: u32) -> u32 {
    if let Some(existing) = remap.get(index as usize).copied().flatten() {
        return existing;
    }
    let new_index = out.positions.len() as u32;
    if let Some(position) = mesh.positions.get(index as usize) {
        out.positions.push(*position);
    }
    if let Some(normal) = mesh.normals.get(index as usize) {
        out.normals.push(*normal);
    }
    if let Some(uv) = mesh.uvs.get(index as usize) {
        out.uvs.push(*uv);
    }
    if let Some(tangent) = mesh.tangents.get(index as usize) {
        out.tangents.push(*tangent);
    }
    if let Some(slot) = remap.get_mut(index as usize) {
        *slot = Some(new_index);
    }
    new_index
}

/// Renumber union-find roots into dense ids in first-seen order.
fn compact(union: &mut UnionFind, count: usize) -> Vec<u32> {
    let mut remap: HashMap<u32, u32> = HashMap::new();
    let mut parts = Vec::with_capacity(count);
    for index in 0..count as u32 {
        let root = union.find(index);
        let next = remap.len() as u32;
        parts.push(*remap.entry(root).or_insert(next));
    }
    parts
}

fn quantize(value: f32) -> i64 {
    (value as f64 * 10_000.0).round() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::glam::{Vec2, Vec3};

    #[test]
    fn two_disjoint_triangles_are_two_parts() {
        let mesh = Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(5.0, 0.0, 0.0), Vec3::new(6.0, 0.0, 0.0), Vec3::new(5.0, 1.0, 0.0)],
            normals: vec![Vec3::Z; 6],
            uvs: vec![Vec2::ZERO; 6],
            tangents: vec![Default::default(); 6],
            indices: vec![0, 1, 2, 3, 4, 5],
            submeshes: Vec::new(),
        };
        let parts = mesh_parts(&mesh);
        assert_eq!(parts.len(), 6);
        assert_eq!(parts[0], parts[1]);
        assert_eq!(parts[0], parts[2]);
        assert_ne!(parts[0], parts[3]);
    }

    #[test]
    fn a_uv_seam_does_not_split_a_part() {
        // A quad split into two triangles that do not share vertex indices at the seam.
        let mesh = Mesh {
            positions: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                // Duplicated seam vertices at different indices, same positions.
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z; 6],
            uvs: vec![Vec2::ZERO; 6],
            tangents: vec![Default::default(); 6],
            indices: vec![0, 1, 2, 3, 4, 5],
            submeshes: Vec::new(),
        };
        let parts = mesh_parts(&mesh);
        assert_eq!(parts.iter().copied().collect::<std::collections::BTreeSet<_>>().len(), 1);
    }

    fn two_triangles() -> Mesh {
        Mesh {
            positions: vec![Vec3::ZERO, Vec3::X, Vec3::Y, Vec3::new(5.0, 0.0, 0.0), Vec3::new(6.0, 0.0, 0.0), Vec3::new(5.0, 1.0, 0.0)],
            normals: vec![Vec3::Z; 6],
            uvs: vec![Vec2::ZERO, Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.5), Vec2::new(0.5, 0.5), Vec2::new(1.0, 0.5), Vec2::new(0.5, 1.0)],
            tangents: vec![Default::default(); 6],
            indices: vec![0, 1, 2, 3, 4, 5],
            submeshes: vec![
                crate::mesh::Submesh { material: 0, first_index: 0, index_count: 3 },
                crate::mesh::Submesh { material: 1, first_index: 3, index_count: 3 },
            ],
        }
    }

    #[test]
    fn material_parts_use_the_submesh_material() {
        let parts = material_parts(&two_triangles());
        assert_eq!(&parts[0..3], &[0, 0, 0]);
        assert_eq!(&parts[3..6], &[1, 1, 1]);
    }

    #[test]
    fn disjoint_uv_islands_are_separate() {
        let islands = uv_islands(&two_triangles());
        assert_ne!(islands[0], islands[3]);
        assert_eq!(islands[0], islands[1]);
    }

    #[test]
    fn filter_parts_keeps_only_the_selected_pieces() {
        let mesh = two_triangles();
        let parts = mesh_parts(&mesh);
        let filtered = filter_parts(&mesh, &parts, |part| part == 0);
        assert_eq!(filtered.indices.len(), 3);
        assert_eq!(filtered.submeshes.len(), 1);
        assert_eq!(filtered.submeshes[0].material, 0);
        assert_eq!(filtered.positions.len(), 3);
    }
}
