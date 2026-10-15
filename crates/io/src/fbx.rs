//! Binary FBX (7.x) mesh importer, written from the public file-format description.
//!
//! Only the binary container is supported (ASCII FBX is not). The reader parses the node tree
//! defensively — every read is bounds-checked, recursion is depth-limited and array sizes are
//! capped — then interprets `Objects`/`Connections` into one combined [`Scene`] mesh with one
//! submesh per material. Geometry, polygon-vertex normals/UVs, per-polygon materials and the
//! `Lcl Translation/Rotation/Scaling` + `PreRotation` node transforms are applied.

use std::collections::HashMap;

use substancecraft_doc::{Material, Scene};
use substancecraft_geom::glam::{EulerRot, Mat3, Mat4, Vec2, Vec3};
use substancecraft_geom::{Mesh, Submesh};

use crate::IoError;

/// The 23-byte binary header prefix (magic + `0x1a 0x00`).
const MAGIC: &[u8] = b"Kaydara FBX Binary  \x00\x1a\x00";
/// Maximum node nesting depth accepted.
const MAX_DEPTH: usize = 256;
/// Cap on a single decompressed array, to keep a hostile file from exhausting memory.
const MAX_ARRAY_BYTES: usize = 256 * 1024 * 1024;

fn fbx_err(message: impl Into<String>) -> IoError {
    IoError::Fbx(message.into())
}

/// Whether these bytes begin with the binary FBX header.
pub fn is_binary(bytes: &[u8]) -> bool {
    bytes.starts_with(MAGIC)
}

/// Import a binary FBX file into a scene.
pub fn load_fbx(bytes: &[u8]) -> Result<Scene, IoError> {
    let doc = parse_document(bytes)?;
    interpret(&doc)
}

/// A parsed FBX node: a name, its properties and its children.
#[derive(Clone, Debug)]
struct Node {
    name: String,
    properties: Vec<Property>,
    children: Vec<Node>,
}

/// A parsed FBX property value. Some payloads (raw blocks, boolean flags) are parsed only so the
/// stream stays aligned; the mesh interpreters never read them.
#[derive(Clone, Debug)]
#[allow(dead_code)]
enum Property {
    I16(i16),
    Bool(bool),
    I32(i32),
    F32(f32),
    F64(f64),
    I64(i64),
    F32Array(Vec<f32>),
    F64Array(Vec<f64>),
    I32Array(Vec<i32>),
    I64Array(Vec<i64>),
    BoolArray(Vec<bool>),
    String(String),
    Raw(Vec<u8>),
}

impl Property {
    fn as_i64(&self) -> Option<i64> {
        match self {
            Property::I16(v) => Some(*v as i64),
            Property::I32(v) => Some(*v as i64),
            Property::I64(v) => Some(*v),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Property::F32(v) => Some(*v as f64),
            Property::F64(v) => Some(*v),
            Property::I16(v) => Some(*v as f64),
            Property::I32(v) => Some(*v as f64),
            Property::I64(v) => Some(*v as f64),
            _ => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Property::String(value) => Some(value),
            _ => None,
        }
    }

    fn as_f64_array(&self) -> Option<Vec<f64>> {
        match self {
            Property::F64Array(values) => Some(values.clone()),
            Property::F32Array(values) => Some(values.iter().map(|value| *value as f64).collect()),
            _ => None,
        }
    }

    fn as_i32_array(&self) -> Option<Vec<i32>> {
        match self {
            Property::I32Array(values) => Some(values.clone()),
            Property::I64Array(values) => Some(values.iter().map(|value| *value as i32).collect()),
            _ => None,
        }
    }
}

impl Node {
    fn child(&self, name: &str) -> Option<&Node> {
        self.children.iter().find(|child| child.name == name)
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> {
        self.children.iter().filter(move |child| child.name == name)
    }

    fn first_i64(&self) -> Option<i64> {
        self.properties.first().and_then(Property::as_i64)
    }
}

fn find_node<'a>(nodes: &'a [Node], name: &str) -> Option<&'a Node> {
    nodes.iter().find(|node| node.name == name)
}

