//! Additional mesh importers (STL, PLY) built on public parsers.
//!
//! Both formats are geometry-only in the common case: STL carries triangle normals but no UVs, and
//! PLY carries whatever vertex properties the exporter wrote. Missing attributes are completed by
//! the geometry crate (smooth normals, zero UVs, default material).

use std::collections::HashMap;

use substancecraft_doc::{Material, Scene};
use substancecraft_geom::Mesh;
use substancecraft_geom::glam::{Vec2, Vec3};

use crate::IoError;

/// Import an STL file (ASCII or binary): indexed triangles with per-face normals, no UVs.
pub fn load_stl(bytes: &[u8]) -> Result<Scene, IoError> {
    let stl = stl_io::read_stl(&mut std::io::Cursor::new(bytes)).map_err(|err| IoError::Stl(err.to_string()))?;

    let mut mesh = Mesh::new();
    let mut cache: HashMap<[u32; 6], u32> = HashMap::new();

    for face in &stl.faces {
        let mut positions = [Vec3::ZERO; 3];
        let mut complete = true;
        for (slot, &vertex_index) in face.vertices.iter().enumerate() {
            match stl.vertices.get(vertex_index) {
                Some(vertex) => positions[slot] = Vec3::new(vertex[0], vertex[1], vertex[2]),
                None => complete = false,
            }
        }
        if !complete {
            continue;
        }

        // Use the stored normal; fall back to the triangle's winding when it is missing.
        let mut normal = Vec3::new(face.normal[0], face.normal[1], face.normal[2]);
        if normal.length_squared() == 0.0 {
            normal = (positions[1] - positions[0]).cross(positions[2] - positions[0]).normalize_or_zero();
        }

        for position in positions {
            let key = [position.x.to_bits(), position.y.to_bits(), position.z.to_bits(), normal.x.to_bits(), normal.y.to_bits(), normal.z.to_bits()];
            let index = match cache.get(&key) {
                Some(&index) => index,
                None => {
                    let index = mesh.positions.len() as u32;
                    mesh.positions.push(position);
                    mesh.normals.push(normal);
                    mesh.uvs.push(Vec2::ZERO);
                    cache.insert(key, index);
                    index
                }
            };
            mesh.indices.push(index);
        }
    }

    if mesh.positions.is_empty() {
        return Err(IoError::Empty);
    }
    if mesh.normals.iter().all(|normal| normal.length_squared() == 0.0) {
        mesh.compute_smooth_normals();
    }
    mesh.complete_attributes();
    mesh.validate()?;
    Ok(Scene { mesh, materials: vec![Material::default()] })
}

/// Import a PLY file (ASCII or binary). Reads `x/y/z`, optional `nx/ny/nz` and `u/v` (or `s/t`),
/// and triangulates the `vertex_indices` face lists.
pub fn load_ply(bytes: &[u8]) -> Result<Scene, IoError> {
    let parser = ply_rs::parser::Parser::<ply_rs::ply::DefaultElement>::new();
    let mut reader = std::io::Cursor::new(bytes);
    let ply = parser.read_ply(&mut reader).map_err(|err| IoError::Ply(err.to_string()))?;

    let mut mesh = Mesh::new();
    if let Some(vertices) = ply.payload.get("vertex") {
        for vertex in vertices {
            let x = scalar(vertex.get("x")).unwrap_or(0.0) as f32;
            let y = scalar(vertex.get("y")).unwrap_or(0.0) as f32;
            let z = scalar(vertex.get("z")).unwrap_or(0.0) as f32;
            mesh.positions.push(Vec3::new(x, y, z));

            let normal = match (scalar(vertex.get("nx")), scalar(vertex.get("ny")), scalar(vertex.get("nz"))) {
                (Some(nx), Some(ny), Some(nz)) => Vec3::new(nx as f32, ny as f32, nz as f32),
                _ => Vec3::ZERO,
            };
            mesh.normals.push(normal);

            let u = scalar(vertex.get("u")).or_else(|| scalar(vertex.get("s")));
            let v = scalar(vertex.get("v")).or_else(|| scalar(vertex.get("t")));
            let uv = match (u, v) {
                (Some(u), Some(v)) => Vec2::new(u as f32, v as f32),
                _ => Vec2::ZERO,
            };
            mesh.uvs.push(uv);
        }
    }

    if let Some(faces) = ply.payload.get("face") {
        for face in faces {
            let indices = list(face.get("vertex_indices")).or_else(|| list(face.get("vertex_index")));
            let Some(indices) = indices else { continue };
            if indices.len() < 3 {
                continue;
            }
            for corner in 1..indices.len() - 1 {
                let triangle = [indices[0], indices[corner], indices[corner + 1]];
                if triangle.iter().any(|index| *index < 0 || *index as usize >= mesh.positions.len()) {
                    continue;
                }
                for index in triangle {
                    mesh.indices.push(index as u32);
                }
            }
        }
    }

    if mesh.positions.is_empty() {
        return Err(IoError::Empty);
    }
    if mesh.normals.len() != mesh.positions.len() || mesh.normals.iter().all(|normal| normal.length_squared() == 0.0) {
        mesh.compute_smooth_normals();
    }
    mesh.complete_attributes();
    mesh.validate()?;
    Ok(Scene { mesh, materials: vec![Material::default()] })
}

