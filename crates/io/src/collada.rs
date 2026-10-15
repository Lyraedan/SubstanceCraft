//! Collada (`.dae`) mesh importer.
//!
//! A small, dependency-free XML reader feeds a focused Collada extractor: `<library_geometries>`
//! meshes (`<triangles>`, `<polylist>`, `<polygons>` with `POSITION`/`NORMAL`/`TEXCOORD` accessors)
//! instanced from `<library_visual_scenes>` nodes with `matrix`/`translate`/`rotate`/`scale`
//! transforms. The result is one combined mesh with one submesh per material symbol.

use std::collections::HashMap;

use substancecraft_doc::{Material, Scene};
use substancecraft_geom::glam::{Mat3, Mat4, Quat, Vec2, Vec3};
use substancecraft_geom::{Mesh, Submesh};

use crate::IoError;

/// Maximum element nesting depth accepted by the XML reader and scene walker.
const MAX_DEPTH: usize = 512;

fn collada_err(message: impl Into<String>) -> IoError {
    IoError::Collada(message.into())
}

/// Import a Collada file into a scene.
pub fn load_collada(bytes: &[u8]) -> Result<Scene, IoError> {
    let text = String::from_utf8_lossy(bytes);
    let root = parse_xml(text.as_bytes())?;
    if root.name != "COLLADA" {
        return Err(collada_err("not a COLLADA document"));
    }
    extract(&root)
}

// ---------------------------------------------------------------------------------------------
// Minimal XML DOM
// ---------------------------------------------------------------------------------------------

/// A parsed XML element.
#[derive(Clone, Debug)]
struct Element {
    name: String,
    attrs: Vec<(String, String)>,
    children: Vec<Element>,
    text: String,
}

impl Element {
    fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str())
    }

    fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.name == name)
    }

    fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |child| child.name == name)
    }

    fn numbers(&self) -> Vec<f64> {
        self.text.split_whitespace().filter_map(|token| token.parse::<f64>().ok()).collect()
    }

    fn ints(&self) -> Vec<i64> {
        self.text.split_whitespace().filter_map(|token| token.parse::<f64>().ok()).map(|value| value as i64).collect()
    }
}

/// Parse an XML document into a DOM. Tags/attributes are ASCII; text is decoded as UTF-8.
fn parse_xml(bytes: &[u8]) -> Result<Element, IoError> {
    let mut pos = if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) { 3 } else { 0 };
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;

    while pos < bytes.len() {
        if bytes[pos] == b'<' {
            if bytes[pos..].starts_with(b"<!--") {
                pos = skip_until(bytes, pos + 4, b"-->").ok_or_else(|| collada_err("unterminated comment"))?;
                continue;
            }
            if bytes[pos..].starts_with(b"<![CDATA[") {
                let end = find(bytes, pos + 9, b"]]>").ok_or_else(|| collada_err("unterminated CDATA"))?;
                if let Some(top) = stack.last_mut() {
                    top.text.push_str(&String::from_utf8_lossy(&bytes[pos + 9..end]));
                }
                pos = end + 3;
                continue;
            }
            if bytes[pos..].starts_with(b"<?") {
                pos = skip_until(bytes, pos + 2, b"?>").ok_or_else(|| collada_err("unterminated declaration"))?;
                continue;
            }
            if bytes[pos..].starts_with(b"<!") {
                pos = skip_until(bytes, pos + 2, b">").unwrap_or(bytes.len());
                continue;
            }
            if bytes[pos..].starts_with(b"</") {
                let (_, next) = parse_tag_name(bytes, pos + 2)?;
                pos = skip_until(bytes, next, b">").ok_or_else(|| collada_err("unterminated end tag"))?;
                if let Some(element) = stack.pop() {
                    attach(&mut stack, &mut root, element);
                }
                continue;
            }

            let (name, after_name) = parse_tag_name(bytes, pos + 1)?;
            let TagAttrs { attrs, self_closing, next } = parse_attrs(bytes, after_name)?;
            let element = Element { name, attrs, children: Vec::new(), text: String::new() };
            if self_closing {
                attach(&mut stack, &mut root, element);
            } else {
                if stack.len() >= MAX_DEPTH {
                    return Err(collada_err("XML nesting is too deep"));
                }
                stack.push(element);
            }
            pos = next;
        } else {
            let end = bytes[pos..].iter().position(|byte| *byte == b'<').map(|offset| pos + offset).unwrap_or(bytes.len());
            if let Some(top) = stack.last_mut() {
                top.text.push_str(&decode_entities(&String::from_utf8_lossy(&bytes[pos..end])));
            }
            pos = end;
        }
    }

    root.ok_or_else(|| collada_err("empty document"))
}