/// Parse the whole document into top-level nodes.
fn parse_document(bytes: &[u8]) -> Result<Vec<Node>, IoError> {
    if bytes.len() < 27 || !bytes.starts_with(MAGIC) {
        return Err(fbx_err("not a binary FBX file (ASCII FBX is not supported)"));
    }
    let version = u32::from_le_bytes([bytes[23], bytes[24], bytes[25], bytes[26]]);
    if !(7000..=7700).contains(&version) {
        return Err(fbx_err(format!("unsupported FBX version {version}")));
    }

    let mut reader = Reader { bytes, pos: 27, version };
    let mut nodes = Vec::new();
    while reader.pos < reader.bytes.len() {
        match parse_node(&mut reader, 0)? {
            Some(node) => nodes.push(node),
            None => break,
        }
    }
    Ok(nodes)
}

/// A bounds-checked little-endian cursor over the file.
struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
    version: u32,
}

impl<'a> Reader<'a> {
    fn is_64(&self) -> bool {
        self.version >= 7500
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], IoError> {
        let end = self.pos.checked_add(count).ok_or_else(|| fbx_err("offset overflow"))?;
        let slice = self.bytes.get(self.pos..end).ok_or_else(|| fbx_err("unexpected end of file"))?;
        self.pos = end;
        Ok(slice)
    }

    fn u8(&mut self) -> Result<u8, IoError> {
        Ok(self.take(1)?.first().copied().unwrap_or(0))
    }

    fn u16(&mut self) -> Result<u16, IoError> {
        let bytes = self.take(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    fn u32(&mut self) -> Result<u32, IoError> {
        let bytes = self.take(4)?;
        Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    }

    fn u64(&mut self) -> Result<u64, IoError> {
        let bytes = self.take(8)?;
        Ok(u64::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]]))
    }

    fn i16(&mut self) -> Result<i16, IoError> {
        Ok(self.u16()? as i16)
    }

    fn i32(&mut self) -> Result<i32, IoError> {
        Ok(self.u32()? as i32)
    }

    fn i64(&mut self) -> Result<i64, IoError> {
        Ok(self.u64()? as i64)
    }

    fn f32(&mut self) -> Result<f32, IoError> {
        Ok(f32::from_bits(self.u32()?))
    }

    fn f64(&mut self) -> Result<f64, IoError> {
        Ok(f64::from_bits(self.u64()?))
    }

    /// Read a node/array offset or count (u32 before FBX 7500, u64 after).
    fn offset(&mut self) -> Result<u64, IoError> {
        if self.is_64() { self.u64() } else { Ok(self.u32()? as u64) }
    }

    fn fbx_string(&mut self) -> Result<String, IoError> {
        let len = self.u32()? as usize;
        let bytes = self.take(len)?;
        Ok(String::from_utf8_lossy(bytes).into_owned())
    }

    fn raw(&mut self) -> Result<Vec<u8>, IoError> {
        let len = self.u32()? as usize;
        Ok(self.take(len)?.to_vec())
    }

    /// Read an array's payload bytes, inflating when it is deflated.
    fn array_bytes(&mut self, element_size: usize) -> Result<Vec<u8>, IoError> {
        let length = self.u32()? as usize;
        let encoding = self.u32()?;
        let compressed = self.u32()? as usize;
        let expected = length.checked_mul(element_size).ok_or_else(|| fbx_err("array size overflow"))?;
        if expected > MAX_ARRAY_BYTES {
            return Err(fbx_err("array is too large"));
        }
        match encoding {
            0 => Ok(self.take(expected)?.to_vec()),
            1 => {
                let raw = self.take(compressed)?;
                inflate(raw)
            }
            other => Err(fbx_err(format!("unknown array encoding {other}"))),
        }
    }

    fn f32_array(&mut self) -> Result<Vec<f32>, IoError> {
        let bytes = self.array_bytes(4)?;
        Ok(bytes.as_chunks::<4>().0.iter().map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
    }

    fn f64_array(&mut self) -> Result<Vec<f64>, IoError> {
        let bytes = self.array_bytes(8)?;
        Ok(bytes.as_chunks::<8>().0.iter().map(|c| f64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])).collect())
    }

    fn i32_array(&mut self) -> Result<Vec<i32>, IoError> {
        let bytes = self.array_bytes(4)?;
        Ok(bytes.as_chunks::<4>().0.iter().map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect())
    }

    fn i64_array(&mut self) -> Result<Vec<i64>, IoError> {
        let bytes = self.array_bytes(8)?;
        Ok(bytes.as_chunks::<8>().0.iter().map(|c| i64::from_le_bytes([c[0], c[1], c[2], c[3], c[4], c[5], c[6], c[7]])).collect())
    }

    fn bool_array(&mut self) -> Result<Vec<bool>, IoError> {
        let bytes = self.array_bytes(1)?;
        Ok(bytes.iter().map(|b| *b != 0).collect())
    }
}