/// Read a numeric PLY scalar property as `f64`.
fn scalar(property: Option<&ply_rs::ply::Property>) -> Option<f64> {
    use ply_rs::ply::Property;
    match property? {
        Property::Char(value) => Some(*value as f64),
        Property::UChar(value) => Some(*value as f64),
        Property::Short(value) => Some(*value as f64),
        Property::UShort(value) => Some(*value as f64),
        Property::Int(value) => Some(*value as f64),
        Property::UInt(value) => Some(*value as f64),
        Property::Float(value) => Some(*value as f64),
        Property::Double(value) => Some(*value),
        _ => None,
    }
}

/// Read a PLY integer list property (e.g. a face's `vertex_indices`) as `i64`.
fn list(property: Option<&ply_rs::ply::Property>) -> Option<Vec<i64>> {
    use ply_rs::ply::Property;
    match property? {
        Property::ListChar(values) => Some(values.iter().map(|value| *value as i64).collect()),
        Property::ListUChar(values) => Some(values.iter().map(|value| *value as i64).collect()),
        Property::ListShort(values) => Some(values.iter().map(|value| *value as i64).collect()),
        Property::ListUShort(values) => Some(values.iter().map(|value| *value as i64).collect()),
        Property::ListInt(values) => Some(values.iter().map(|value| *value as i64).collect()),
        Property::ListUInt(values) => Some(values.iter().map(|value| *value as i64).collect()),
        _ => None,
    }
}

/// A triangle count helper used in tests.
#[cfg(test)]
fn triangles(mesh: &Mesh) -> usize {
    mesh.indices.len() / 3
}

#[cfg(test)]
mod tests {
    use super::*;

    const ASCII_STL: &str = "\
solid tri
  facet normal 0 0 1
    outer loop
      vertex 0 0 0
      vertex 1 0 0
      vertex 0 1 0
    endloop
  endfacet
endsolid tri
";

    const ASCII_PLY: &str = "\
ply
format ascii 1.0
element vertex 3
property float x
property float y
property float z
element face 1
property list uchar int vertex_indices
end_header
0 0 0
1 0 0
0 1 0
3 0 1 2
";

    #[test]
    fn parses_ascii_stl() {
        let scene = load_stl(ASCII_STL.as_bytes()).expect("stl");
        assert_eq!(scene.mesh.vertex_count(), 3);
        assert_eq!(triangles(&scene.mesh), 1);
        assert_eq!(scene.mesh.normals.len(), scene.mesh.positions.len());
        assert!((scene.mesh.normals[0].z - 1.0).abs() < 1e-6);
    }

    #[test]
    fn parses_ascii_ply() {
        let scene = load_ply(ASCII_PLY.as_bytes()).expect("ply");
        assert_eq!(scene.mesh.vertex_count(), 3);
        assert_eq!(triangles(&scene.mesh), 1);
        assert_eq!(scene.mesh.tangents.len(), scene.mesh.vertex_count());
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(load_stl(b"not an stl at all").is_err());
    }
}