/// Push a finished element into its parent (or the root).
fn attach(stack: &mut [Element], root: &mut Option<Element>, element: Element) {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(element);
    } else {
        *root = Some(element);
    }
}

fn parse_tag_name(bytes: &[u8], start: usize) -> Result<(String, usize), IoError> {
    let mut end = start;
    while end < bytes.len() && !bytes[end].is_ascii_whitespace() && bytes[end] != b'>' && bytes[end] != b'/' {
        end += 1;
    }
    if end == start {
        return Err(collada_err("empty tag name"));
    }
    Ok((String::from_utf8_lossy(&bytes[start..end]).into_owned(), end))
}

/// The result of reading a start tag's attributes.
struct TagAttrs {
    attrs: Vec<(String, String)>,
    self_closing: bool,
    next: usize,
}

fn parse_attrs(bytes: &[u8], mut pos: usize) -> Result<TagAttrs, IoError> {
    let mut attrs = Vec::new();
    loop {
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
            pos += 1;
        }
        match bytes.get(pos) {
            Some(b'>') => return Ok(TagAttrs { attrs, self_closing: false, next: pos + 1 }),
            Some(b'/') => {
                return Ok(TagAttrs {
                    attrs,
                    self_closing: true,
                    next: skip_until(bytes, pos, b">").ok_or_else(|| collada_err("unterminated tag"))?,
                });
            }
            Some(_) => {
                let start = pos;
                while pos < bytes.len() && bytes[pos] != b'=' && !bytes[pos].is_ascii_whitespace() && bytes[pos] != b'>' && bytes[pos] != b'/' {
                    pos += 1;
                }
                let key = String::from_utf8_lossy(&bytes[start..pos]).into_owned();
                while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                    pos += 1;
                }
                if bytes.get(pos) == Some(&b'=') {
                    pos += 1;
                    while pos < bytes.len() && bytes[pos].is_ascii_whitespace() {
                        pos += 1;
                    }
                    let quote = *bytes.get(pos).ok_or_else(|| collada_err("unterminated attribute"))?;
                    if quote != b'"' && quote != b'\'' {
                        return Err(collada_err("attribute value must be quoted"));
                    }
                    let value_start = pos + 1;
                    let value_end = bytes[value_start..]
                        .iter()
                        .position(|byte| *byte == quote)
                        .map(|offset| value_start + offset)
                        .ok_or_else(|| collada_err("unterminated attribute value"))?;
                    attrs.push((key, decode_entities(&String::from_utf8_lossy(&bytes[value_start..value_end]))));
                    pos = value_end + 1;
                } else {
                    attrs.push((key, String::new()));
                }
            }
            None => return Err(collada_err("unterminated tag")),
        }
    }
}

fn find(bytes: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    bytes.get(start..).and_then(|slice| slice.windows(needle.len()).position(|window| window == needle).map(|offset| start + offset))
}

fn skip_until(bytes: &[u8], start: usize, needle: &[u8]) -> Option<usize> {
    find(bytes, start, needle).map(|end| end + needle.len())
}

/// Decode the predefined XML entities and numeric character references.
fn decode_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(index) = rest.find('&') {
        out.push_str(&rest[..index]);
        let tail = &rest[index..];
        let Some(semicolon) = tail.find(';') else {
            out.push_str(tail);
            return out;
        };
        match &tail[1..semicolon] {
            "amp" => out.push('&'),
            "lt" => out.push('<'),
            "gt" => out.push('>'),
            "quot" => out.push('"'),
            "apos" => out.push('\''),
            entity if entity.starts_with("#x") || entity.starts_with("#X") => {
                if let Some(ch) = u32::from_str_radix(&entity[2..], 16).ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            entity if entity.starts_with('#') => {
                if let Some(ch) = entity[1..].parse::<u32>().ok().and_then(char::from_u32) {
                    out.push(ch);
                }
            }
            entity => {
                out.push('&');
                out.push_str(entity);
                out.push(';');
            }
        }
        rest = &tail[semicolon + 1..];
    }
    out.push_str(rest);
    out
}

// ---------------------------------------------------------------------------------------------
// Collada extraction
// ---------------------------------------------------------------------------------------------

/// A resolved `<source>` array.
struct Source {
    values: Vec<f64>,
    stride: usize,
}