/// Inflate a zlib stream (FBX's deflated array encoding).
fn inflate(raw: &[u8]) -> Result<Vec<u8>, IoError> {
    use std::io::Read;
    let mut decoder = flate2::read::ZlibDecoder::new(raw);
    let mut out = Vec::new();
    decoder.by_ref().take((MAX_ARRAY_BYTES + 1) as u64).read_to_end(&mut out).map_err(|err| fbx_err(format!("zlib error: {err}")))?;
    if out.len() > MAX_ARRAY_BYTES {
        return Err(fbx_err("decompressed array is too large"));
    }
    Ok(out)
}

/// Parse one node; `Ok(None)` marks the null record that ends a node list.
fn parse_node(reader: &mut Reader<'_>, depth: usize) -> Result<Option<Node>, IoError> {
    if depth > MAX_DEPTH {
        return Err(fbx_err("node nesting is too deep"));
    }
    let start = reader.pos;
    let end_offset = reader.offset()? as usize;
    if end_offset == 0 {
        return Ok(None);
    }
    if end_offset <= start || end_offset > reader.bytes.len() {
        return Err(fbx_err("invalid node end offset"));
    }

    let property_count = reader.offset()?;
    if property_count > 100_000 {
        return Err(fbx_err("unreasonable property count"));
    }
    let _property_list_len = reader.offset()?;
    let name_len = reader.u8()? as usize;
    let name = String::from_utf8_lossy(reader.take(name_len)?).into_owned();

    let mut properties = Vec::with_capacity(property_count as usize);
    for _ in 0..property_count {
        let kind = reader.u8()?;
        let property = match kind {
            b'Y' => Property::I16(reader.i16()?),
            b'C' => Property::Bool(reader.u8()? != 0),
            b'I' => Property::I32(reader.i32()?),
            b'F' => Property::F32(reader.f32()?),
            b'D' => Property::F64(reader.f64()?),
            b'L' => Property::I64(reader.i64()?),
            b'f' => Property::F32Array(reader.f32_array()?),
            b'd' => Property::F64Array(reader.f64_array()?),
            b'l' => Property::I64Array(reader.i64_array()?),
            b'i' => Property::I32Array(reader.i32_array()?),
            b'b' => Property::BoolArray(reader.bool_array()?),
            b'S' => Property::String(reader.fbx_string()?),
            b'R' => Property::Raw(reader.raw()?),
            other => return Err(fbx_err(format!("unknown property type {}", other as char))),
        };
        properties.push(property);
    }

    let mut children = Vec::new();
    while reader.pos < end_offset {
        match parse_node(reader, depth + 1)? {
            Some(child) => children.push(child),
            None => break,
        }
    }
    // Skip any trailing padding to the declared end.
    reader.pos = reader.pos.max(end_offset.min(reader.bytes.len()));

    Ok(Some(Node { name, properties, children }))
}

// ---------------------------------------------------------------------------------------------
// Interpretation
// ---------------------------------------------------------------------------------------------

