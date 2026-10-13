//! Mesh import: OBJ and glTF 2.0 into a [`Scene`].

use std::collections::HashMap;
use std::path::Path;

use substancecraft_doc::{Material, Scene};
use substancecraft_geom::glam::{Vec2, Vec3};
use substancecraft_geom::{Mesh, Submesh};

use crate::IoError;

/// Load a mesh from a path, choosing the parser by extension (`.obj`, `.gltf`, `.glb`).
pub fn load_scene(path: impl AsRef<Path>) -> Result<Scene, IoError> {
    let path = path.as_ref();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default().to_ascii_lowercase();
    match extension.as_str() {
        "obj" => {
            let bytes = std::fs::read(path)?;
            load_obj(&bytes)
        }
        "gltf" | "glb" => load_gltf(path),
        other => Err(IoError::UnsupportedFormat(other.to_string())),
    }
}

/// Parse a Wavefront OBJ file from bytes into a scene.
pub fn load_obj(bytes: &[u8]) -> Result<Scene, IoError> {
    let data = obj::ObjData::load_buf(std::io::Cursor::new(bytes)).map_err(|err| IoError::Obj(err.to_string()))?;

    let mut mesh = Mesh::new();
    let mut materials: Vec<Material> = vec![Material::default()];
    let mut material_index: HashMap<String, u32> = HashMap::new();
    let mut vertices: HashMap<(usize, Option<usize>, Option<usize>), u32> = HashMap::new();
    let mut complete_normals = !data.normal.is_empty();

    for object in &data.objects {
        for group in &object.groups {
            let material = material_id(&group.material, &mut materials, &mut material_index);
            let first_index = mesh.indices.len() as u32;

            for polygon in &group.polys {
                let indices = &polygon.0;
                if indices.len() < 3 {
                    continue;
                }
                for corner in 1..indices.len().saturating_sub(1) {
                    let (Some(a), Some(b), Some(c)) = (indices.first(), indices.get(corner), indices.get(corner + 1)) else {
                        continue;
                    };
                    for tuple in [a, b, c] {
                        let key = (tuple.0, tuple.1, tuple.2);
                        let index = match vertices.get(&key) {
                            Some(&index) => index,
                            None => {
                                let position = data
                                    .position
                                    .get(tuple.0)
                                    .copied()
                                    .ok_or_else(|| IoError::Obj(format!("position index {} out of range", tuple.0)))?;
                                let normal = tuple.2.and_then(|n| data.normal.get(n).copied());
                                if normal.is_none() {
                                    complete_normals = false;
                                }
                                let uv = tuple.1.and_then(|t| data.texture.get(t).copied());
                                let index = mesh.positions.len() as u32;
                                mesh.positions.push(Vec3::from(position));
                                mesh.normals.push(normal.map(Vec3::from).unwrap_or(Vec3::ZERO));
                                mesh.uvs.push(uv.map(Vec2::from).unwrap_or(Vec2::ZERO));
                                vertices.insert(key, index);
                                index
                            }
                        };
                        mesh.indices.push(index);
                    }
                }
            }

            let index_count = mesh.indices.len() as u32 - first_index;
            if index_count > 0 {
                mesh.submeshes.push(Submesh { material, first_index, index_count });
            }
        }
    }

    if mesh.positions.is_empty() {
        return Err(IoError::Empty);
    }
    if !complete_normals {
        mesh.compute_smooth_normals();
    }
    mesh.complete_attributes();
    mesh.validate()?;

    Ok(Scene { mesh, materials })
}