/// One primitive input binding.
struct Input {
    semantic: String,
    values: Vec<f64>,
    stride: usize,
    offset: usize,
}

/// One polygon corner's attributes.
#[derive(Clone, Copy)]
struct Corner {
    position: Vec3,
    normal: Vec3,
    uv: Vec2,
}

/// Extract the geometry and scenes from a parsed Collada document.
fn extract(root: &Element) -> Result<Scene, IoError> {
    let sources = collect_sources(root);
    let vertices_pos = collect_vertices(root);
    let geometries = collect_by_name(root, "geometry");

    let mut scene = Scene { mesh: Mesh::new(), materials: vec![Material::default()] };
    let mut material_index: HashMap<String, u32> = HashMap::new();

    let Some(visual_scene) = primary_visual_scene(root) else {
        return Err(collada_err("the document has no visual scene"));
    };

    for node in visual_scene.children_named("node") {
        walk_node(node, Mat4::IDENTITY, 0, &geometries, &sources, &vertices_pos, &mut scene, &mut material_index);
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

/// Collect every `<source id=...>` into a map keyed by its id.
fn collect_sources(root: &Element) -> HashMap<String, Source> {
    let mut sources = HashMap::new();
    for_each(root, 0, &mut |element| {
        if element.name != "source" {
            return;
        }
        let Some(id) = element.attr("id") else { return };
        let Some(array) = element.child("float_array") else { return };
        let stride = element
            .child("technique_common")
            .and_then(|common| common.child("accessor"))
            .and_then(|accessor| accessor.attr("stride"))
            .and_then(|stride| stride.parse::<usize>().ok())
            .unwrap_or(1)
            .max(1);
        sources.insert(id.to_string(), Source { values: array.numbers(), stride });
    });
    sources
}

/// Map each `<vertices id=...>` to the id of its `POSITION` source.
fn collect_vertices(root: &Element) -> HashMap<String, String> {
    let mut vertices = HashMap::new();
    for_each(root, 0, &mut |element| {
        if element.name != "vertices" {
            return;
        }
        let Some(id) = element.attr("id") else { return };
        for input in element.children_named("input") {
            if input.attr("semantic") == Some("POSITION")
                && let Some(source) = input.attr("source").and_then(strip_hash)
            {
                vertices.insert(id.to_string(), source.to_string());
            }
        }
    });
    vertices
}

/// Collect every element of a given name by `id`.
fn collect_by_name<'a>(root: &'a Element, name: &str) -> HashMap<String, &'a Element> {
    let mut found = HashMap::new();
    for_each(root, 0, &mut |element| {
        if element.name == name
            && let Some(id) = element.attr("id")
        {
            found.insert(id.to_string(), element);
        }
    });
    found
}

/// Find the visual scene referenced by `<scene><instance_visual_scene url=...>`.
fn primary_visual_scene(root: &Element) -> Option<&Element> {
    let wanted =
        root.child("scene").and_then(|scene| scene.child("instance_visual_scene")).and_then(|instance| instance.attr("url")).and_then(strip_hash);
    let mut fallback = None;
    let mut scene = None;
    for_each(root, 0, &mut |element| {
        if element.name == "visual_scene" {
            if fallback.is_none() {
                fallback = Some(element);
            }
            if wanted == element.attr("id") {
                scene = Some(element);
            }
        }
    });
    scene.or(fallback)
}

/// Depth-first visit every element (depth-limited).
fn for_each<'a>(element: &'a Element, depth: usize, visit: &mut impl FnMut(&'a Element)) {
    if depth > MAX_DEPTH {
        return;
    }
    visit(element);
    for child in &element.children {
        for_each(child, depth + 1, visit);
    }
}

fn strip_hash(value: &str) -> Option<&str> {
    Some(value.strip_prefix('#').unwrap_or(value))
}

/// Walk a visual-scene node, composing transforms and instancing geometries.
#[allow(clippy::too_many_arguments)]
fn walk_node(
    node: &Element,
    parent: Mat4,
    depth: usize,
    geometries: &HashMap<String, &Element>,
    sources: &HashMap<String, Source>,
    vertices_pos: &HashMap<String, String>,
    scene: &mut Scene,
    material_index: &mut HashMap<String, u32>,
) {
    if depth > MAX_DEPTH {
        return;
    }
    let world = parent * node_transform(node);

    for instance in node.children_named("instance_geometry") {
        let Some(geometry_id) = instance.attr("url").and_then(strip_hash) else { continue };
        if let Some(geometry) = geometries.get(geometry_id)
            && let Some(mesh) = geometry.child("mesh")
        {
            append_mesh(mesh, world, sources, vertices_pos, scene, material_index);
        }
    }

    for child in node.children_named("node") {
        walk_node(child, world, depth + 1, geometries, sources, vertices_pos, scene, material_index);
    }
}