/// Interpret the parsed nodes into a scene.
fn interpret(doc: &[Node]) -> Result<Scene, IoError> {
    let Some(objects) = find_node(doc, "Objects") else {
        return Err(fbx_err("the file has no Objects section"));
    };

    let mut scene = Scene { mesh: Mesh::new(), materials: vec![Material::default()] };
    let mut material_index: HashMap<i64, u32> = HashMap::new();
    let mut material_order: Vec<i64> = Vec::new();

    for object in objects.children_named("Material") {
        let Some(id) = object.first_i64() else { continue };
        if material_index.contains_key(&id) {
            continue;
        }
        let name = object.properties.get(1).and_then(Property::as_str).unwrap_or("material").to_string();
        let index = scene.materials.len() as u32;
        scene.materials.push(Material { name, ..Material::default() });
        material_index.insert(id, index);
        material_order.push(id);
    }

    // Connections: OO links (child object -> parent object).
    let mut object_parent: HashMap<i64, i64> = HashMap::new();
    if let Some(connections) = find_node(doc, "Connections") {
        for connection in connections.children_named("C") {
            let kind = connection.properties.first().and_then(Property::as_str);
            if kind != Some("OO") {
                continue;
            }
            let Some(child) = connection.properties.get(1).and_then(Property::as_i64) else { continue };
            let Some(parent) = connection.properties.get(2).and_then(Property::as_i64) else { continue };
            object_parent.insert(child, parent);
        }
    }
    let model_ids: Vec<i64> = objects.children_named("Model").filter_map(Node::first_i64).collect();
    let is_model = |id: i64| model_ids.contains(&id);

    let locals: HashMap<i64, Mat4> = objects.children_named("Model").filter_map(|model| Some((model.first_i64()?, local_matrix(model)))).collect();
    let mut world_cache: HashMap<i64, Mat4> = HashMap::new();

    for geometry in objects.children_named("Geometry") {
        let Some(geometry_id) = geometry.first_i64() else { continue };
        let kind = geometry.properties.get(2).and_then(Property::as_str).unwrap_or("Mesh");
        if kind != "Mesh" {
            continue;
        }

        // The model that owns this geometry, and its world transform.
        let owner = object_parent.get(&geometry_id).copied().filter(|id| is_model(*id));
        let world = owner.map(|id| world_matrix(id, &locals, &object_parent, &mut world_cache, 0)).unwrap_or(Mat4::IDENTITY);
        let normal_matrix = Mat3::from_mat4(world).inverse().transpose();

        // Materials connected to the owner, in declaration order.
        let model_materials: Vec<u32> =
            material_order.iter().filter(|id| owner == object_parent.get(*id).copied()).filter_map(|id| material_index.get(id).copied()).collect();

        append_geometry(&mut scene.mesh, geometry, world, normal_matrix, &model_materials)?;
    }

    if scene.mesh.positions.is_empty() {
        return Err(IoError::Empty);
    }
    if scene.mesh.normals.iter().all(|normal| normal.length_squared() == 0.0) {
        scene.mesh.compute_smooth_normals();
    }
    scene.mesh.complete_attributes();
    scene.mesh.validate()?;
    Ok(scene)
}

/// The world matrix for a model, walking up the parent chain (depth-limited).
fn world_matrix(id: i64, locals: &HashMap<i64, Mat4>, parents: &HashMap<i64, i64>, cache: &mut HashMap<i64, Mat4>, depth: usize) -> Mat4 {
    if let Some(matrix) = cache.get(&id) {
        return *matrix;
    }
    if depth > MAX_DEPTH {
        return Mat4::IDENTITY;
    }
    let local = locals.get(&id).copied().unwrap_or(Mat4::IDENTITY);
    let world = match parents.get(&id).copied() {
        Some(parent) => world_matrix(parent, locals, parents, cache, depth + 1) * local,
        None => local,
    };
    cache.insert(id, world);
    world
}

/// Read a `Properties70` value triplet (translation/rotation/scaling).
fn properties70(node: &Node, key: &str) -> Option<[f64; 3]> {
    let props = node.child("Properties70")?;
    for property in props.children_named("P") {
        if property.properties.first().and_then(Property::as_str) != Some(key) {
            continue;
        }
        let x = property.properties.get(4).and_then(Property::as_f64)?;
        let y = property.properties.get(5).and_then(Property::as_f64).unwrap_or(x);
        let z = property.properties.get(6).and_then(Property::as_f64).unwrap_or(x);
        return Some([x, y, z]);
    }
    None
}

