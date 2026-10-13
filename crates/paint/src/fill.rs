//! UV-island polyfill: flood the connected triangles under a cursor and rasterise them.

use std::collections::HashMap;

use substancecraft_geom::Mesh;
use substancecraft_geom::glam::Vec2;
use substancecraft_raster::RasterImage;

/// Fill the UV island containing `start_triangle` with `color`. Returns false for an out-of-range
/// triangle or an empty target.
///
/// Triangles are connected when they share an edge with the same vertex indices; because the
/// importer splits vertices at UV seams, this keeps distinct UV islands separate.
pub fn polyfill(mesh: &Mesh, target: &mut RasterImage, start_triangle: u32, color: [f32; 4]) -> bool {
    let triangles = mesh.indices.as_chunks::<3>().0;
    let start = start_triangle as usize;
    if start >= triangles.len() || target.is_empty() {
        return false;
    }

    let mut edges: HashMap<(u32, u32), Vec<usize>> = HashMap::new();
    for (index, triangle) in triangles.iter().enumerate() {
        let [a, b, c] = *triangle;
        for (from, to) in [(a, b), (b, c), (c, a)] {
            edges.entry(edge_key(from, to)).or_default().push(index);
        }
    }

    let mut visited = vec![false; triangles.len()];
    let mut stack = vec![start];
    visited[start] = true;
    let mut component = Vec::new();
    while let Some(index) = stack.pop() {
        component.push(index);
        let [a, b, c] = triangles[index];
        for (from, to) in [(a, b), (b, c), (c, a)] {
            if let Some(neighbours) = edges.get(&edge_key(from, to)) {
                for &neighbour in neighbours {
                    if !visited[neighbour] {
                        visited[neighbour] = true;
                        stack.push(neighbour);
                    }
                }
            }
        }
    }

    for index in component {
        let [i0, i1, i2] = triangles[index];
        let (Some(&uv0), Some(&uv1), Some(&uv2)) = (mesh.uvs.get(i0 as usize), mesh.uvs.get(i1 as usize), mesh.uvs.get(i2 as usize)) else {
            continue;
        };
        rasterize_triangle(target, uv0, uv1, uv2, color);
    }
    true
}

fn edge_key(a: u32, b: u32) -> (u32, u32) {
    if a < b { (a, b) } else { (b, a) }
}

/// Rasterise a UV-space triangle into the image.
fn rasterize_triangle(target: &mut RasterImage, uv0: Vec2, uv1: Vec2, uv2: Vec2, color: [f32; 4]) {
    let width = target.width as f32;
    let height = target.height as f32;
    let (ax, ay) = (uv0.x * width, uv0.y * height);
    let (bx, by) = (uv1.x * width, uv1.y * height);
    let (cx, cy) = (uv2.x * width, uv2.y * height);

    let area = edge(ax, ay, bx, by, cx, cy);
    if area.abs() < 1e-6 {
        return;
    }

    let min_x = ax.min(bx).min(cx).floor().max(0.0) as u32;
    let max_x = (ax.max(bx).max(cx).ceil() as i64).clamp(0, target.width as i64 - 1) as u32;
    let min_y = ay.min(by).min(cy).floor().max(0.0) as u32;
    let max_y = (ay.max(by).max(cy).ceil() as i64).clamp(0, target.height as i64 - 1) as u32;

    let epsilon = -1e-4;
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let px = x as f32 + 0.5;
            let py = y as f32 + 0.5;
            let w0 = edge(bx, by, cx, cy, px, py) / area;
            let w1 = edge(cx, cy, ax, ay, px, py) / area;
            let w2 = edge(ax, ay, bx, by, px, py) / area;
            if w0 >= epsilon && w1 >= epsilon && w2 >= epsilon {
                target.set_pixel(x, y, color);
            }
        }
    }
}

fn edge(ax: f32, ay: f32, bx: f32, by: f32, px: f32, py: f32) -> f32 {
    (bx - ax) * (py - ay) - (by - ay) * (px - ax)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_geom::glam::Vec3;
    use substancecraft_geom::mesh::Submesh;

    /// A quad island in UV `0..0.5` (two triangles sharing an edge) and a separate triangle island
    /// in UV `0.6..0.9`.
    fn mesh() -> Mesh {
        Mesh {
            positions: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(1.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(0.5, 1.1, 0.0),
                Vec3::new(0.0, 1.4, 0.0),
            ],
            normals: vec![Vec3::Z; 7],
            uvs: vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(0.5, 0.0),
                Vec2::new(0.5, 0.5),
                Vec2::new(0.0, 0.5),
                Vec2::new(0.6, 0.1),
                Vec2::new(0.9, 0.1),
                Vec2::new(0.6, 0.4),
            ],
            tangents: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3, 4, 5, 6],
            submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 9 }],
        }
    }

    #[test]
    fn fills_only_the_connected_island() {
        let mut image = RasterImage::new(64, 64);
        assert!(polyfill(&mesh(), &mut image, 0, [1.0, 0.0, 0.0, 1.0]));
        // Inside the quad island (uv ~0.25, 0.25).
        assert!(image.pixel(16, 16)[0] > 0.9, "quad island not filled");
        // The isolated island's region should be untouched (uv ~0.75, 0.15).
        assert!(image.pixel(48, 10)[3] < 0.1, "polyfill bled into the other island");
    }

    #[test]
    fn fills_the_isolated_island() {
        let mut image = RasterImage::new(64, 64);
        assert!(polyfill(&mesh(), &mut image, 2, [0.0, 1.0, 0.0, 1.0]));
        assert!(image.pixel(41, 10)[1] > 0.9, "isolated island not filled");
        assert!(image.pixel(16, 16)[3] < 0.1);
    }

    #[test]
    fn out_of_range_triangle_is_rejected() {
        let mut image = RasterImage::new(8, 8);
        assert!(!polyfill(&mesh(), &mut image, 99, [1.0; 4]));
    }
}