/// Import a glTF 2.0 (`.gltf` or `.glb`) file into a scene, combining primitives into one mesh.
pub fn load_gltf(path: impl AsRef<Path>) -> Result<Scene, IoError> {
    let (document, buffers, _images) = gltf::import(path).map_err(|err| IoError::Gltf(err.to_string()))?;

    let mut scene = Scene { mesh: Mesh::new(), materials: vec![Material::default()] };
    let mut material_index: HashMap<usize, u32> = HashMap::new();

    for mesh in document.meshes() {
        for primitive in mesh.primitives() {
            let reader = primitive.reader(|buffer| buffers.get(buffer.index()).map(|data| data.0.as_slice()));

            let positions: Vec<Vec3> = match reader.read_positions() {
                Some(iter) => iter.map(Vec3::from).collect(),
                None => continue,
            };
            if positions.is_empty() {
                continue;
            }

            let material = *material_index.entry(primitive.material().index().unwrap_or(usize::MAX)).or_insert_with(|| {
                let mat = primitive.material();
                let pbr = mat.pbr_metallic_roughness();
                scene.materials.push(Material {
                    name: mat.name().unwrap_or("material").to_string(),
                    base_color: pbr.base_color_factor(),
                    metallic: pbr.metallic_factor(),
                    roughness: pbr.roughness_factor(),
                });
                (scene.materials.len() - 1) as u32
            });

            let base = scene.mesh.positions.len() as u32;
            let first_index = scene.mesh.indices.len() as u32;

            let normals: Vec<Vec3> = reader.read_normals().map(|iter| iter.map(Vec3::from).collect()).unwrap_or_default();
            let uvs: Vec<Vec2> = reader.read_tex_coords(0).map(|iter| iter.into_f32().map(Vec2::from).collect()).unwrap_or_default();
            let indices: Vec<u32> = match reader.read_indices() {
                Some(iter) => iter.into_u32().collect(),
                None => (0..positions.len() as u32).collect(),
            };

            scene.mesh.positions.extend_from_slice(&positions);
            scene.mesh.normals.extend_from_slice(&normals);
            scene.mesh.uvs.extend_from_slice(&uvs);
            scene.mesh.indices.extend(indices.iter().map(|index| index.saturating_add(base)));

            let index_count = scene.mesh.indices.len() as u32 - first_index;
            if index_count > 0 {
                scene.mesh.submeshes.push(Submesh { material, first_index, index_count });
            }
        }
    }

    if scene.mesh.positions.is_empty() {
        return Err(IoError::Empty);
    }
    if scene.mesh.normals.len() != scene.mesh.positions.len() {
        scene.mesh.compute_smooth_normals();
    }
    scene.mesh.complete_attributes();
    scene.mesh.validate()?;

    Ok(scene)
}

/// Resolve a group's material name to an id, registering a new material if needed.
fn material_id(material: &Option<obj::ObjMaterial>, materials: &mut Vec<Material>, index: &mut HashMap<String, u32>) -> u32 {
    let name = match material {
        Some(obj::ObjMaterial::Ref(name)) => Some(name.as_str()),
        Some(obj::ObjMaterial::Mtl(mtl)) => Some(mtl.name.as_str()),
        None => None,
    };
    let Some(name) = name else { return 0 };
    if let Some(&id) = index.get(name) {
        return id;
    }
    let id = materials.len() as u32;
    materials.push(Material { name: name.to_string(), ..Material::default() });
    index.insert(name.to_string(), id);
    id
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRIANGLE_OBJ: &str = "\
o tri
v 0 0 0
v 1 0 0
v 0 1 0
vt 0 0
vt 1 0
vt 0 1
vn 0 0 1
f 1/1/1 2/2/1 3/3/1
";

    const QUAD_OBJ: &str = "\
v -1 -1 0
v 1 -1 0
v 1 1 0
v -1 1 0
f 1 2 3 4
";

    #[test]
    fn parses_a_triangle() {
        let scene = load_obj(TRIANGLE_OBJ.as_bytes()).expect("obj");
        assert_eq!(scene.mesh.vertex_count(), 3);
        assert_eq!(scene.mesh.triangle_count(), 1);
        assert_eq!(scene.mesh.tangents.len(), 3);
    }

    #[test]
    fn triangulates_a_quad() {
        let scene = load_obj(QUAD_OBJ.as_bytes()).expect("obj");
        assert_eq!(scene.mesh.vertex_count(), 4);
        assert_eq!(scene.mesh.triangle_count(), 2);
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(load_obj(b"this is not an obj file\n").is_err());
    }

    #[test]
    fn empty_obj_is_rejected() {
        assert!(matches!(load_obj(b"# just a comment\n").expect_err("empty"), IoError::Empty));
    }

    #[test]
    fn unsupported_extension_is_reported() {
        let err = load_scene("/tmp/does-not-exist.fbx").expect_err("unsupported");
        assert!(matches!(err, IoError::UnsupportedFormat(_)));
    }
}