/// Read the local transform from a model's `Lcl Translation/Rotation/Scaling` + `PreRotation`.
fn local_matrix(model: &Node) -> Mat4 {
    let translation = properties70(model, "Lcl Translation").unwrap_or([0.0; 3]);
    let rotation = properties70(model, "Lcl Rotation").unwrap_or([0.0; 3]);
    let scaling = properties70(model, "Lcl Scaling").unwrap_or([1.0; 3]);
    let pre = properties70(model, "PreRotation").unwrap_or([0.0; 3]);

    let order = rotation_order(model);
    let translation = Mat4::from_translation(Vec3::new(translation[0] as f32, translation[1] as f32, translation[2] as f32));
    let pre = Mat4::from_euler(order, pre[0].to_radians() as f32, pre[1].to_radians() as f32, pre[2].to_radians() as f32);
    let rotation = Mat4::from_euler(order, rotation[0].to_radians() as f32, rotation[1].to_radians() as f32, rotation[2].to_radians() as f32);
    let scaling = Mat4::from_scale(Vec3::new(scaling[0] as f32, scaling[1] as f32, scaling[2] as f32));

    translation * pre * rotation * scaling
}

/// Resolve the FBX rotation order (defaults to XYZ).
fn rotation_order(model: &Node) -> EulerRot {
    let Some(props) = model.child("Properties70") else { return EulerRot::XYZ };
    for property in props.children_named("P") {
        if property.properties.first().and_then(Property::as_str) != Some("RotationOrder") {
            continue;
        }
        let code = property.properties.get(4).and_then(Property::as_i64).unwrap_or(0);
        return match code {
            0 => EulerRot::XYZ,
            1 => EulerRot::XZY,
            2 => EulerRot::YXZ,
            3 => EulerRot::YZX,
            4 => EulerRot::ZXY,
            _ => EulerRot::ZYX,
        };
    }
    EulerRot::XYZ
}

/// A per-corner UV or normal layer.
struct CornerLayer<T> {
    mapping: String,
    indexed: bool,
    values: Vec<T>,
    indices: Vec<i32>,
}

/// One polygon corner's attributes.
#[derive(Clone, Copy)]
struct Corner {
    position: Vec3,
    normal: Vec3,
    uv: Vec2,
}

/// Append one geometry node's polygons to the combined mesh.
fn append_geometry(mesh: &mut Mesh, geometry: &Node, world: Mat4, normal_matrix: Mat3, model_materials: &[u32]) -> Result<(), IoError> {
    let positions = geometry
        .child("Vertices")
        .and_then(|node| node.properties.first())
        .and_then(Property::as_f64_array)
        .ok_or_else(|| fbx_err("geometry has no vertices"))?;
    let positions: Vec<Vec3> = positions.as_chunks::<3>().0.iter().map(|c| Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)).collect();

    let polygon_index = geometry
        .child("PolygonVertexIndex")
        .and_then(|node| node.properties.first())
        .and_then(Property::as_i32_array)
        .ok_or_else(|| fbx_err("geometry has no polygon indices"))?;

    let polygons = split_polygons(&polygon_index);
    let normals = read_corner_layer::<Vec3>(geometry, "LayerElementNormal", "Normals", |values| {
        values.as_chunks::<3>().0.iter().map(|c| Vec3::new(c[0] as f32, c[1] as f32, c[2] as f32)).collect()
    });
    let uvs = read_corner_layer::<Vec2>(geometry, "LayerElementUV", "UV", |values| {
        values.as_chunks::<2>().0.iter().map(|c| Vec2::new(c[0] as f32, c[1] as f32)).collect()
    });
    let polygon_materials = read_polygon_materials(geometry);

    let mut corner_counter = 0usize;
    let mut current_material: Option<u32> = None;

    for (polygon_index_number, polygon) in polygons.iter().enumerate() {
        let mut corners: Vec<Corner> = Vec::with_capacity(polygon.len());
        for &vertex in polygon {
            let mut normal = Vec3::ZERO;
            let mut uv = Vec2::ZERO;
            let vertex_index = vertex.max(0) as usize;

            if let Some(layer) = &normals {
                let index = corner_index(&layer.mapping, layer.indexed, corner_counter, vertex_index, polygon_index_number, &layer.indices);
                if let Some(value) = index.and_then(|i| layer.values.get(i as usize)) {
                    normal = *value;
                }
            }
            if let Some(layer) = &uvs {
                let index = corner_index(&layer.mapping, layer.indexed, corner_counter, vertex_index, polygon_index_number, &layer.indices);
                if let Some(value) = index.and_then(|i| layer.values.get(i as usize)) {
                    uv = *value;
                }
            }
            let position = positions.get(vertex_index).copied().unwrap_or(Vec3::ZERO);
            corners.push(Corner { position, normal, uv });
            corner_counter += 1;
        }

        let material = polygon_material(polygon_index_number, &polygon_materials, model_materials);

        for triangle in 1..corners.len().saturating_sub(1) {
            for corner in [corners[0], corners[triangle], corners[triangle + 1]] {
                let position = world.transform_point3(corner.position);
                let normal = (normal_matrix * corner.normal).normalize_or_zero();
                mesh.positions.push(position);
                mesh.normals.push(normal);
                mesh.uvs.push(corner.uv);
            }

            if current_material != Some(material) {
                mesh.submeshes.push(Submesh { material, first_index: mesh.indices.len() as u32, index_count: 0 });
                current_material = Some(material);
            }
            let base = mesh.positions.len() as u32;
            mesh.indices.extend_from_slice(&[base - 3, base - 2, base - 1]);
            if let Some(submesh) = mesh.submeshes.last_mut() {
                submesh.index_count += 3;
            }
        }
    }

    Ok(())
}