/// Compose a node's transform children (`matrix`/`translate`/`rotate`/`scale`).
fn node_transform(node: &Element) -> Mat4 {
    let mut transform = Mat4::IDENTITY;
    for child in &node.children {
        let matrix = match child.name.as_str() {
            "matrix" => {
                let values = child.numbers();
                if values.len() == 16 {
                    let mut columns = [0.0f32; 16];
                    for (slot, value) in columns.iter_mut().zip(&values) {
                        *slot = *value as f32;
                    }
                    // Collada matrices are column-major, matching glam's `from_cols_array`.
                    Some(Mat4::from_cols_array(&columns))
                } else {
                    None
                }
            }
            "translate" => {
                let v = child.numbers();
                (v.len() == 3).then(|| Mat4::from_translation(Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)))
            }
            "scale" => {
                let v = child.numbers();
                (v.len() == 3).then(|| Mat4::from_scale(Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)))
            }
            "rotate" => {
                let v = child.numbers();
                if v.len() == 4 {
                    let axis = Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32).normalize_or_zero();
                    Some(Mat4::from_quat(Quat::from_axis_angle(axis, (v[3] as f32).to_radians())))
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(matrix) = matrix {
            transform *= matrix;
        }
    }
    transform
}

/// Append one Collada `<mesh>` to the combined mesh.
fn append_mesh(
    mesh_el: &Element,
    world: Mat4,
    sources: &HashMap<String, Source>,
    vertices_pos: &HashMap<String, String>,
    scene: &mut Scene,
    material_index: &mut HashMap<String, u32>,
) {
    let normal_matrix = Mat3::from_mat4(world).inverse().transpose();

    for primitive in &mesh_el.children {
        if !matches!(primitive.name.as_str(), "triangles" | "polylist" | "polygons") {
            continue;
        }
        let symbol = primitive.attr("material").unwrap_or("").to_string();
        let material = *material_index.entry(symbol.clone()).or_insert_with(|| {
            let index = scene.materials.len() as u32;
            scene.materials.push(Material { name: symbol, ..Material::default() });
            index
        });

        let inputs = resolve_inputs(primitive, sources, vertices_pos);
        let Some(vertex_input) = inputs.iter().position(|input| input.semantic == "VERTEX") else { continue };
        let normal_input = inputs.iter().position(|input| input.semantic == "NORMAL");
        let uv_input = inputs.iter().position(|input| input.semantic == "TEXCOORD");
        let stride = inputs.iter().map(|input| input.offset).max().map(|offset| offset + 1).unwrap_or(0);
        if stride == 0 {
            continue;
        }

        let vcount = primitive.child("vcount").map(Element::ints);
        let mut polygons: Vec<Vec<Corner>> = Vec::new();
        for p in primitive.children_named("p") {
            let numbers = p.ints();
            let groups: Vec<&[i64]> = numbers.chunks(stride).collect();
            let counts: Vec<usize> = match primitive.name.as_str() {
                "triangles" => vec![3; groups.len() / 3],
                "polylist" => vcount.as_ref().map(|counts| counts.iter().map(|count| (*count).max(0) as usize).collect()).unwrap_or_default(),
                _ => vec![groups.len()],
            };

            let mut cursor = 0usize;
            for count in counts {
                let end = cursor.saturating_add(count).min(groups.len());
                let polygon: Vec<Corner> =
                    groups[cursor..end].iter().map(|group| make_corner(&inputs, vertex_input, normal_input, uv_input, group)).collect();
                cursor = end;
                if polygon.len() >= 3 {
                    polygons.push(polygon);
                }
            }
        }

        let first_index = scene.mesh.indices.len() as u32;
        for polygon in &polygons {
            for corner in polygon {
                scene.mesh.positions.push(world.transform_point3(corner.position));
                scene.mesh.normals.push((normal_matrix * corner.normal).normalize_or_zero());
                scene.mesh.uvs.push(corner.uv);
            }
            let base = scene.mesh.positions.len() as u32 - polygon.len() as u32;
            for corner in 1..polygon.len() - 1 {
                scene.mesh.indices.push(base);
                scene.mesh.indices.push(base + corner as u32);
                scene.mesh.indices.push(base + corner as u32 + 1);
            }
        }
        let index_count = scene.mesh.indices.len() as u32 - first_index;
        if index_count > 0 {
            scene.mesh.submeshes.push(Submesh { material, first_index, index_count });
        }
    }
}