/// Split an FBX polygon index stream into per-polygon vertex lists (a negative entry ends one).
fn split_polygons(indices: &[i32]) -> Vec<Vec<i32>> {
    let mut polygons = Vec::new();
    let mut current = Vec::new();
    for &index in indices {
        if index < 0 {
            current.push(!index);
            polygons.push(std::mem::take(&mut current));
        } else {
            current.push(index);
        }
        if polygons.len() > 100_000_000 {
            break;
        }
    }
    if !current.is_empty() {
        polygons.push(current);
    }
    polygons
}

/// Read a `LayerElement*` corner layer (normals or UVs) into mapped values.
fn read_corner_layer<T>(geometry: &Node, element: &str, array: &str, map: impl Fn(&[f64]) -> Vec<T>) -> Option<CornerLayer<T>> {
    let layer = geometry.child(element)?;
    let mapping = layer.child("MappingInformationType")?.properties.first()?.as_str()?.to_string();
    let indexed = layer.child("ReferenceInformationType")?.properties.first()?.as_str()? == "IndexToDirect";
    let values = map(&layer.child(array)?.properties.first()?.as_f64_array()?);
    let indices = layer.child(&format!("{array}Index")).and_then(|node| node.properties.first()).and_then(Property::as_i32_array).unwrap_or_default();
    Some(CornerLayer { mapping, indexed, values, indices })
}

/// Resolve a corner's index into a corner layer by its mapping type.
fn corner_index(mapping: &str, indexed: bool, corner: usize, vertex: usize, polygon: usize, indices: &[i32]) -> Option<i32> {
    match mapping {
        "ByPolygonVertex" => {
            if indexed {
                indices.get(corner).copied()
            } else {
                Some(corner as i32)
            }
        }
        "ByVertex" | "ByVertice" => Some(vertex as i32),
        "ByPolygon" => Some(polygon as i32),
        _ => Some(corner as i32),
    }
}

/// Read the per-polygon material indices, if the geometry carries them.
fn read_polygon_materials(geometry: &Node) -> Vec<i32> {
    geometry
        .child("LayerElementMaterial")
        .and_then(|layer| layer.child("Materials"))
        .and_then(|node| node.properties.first())
        .and_then(Property::as_i32_array)
        .unwrap_or_default()
}

/// Map a polygon's local material index to a scene material index.
fn polygon_material(polygon: usize, polygon_materials: &[i32], model_materials: &[u32]) -> u32 {
    let local = polygon_materials.get(polygon).copied().or_else(|| polygon_materials.first().copied()).unwrap_or(0);
    if local < 0 {
        return 0;
    }
    model_materials.get(local as usize).copied().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prop_i64(value: i64) -> Vec<u8> {
        let mut out = vec![b'L'];
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn prop_f64(value: f64) -> Vec<u8> {
        let mut out = vec![b'D'];
        out.extend_from_slice(&value.to_le_bytes());
        out
    }

    fn prop_str(value: &str) -> Vec<u8> {
        let mut out = vec![b'S'];
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value.as_bytes());
        out
    }

    fn prop_f64_array(values: &[f64]) -> Vec<u8> {
        let mut out = vec![b'd'];
        out.extend_from_slice(&(values.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // encoding: raw
        out.extend_from_slice(&0u32.to_le_bytes()); // compressed length
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    fn prop_i32_array(values: &[i32]) -> Vec<u8> {
        let mut out = vec![b'i'];
        out.extend_from_slice(&(values.len() as u32).to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes());
        for value in values {
            out.extend_from_slice(&value.to_le_bytes());
        }
        out
    }

    /// A minimal binary FBX 7400 encoder used only to build test fixtures.
    struct Enc {
        out: Vec<u8>,
    }

    impl Enc {
        fn new() -> Self {
            let mut out = Vec::new();
            out.extend_from_slice(MAGIC);
            out.extend_from_slice(&7400u32.to_le_bytes());
            Self { out }
        }

        fn node(&mut self, name: &str, properties: &[Vec<u8>], children: impl FnOnce(&mut Enc)) {
            let start = self.out.len();
            self.out.extend_from_slice(&[0u8; 4]); // end offset (patched)
            self.out.extend_from_slice(&(properties.len() as u32).to_le_bytes());
            let property_bytes: usize = properties.iter().map(Vec::len).sum();
            self.out.extend_from_slice(&(property_bytes as u32).to_le_bytes());
            self.out.push(name.len() as u8);
            self.out.extend_from_slice(name.as_bytes());
            for property in properties {
                self.out.extend_from_slice(property);
            }
            children(self);
            self.out.extend_from_slice(&[0u8; 13]); // null record
            let end = self.out.len() as u32;
            self.out[start..start + 4].copy_from_slice(&end.to_le_bytes());
        }
    }

    /// One triangle at the origin, a model translated to (1,2,3), and an OO connection.
    fn fixture() -> Vec<u8> {
        let mut enc = Enc::new();
        enc.node("Objects", &[], |enc| {
            enc.node("Geometry", &[prop_i64(100), prop_str("Geom"), prop_str("Mesh")], |enc| {
                enc.node("Vertices", &[prop_f64_array(&[0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 0.0])], |_| {});
                enc.node("PolygonVertexIndex", &[prop_i32_array(&[0, 1, -3])], |_| {});
                enc.node("LayerElementNormal", &[], |enc| {
                    enc.node("MappingInformationType", &[prop_str("ByPolygonVertex")], |_| {});
                    enc.node("ReferenceInformationType", &[prop_str("Direct")], |_| {});
                    enc.node("Normals", &[prop_f64_array(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0])], |_| {});
                });
                enc.node("LayerElementUV", &[], |enc| {
                    enc.node("MappingInformationType", &[prop_str("ByPolygonVertex")], |_| {});
                    enc.node("ReferenceInformationType", &[prop_str("Direct")], |_| {});
                    enc.node("UV", &[prop_f64_array(&[0.0, 0.0, 1.0, 0.0, 0.0, 1.0])], |_| {});
                });
            });
            enc.node("Model", &[prop_i64(200), prop_str("Model"), prop_str("Mesh")], |enc| {
                enc.node("Properties70", &[], |enc| {
                    enc.node(
                        "P",
                        &[
                            prop_str("Lcl Translation"),
                            prop_str("Lcl Translation"),
                            prop_str(""),
                            prop_str("A"),
                            prop_f64(1.0),
                            prop_f64(2.0),
                            prop_f64(3.0),
                        ],
                        |_| {},
                    );
                });
            });
        });
        enc.node("Connections", &[], |enc| {
            enc.node("C", &[prop_str("OO"), prop_i64(100), prop_i64(200)], |_| {});
        });
        enc.out
    }

    #[test]
    fn imports_geometry_normals_uvs_and_transform() {
        let scene = load_fbx(&fixture()).expect("fbx");
        assert_eq!(scene.mesh.triangle_count(), 1);
        assert_eq!(scene.mesh.vertex_count(), 3);
        // The model's Lcl Translation (1,2,3) moved the triangle.
        assert!((scene.mesh.positions[0] - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-5);
        assert!((scene.mesh.normals[0].z - 1.0).abs() < 1e-5);
        assert!((scene.mesh.uvs[1].x - 1.0).abs() < 1e-5);
    }

    #[test]
    fn rejects_non_fbx_and_truncated_input() {
        assert!(load_fbx(b"this is not fbx").is_err());
        let bytes = fixture();
        assert!(load_fbx(&bytes[..bytes.len() / 2]).is_err());
    }
}