/// Build one corner from its index group.
fn make_corner(inputs: &[Input], vertex_input: usize, normal_input: Option<usize>, uv_input: Option<usize>, group: &[i64]) -> Corner {
    let position = sample3(&inputs[vertex_input], corner_index(inputs, vertex_input, group));
    let normal = normal_input.map(|index| sample3(&inputs[index], corner_index(inputs, index, group))).unwrap_or(Vec3::ZERO);
    let uv = uv_input.map(|index| sample2(&inputs[index], corner_index(inputs, index, group))).unwrap_or(Vec2::ZERO);
    Corner { position, normal, uv }
}

/// The value index for one input within a corner group.
fn corner_index(inputs: &[Input], input: usize, group: &[i64]) -> usize {
    group.get(inputs[input].offset).copied().unwrap_or(0).max(0) as usize
}

fn sample3(input: &Input, index: usize) -> Vec3 {
    let start = index.saturating_mul(input.stride);
    let value = |component: usize| input.values.get(start + component).copied().unwrap_or(0.0) as f32;
    Vec3::new(value(0), value(1), value(2))
}

fn sample2(input: &Input, index: usize) -> Vec2 {
    let start = index.saturating_mul(input.stride);
    let value = |component: usize| input.values.get(start + component).copied().unwrap_or(0.0) as f32;
    Vec2::new(value(0), value(1))
}

/// Resolve a primitive's inputs into flat value arrays.
fn resolve_inputs(primitive: &Element, sources: &HashMap<String, Source>, vertices_pos: &HashMap<String, String>) -> Vec<Input> {
    let mut inputs = Vec::new();
    for input in primitive.children_named("input") {
        let Some(semantic) = input.attr("semantic") else { continue };
        let mut source_id = input.attr("source").and_then(strip_hash).unwrap_or("").to_string();
        if semantic == "VERTEX"
            && let Some(resolved) = vertices_pos.get(&source_id)
        {
            source_id = resolved.clone();
        }
        let Some(source) = sources.get(&source_id) else { continue };
        let offset = input.attr("offset").and_then(|value| value.parse::<usize>().ok()).unwrap_or(0);
        inputs.push(Input { semantic: semantic.to_string(), values: source.values.clone(), stride: source.stride, offset });
    }
    inputs
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRIANGLE_DAE: &str = r##"<?xml version="1.0"?>
<COLLADA xmlns="http://www.collada.org/2005/11/COLLADASchema" version="1.4.1">
  <library_geometries>
    <geometry id="tri" name="tri">
      <mesh>
        <source id="pos">
          <float_array id="pos-array" count="9">0 0 0 1 0 0 0 1 0</float_array>
          <technique_common><accessor source="#pos-array" count="3" stride="3"><param name="X"/><param name="Y"/><param name="Z"/></accessor></technique_common>
        </source>
        <vertices id="verts"><input semantic="POSITION" source="#pos"/></vertices>
        <triangles count="1" material="Mat0">
          <input semantic="VERTEX" source="#verts" offset="0"/>
          <p>0 1 2</p>
        </triangles>
      </mesh>
    </geometry>
  </library_geometries>
  <library_visual_scenes>
    <visual_scene id="Scene">
      <node id="Node"><instance_geometry url="#tri"/></node>
    </visual_scene>
  </library_visual_scenes>
  <scene><instance_visual_scene url="#Scene"/></scene>
</COLLADA>
"##;

    #[test]
    fn parses_a_minimal_collada_triangle() {
        let scene = load_collada(TRIANGLE_DAE.as_bytes()).expect("collada");
        assert_eq!(scene.mesh.triangle_count(), 1);
        assert_eq!(scene.mesh.vertex_count(), 3);
        assert_eq!(scene.materials.len(), 2);
        assert_eq!(scene.materials[1].name, "Mat0");
    }

    #[test]
    fn applies_node_transforms() {
        let dae = TRIANGLE_DAE.replace("<node id=\"Node\">", "<node id=\"Node\"><translate>1 2 3</translate>");
        let scene = load_collada(dae.as_bytes()).expect("collada");
        assert!((scene.mesh.positions[0] - Vec3::new(1.0, 2.0, 3.0)).length() < 1e-5);
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(load_collada(b"not xml at all").is_err());
        assert!(load_collada(b"<notcollada><a></notcollada").is_err());
    }
}
