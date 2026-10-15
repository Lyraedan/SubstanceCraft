//! Layer-stack commands: add, delete, reorder, group, and per-layer properties, plus undo/redo.

use std::path::PathBuf;

use serde_json::{Map, Value, json};
use substancecraft_doc::{
    BlendMode, Channel, Effect, FillSource, Filter, Generator, Layer, LayerKind, Mask, MaskEffect, MeshMapKind, Projection, ProjectionKind,
};
use substancecraft_raster::RasterImage;

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The layer commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("layer.list", "List Layers", "Window", "no parameters", list),
        CommandSpec::new("layer.select", "Select Layer", "Window", "index: number | null", select),
        CommandSpec::new("layer.add_fill", "Add Fill Layer", "Window", "name?: string, color?: [r,g,b,a], value?: number", add_fill),
        CommandSpec::new("layer.add_paint", "Add Paint Layer", "Window", "name?: string", add_paint),
        CommandSpec::new("layer.add_group", "Add Group", "Window", "name?: string", add_group),
        CommandSpec::new(
            "layer.add_generator",
            "Add Generator",
            "Window",
            "generator: perlin|simplex|voronoi|white, seed?: number, scale?: number",
            add_generator,
        ),
        CommandSpec::new("layer.add_filter", "Add Filter", "Window", "filter: levels|blur|invert|contrast|brightness, radius?: number", add_filter),
        CommandSpec::new("layer.set_effect", "Set Effect Parameters", "Window", "index: number, <param>: number", set_effect),
        CommandSpec::new("layer.delete", "Delete Layer", "Window", "index: number", delete),
        CommandSpec::new("layer.set_visible", "Set Layer Visible", "Window", "index: number, visible: bool", set_visible),
        CommandSpec::new("layer.set_opacity", "Set Layer Opacity", "Window", "index: number, opacity: number (0..1)", set_opacity),
        CommandSpec::new("layer.set_blend", "Set Layer Blend", "Window", "index: number, mode: string", set_blend),
        CommandSpec::new(
            "layer.set_channel_opacity",
            "Set Channel Opacity",
            "Window",
            "index: number, channel: string, opacity: number (0..1)",
            set_channel_opacity,
        ),
        CommandSpec::new("layer.clear_channel_opacity", "Clear Channel Opacity", "Window", "index: number, channel: string", clear_channel_opacity),
        CommandSpec::new("layer.set_name", "Rename Layer", "Window", "index: number, name: string", set_name),
        CommandSpec::new("layer.set_mask", "Set Layer Mask", "Window", "index: number, value: number|null", set_mask),
        CommandSpec::new(
            "layer.add_mask_effect",
            "Add Mask Effect",
            "Window",
            "index: number, effect: fill|generator|filter|mesh_map|image, value?/generator?/filter?/mesh_map?/path?",
            add_mask_effect,
        ),
        CommandSpec::new(
            "layer.set_mask_effect",
            "Set Mask Effect",
            "Window",
            "index: number, effect_index: number, <param>: number",
            set_mask_effect,
        ),
        CommandSpec::new("layer.remove_mask_effect", "Remove Mask Effect", "Window", "index: number, effect_index: number", remove_mask_effect),
        CommandSpec::new("layer.set_parts", "Set Layer Parts", "Window", "index: number, parts: [ids] | null", set_parts),
        CommandSpec::new(
            "layer.add_part_fill",
            "Add Part Fill",
            "Window",
            "part: number, name?: string, color?: [r,g,b,a], value?: number",
            add_part_fill,
        ),
        CommandSpec::new("layer.mask_to_part", "Mask Layer to Part", "Window", "index: number, part: number", mask_to_part),
        CommandSpec::new("layer.import_texture_set", "Import Texture Set", "Window", "paths: [strings]", import_texture_set),
        CommandSpec::new("layer.set_fill_color", "Set Fill Color", "Window", "index: number, color: [r,g,b,a]", set_fill_color),
        CommandSpec::new("layer.set_fill_value", "Set Fill Value", "Window", "index: number, value: number", set_fill_value),
        CommandSpec::new(
            "layer.set_fill_source",
            "Set Fill Source",
            "Window",
            "index: number, source: uniform|procedural|mesh_map, generator?: string, mesh_map?: string",
            set_fill_source,
        ),
        CommandSpec::new("layer.set_fill_image", "Set Fill Image", "Window", "index: number, path: string", set_fill_image),
        CommandSpec::new(
            "layer.set_fill_projection",
            "Set Fill Projection",
            "Window",
            "index: number, kind?: uv|planar|triplanar, scale_x?, scale_y?, rotation?, offset_x?, offset_y?, axis?",
            set_fill_projection,
        ),
        CommandSpec::new("layer.set_fill_strength", "Set Fill Strength", "Window", "index: number, strength: number", set_fill_strength),
        CommandSpec::new("layer.move", "Move Layer", "Window", "index: number, direction: up|down", move_layer),
        CommandSpec::new("layer.group", "Group Layer", "Window", "index: number, name?: string", group_layer),
        CommandSpec::new("edit.undo", "Undo", "Edit", "no parameters", undo).with_shortcut("Ctrl+Z"),
        CommandSpec::new("edit.redo", "Redo", "Edit", "no parameters", redo).with_shortcut("Ctrl+Shift+Z"),
        CommandSpec::new("edit.undo_steps", "Undo Steps", "Edit", "count: number", undo_steps),
        CommandSpec::new("edit.redo_steps", "Redo Steps", "Edit", "count: number", redo_steps),
        CommandSpec::new("edit.history", "History Depth", "Edit", "no parameters", history),
    ]
}

/// Register the layer commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn list(session: &mut Session, _params: &Value) -> CommandResult {
    let Some(stack) = session.layers() else {
        return Ok(CommandOutcome::data(Value::Array(Vec::new())));
    };
    let layers: Vec<Value> = stack
        .layers
        .iter()
        .enumerate()
        .map(|(index, layer)| {
            let mut entry = Map::new();
            entry.insert("index".into(), json!(index));
            entry.insert("name".into(), json!(layer.name));
            entry.insert("type".into(), json!(layer.kind.type_id()));
            entry.insert("visible".into(), json!(layer.visible));
            entry.insert("opacity".into(), json!(layer.opacity));
            entry.insert("blend".into(), json!(layer.blend.id()));
            entry.insert("mask".into(), json!(layer.mask.as_ref().map(Mask::coverage)));
            if !layer.channel_opacity.is_empty() {
                let mut map = Map::new();
                for (channel, value) in &layer.channel_opacity {
                    map.insert(channel.id().to_string(), json!(value));
                }
                entry.insert("channel_opacity".into(), Value::Object(map));
            }
            if let LayerKind::Fill { color, value, source, projection, strength } = &layer.kind {
                entry.insert("color".into(), json!(color));
                entry.insert("value".into(), json!(value));
                entry.insert("source".into(), json!(source.id()));
                entry.insert("strength".into(), json!(strength));
                entry.insert("projection".into(), json!(projection.kind.id()));
            }
            if let LayerKind::Effect { effect } = &layer.kind {
                entry.insert("effect".into(), json!(effect.id()));
                let mut params = Map::new();
                for param in effect.params() {
                    params.insert(param.key.into(), json!(param.value));
                }
                entry.insert("effect_params".into(), Value::Object(params));
            }
            Value::Object(entry)
        })
        .collect();
    Ok(CommandOutcome::data(Value::Array(layers)))
}

fn select(session: &mut Session, params: &Value) -> CommandResult {
    let index = match params.get("index") {
        None | Some(Value::Null) => None,
        Some(value) => Some(value.as_u64().ok_or_else(|| invalid("layer.select", "index must be a number or null"))? as usize),
    };
    if let Some(index) = index {
        let count = session.layers().map(|stack| stack.len()).unwrap_or(0);
        if index >= count {
            return Err(invalid("layer.select", &format!("index {index} is out of range")));
        }
    }
    session.set_active_layer(index);
    Ok(CommandOutcome::message("layer selected"))
}

fn add_fill(session: &mut Session, params: &Value) -> CommandResult {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("Fill").to_string();
    let color = optional_color(params, "color").unwrap_or([0.8, 0.8, 0.8, 1.0]);
    let value = optional_f32(params, "value").unwrap_or(0.5).clamp(0.0, 1.0);
    session.add_layer(Layer::fill(name.clone(), color, value));
    session.set_status(format!("added layer {name}"));
    Ok(CommandOutcome::message(format!("added {name}")))
}

fn add_paint(session: &mut Session, params: &Value) -> CommandResult {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("Paint").to_string();
    session.add_layer(Layer::paint(name.clone()));
    Ok(CommandOutcome::message(format!("added {name}")))
}

fn add_group(session: &mut Session, params: &Value) -> CommandResult {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("Group").to_string();
    session.add_layer(Layer::group(name.clone()));
    Ok(CommandOutcome::message(format!("added {name}")))
}

fn add_generator(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("generator").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_generator", "expected a \"generator\" string"))?;
    let mut generator = Generator::from_id(id).ok_or_else(|| invalid("layer.add_generator", &format!("unknown generator: {id}")))?;
    if let Some(seed) = params.get("seed").and_then(Value::as_u64) {
        generator = with_seed(generator, seed as u32);
    }
    if let Some(scale) = optional_f32(params, "scale") {
        generator = with_scale(generator, scale.max(1.0));
    }
    let label = generator.label().to_string();
    session.add_layer(Layer::effect(label.clone(), Effect::Generator(generator)));
    Ok(CommandOutcome::message(format!("added {label}")))
}

fn add_filter(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("filter").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_filter", "expected a \"filter\" string"))?;
    let mut filter = Filter::from_id(id).ok_or_else(|| invalid("layer.add_filter", &format!("unknown filter: {id}")))?;
    if let Some(radius) = params.get("radius").and_then(Value::as_u64)
        && let Filter::Blur { .. } = filter
    {
        filter = Filter::Blur { radius: (radius as u32).min(64) };
    }
    let label = filter.label().to_string();
    session.add_layer(Layer::effect(label.clone(), Effect::Filter(filter)));
    Ok(CommandOutcome::message(format!("added {label}")))
}

fn set_effect(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_effect", params)?;
    let Some(effect) = session.layers().and_then(|stack| stack.get(index)).and_then(Layer::effect_kind) else {
        return Err(invalid("layer.set_effect", &format!("layer {index} is not an effect layer")));
    };
    let mut updated = effect;
    let mut applied = 0usize;
    for param in effect.params() {
        if let Some(value) = optional_f32(params, param.key) {
            updated = updated.set_param(param.key, value);
            applied += 1;
        }
    }
    if applied == 0 {
        return Err(invalid("layer.set_effect", "expected at least one effect parameter"));
    }
    if session.set_layer_effect(index, updated) {
        Ok(CommandOutcome::message(format!("layer {index} effect set ({applied} param)")))
    } else {
        Err(invalid("layer.set_effect", &format!("no layer at index {index}")))
    }
}

fn with_seed(generator: Generator, seed: u32) -> Generator {
    match generator {
        Generator::Perlin { scale, octaves, .. } => Generator::Perlin { scale, octaves, seed },
        Generator::Simplex { scale, octaves, .. } => Generator::Simplex { scale, octaves, seed },
        Generator::Voronoi { cells, .. } => Generator::Voronoi { cells, seed },
        Generator::White { .. } => Generator::White { seed },
    }
}

fn with_scale(generator: Generator, scale: f32) -> Generator {
    match generator {
        Generator::Perlin { octaves, seed, .. } => Generator::Perlin { scale, octaves, seed },
        Generator::Simplex { octaves, seed, .. } => Generator::Simplex { scale, octaves, seed },
        other => other,
    }
}

fn delete(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.delete", params)?;
    if session.delete_layer(index) {
        Ok(CommandOutcome::message(format!("deleted layer {index}")))
    } else {
        Err(CommandError::InvalidParams { command: "layer.delete".to_string(), message: format!("no layer at index {index}") })
    }
}

fn set_visible(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_visible", params)?;
    let visible = params.get("visible").and_then(Value::as_bool).ok_or_else(|| invalid("layer.set_visible", "expected a \"visible\" bool"))?;
    if session.set_layer_visible(index, visible) {
        Ok(CommandOutcome::message(format!("layer {index} visible: {visible}")))
    } else {
        Err(invalid("layer.set_visible", &format!("no layer at index {index}")))
    }
}

fn set_opacity(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_opacity", params)?;
    let opacity = optional_f32(params, "opacity").ok_or_else(|| invalid("layer.set_opacity", "expected a numeric \"opacity\""))?;
    if session.set_layer_opacity(index, opacity) {
        Ok(CommandOutcome::message(format!("layer {index} opacity: {opacity:.2}")))
    } else {
        Err(invalid("layer.set_opacity", &format!("no layer at index {index}")))
    }
}

fn set_blend(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_blend", params)?;
    let id = params.get("mode").and_then(Value::as_str).ok_or_else(|| invalid("layer.set_blend", "expected a \"mode\" string"))?;
    let blend = BlendMode::from_id(id).ok_or_else(|| invalid("layer.set_blend", &format!("unknown blend mode: {id}")))?;
    if session.set_layer_blend(index, blend) {
        Ok(CommandOutcome::message(format!("layer {index} blend: {id}")))
    } else {
        Err(invalid("layer.set_blend", &format!("no layer at index {index}")))
    }
}

fn set_channel_opacity(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_channel_opacity", params)?;
    let channel = required_channel("layer.set_channel_opacity", params)?;
    let opacity = optional_f32(params, "opacity").ok_or_else(|| invalid("layer.set_channel_opacity", "expected a numeric \"opacity\""))?;
    if session.set_layer_channel_opacity(index, channel, opacity) {
        Ok(CommandOutcome::message(format!("layer {index} {} opacity: {opacity:.2}", channel.id())))
    } else {
        Err(invalid("layer.set_channel_opacity", &format!("no layer at index {index}")))
    }
}

fn clear_channel_opacity(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.clear_channel_opacity", params)?;
    let channel = required_channel("layer.clear_channel_opacity", params)?;
    if session.clear_layer_channel_opacity(index, channel) {
        Ok(CommandOutcome::message(format!("layer {index} {} opacity cleared", channel.id())))
    } else {
        Err(invalid("layer.clear_channel_opacity", &format!("no layer at index {index}")))
    }
}

fn set_name(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_name", params)?;
    let name = params.get("name").and_then(Value::as_str).ok_or_else(|| invalid("layer.set_name", "expected a \"name\" string"))?;
    if name.trim().is_empty() {
        return Err(invalid("layer.set_name", "name must not be empty"));
    }
    if session.set_layer_name(index, name.to_string()) {
        Ok(CommandOutcome::message(format!("layer {index} renamed")))
    } else {
        Err(invalid("layer.set_name", &format!("no layer at index {index}")))
    }
}

fn set_mask(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_mask", params)?;
    let mask = match params.get("value") {
        None | Some(Value::Null) => None,
        Some(value) => {
            let value = value.as_f64().ok_or_else(|| invalid("layer.set_mask", "expected a numeric or null \"value\""))?;
            Some(Mask::Constant(value as f32))
        }
    };
    if session.set_layer_mask(index, mask) {
        Ok(CommandOutcome::message(format!("layer {index} mask set")))
    } else {
        Err(invalid("layer.set_mask", &format!("no layer at index {index}")))
    }
}

fn set_fill_color(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_fill_color", params)?;
    let color = required_color("layer.set_fill_color", params)?;
    if session.set_fill_color(index, color) {
        Ok(CommandOutcome::message(format!("layer {index} color set")))
    } else {
        Err(invalid("layer.set_fill_color", &format!("layer {index} is not a fill")))
    }
}

fn set_fill_value(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_fill_value", params)?;
    let value = optional_f32(params, "value").ok_or_else(|| invalid("layer.set_fill_value", "expected a numeric \"value\""))?;
    if session.set_fill_value(index, value.clamp(0.0, 1.0)) {
        Ok(CommandOutcome::message(format!("layer {index} value set")))
    } else {
        Err(invalid("layer.set_fill_value", &format!("layer {index} is not a fill")))
    }
}

fn set_fill_source(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_fill_source", params)?;
    let id = params.get("source").and_then(Value::as_str).ok_or_else(|| invalid("layer.set_fill_source", "expected a \"source\" string"))?;
    let source = match id {
        "uniform" => FillSource::Uniform,
        "procedural" => {
            let generator_id = params
                .get("generator")
                .and_then(Value::as_str)
                .ok_or_else(|| invalid("layer.set_fill_source", "procedural needs a \"generator\""))?;
            let generator =
                Generator::from_id(generator_id).ok_or_else(|| invalid("layer.set_fill_source", &format!("unknown generator: {generator_id}")))?;
            FillSource::Procedural { generator }
        }
        "mesh_map" => {
            let map_id =
                params.get("mesh_map").and_then(Value::as_str).ok_or_else(|| invalid("layer.set_fill_source", "mesh_map needs a \"mesh_map\""))?;
            let kind = MeshMapKind::from_id(map_id).ok_or_else(|| invalid("layer.set_fill_source", &format!("unknown mesh map: {map_id}")))?;
            FillSource::MeshMap { kind }
        }
        other => return Err(invalid("layer.set_fill_source", &format!("unknown source: {other}"))),
    };
    if session.set_fill_source(index, source) {
        Ok(CommandOutcome::message(format!("layer {index} source: {id}")))
    } else {
        Err(invalid("layer.set_fill_source", &format!("layer {index} is not a fill")))
    }
}

fn set_fill_image(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_fill_image", params)?;
    let path = params.get("path").and_then(Value::as_str).ok_or_else(|| invalid("layer.set_fill_image", "expected a \"path\" string"))?;
    let loaded = substancecraft_codecs::load_rgba8(path).map_err(|err| invalid("layer.set_fill_image", &format!("could not load image: {err}")))?;
    if loaded.width == 0 || loaded.height == 0 {
        return Err(invalid("layer.set_fill_image", "the image is empty"));
    }
    let mut image = RasterImage::new(loaded.width, loaded.height);
    for (slot, value) in image.pixels.iter_mut().zip(loaded.pixels) {
        *slot = value;
    }
    if session.set_fill_source(index, FillSource::Image { image }) {
        Ok(CommandOutcome::message(format!("layer {index} image set")))
    } else {
        Err(invalid("layer.set_fill_image", &format!("layer {index} is not a fill")))
    }
}

fn set_fill_projection(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_fill_projection", params)?;
    let mut projection = session
        .layers()
        .and_then(|stack| stack.get(index))
        .and_then(|layer| match &layer.kind {
            LayerKind::Fill { projection, .. } => Some(*projection),
            _ => None,
        })
        .ok_or_else(|| invalid("layer.set_fill_projection", &format!("layer {index} is not a fill")))?;
    if let Some(kind) = params.get("kind").and_then(Value::as_str) {
        projection.kind =
            ProjectionKind::from_id(kind).ok_or_else(|| invalid("layer.set_fill_projection", &format!("unknown projection: {kind}")))?;
    }
    if let Some(value) = optional_f32(params, "scale_x") {
        projection.scale[0] = value.max(1e-3);
    }
    if let Some(value) = optional_f32(params, "scale_y") {
        projection.scale[1] = value.max(1e-3);
    }
    if let Some(value) = optional_f32(params, "rotation") {
        projection.rotation = value;
    }
    if let Some(value) = optional_f32(params, "offset_x") {
        projection.offset[0] = value;
    }
    if let Some(value) = optional_f32(params, "offset_y") {
        projection.offset[1] = value;
    }
    if let Some(value) = params.get("axis").and_then(Value::as_u64) {
        projection.axis = value.min(2) as u8;
    }
    if session.set_fill_projection(index, projection) {
        Ok(CommandOutcome::message(format!("layer {index} projection set")))
    } else {
        Err(invalid("layer.set_fill_projection", &format!("layer {index} is not a fill")))
    }
}

fn set_fill_strength(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_fill_strength", params)?;
    let strength = optional_f32(params, "strength").ok_or_else(|| invalid("layer.set_fill_strength", "expected a numeric \"strength\""))?;
    if session.set_fill_strength(index, strength) {
        Ok(CommandOutcome::message(format!("layer {index} strength: {strength:.2}")))
    } else {
        Err(invalid("layer.set_fill_strength", &format!("layer {index} is not a fill")))
    }
}

fn add_mask_effect(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.add_mask_effect", params)?;
    let kind = params.get("effect").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_mask_effect", "expected an \"effect\" string"))?;
    let invert = params.get("invert").and_then(Value::as_bool).unwrap_or(false);
    let effect = match kind {
        "fill" => MaskEffect::Fill { value: optional_f32(params, "value").unwrap_or(1.0).clamp(0.0, 1.0) },
        "generator" => {
            let id =
                params.get("generator").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_mask_effect", "generator needs a \"generator\""))?;
            MaskEffect::Generator(Generator::from_id(id).ok_or_else(|| invalid("layer.add_mask_effect", &format!("unknown generator: {id}")))?)
        }
        "filter" => {
            let id = params.get("filter").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_mask_effect", "filter needs a \"filter\""))?;
            MaskEffect::Filter(Filter::from_id(id).ok_or_else(|| invalid("layer.add_mask_effect", &format!("unknown filter: {id}")))?)
        }
        "mesh_map" => {
            let id =
                params.get("mesh_map").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_mask_effect", "mesh_map needs a \"mesh_map\""))?;
            let kind = MeshMapKind::from_id(id).ok_or_else(|| invalid("layer.add_mask_effect", &format!("unknown mesh map: {id}")))?;
            MaskEffect::MeshMap { kind, invert }
        }
        "image" => {
            let path = params.get("path").and_then(Value::as_str).ok_or_else(|| invalid("layer.add_mask_effect", "image needs a \"path\""))?;
            MaskEffect::Image { image: load_mask_image(path)?, projection: Projection::default(), invert }
        }
        other => return Err(invalid("layer.add_mask_effect", &format!("unknown effect: {other}"))),
    };
    if session.add_mask_effect(index, effect) {
        Ok(CommandOutcome::message(format!("layer {index} mask effect added")))
    } else {
        Err(invalid("layer.add_mask_effect", &format!("no layer at index {index}")))
    }
}

fn set_mask_effect(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_mask_effect", params)?;
    let effect_index = params
        .get("effect_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .ok_or_else(|| invalid("layer.set_mask_effect", "expected an \"effect_index\" number"))?;
    let current = session
        .layers()
        .and_then(|stack| stack.get(index))
        .and_then(|layer| layer.mask.as_ref())
        .and_then(|mask| match mask {
            Mask::Stack { effects } => effects.get(effect_index),
            Mask::Constant(_) => None,
        })
        .cloned()
        .ok_or_else(|| invalid("layer.set_mask_effect", "no such mask effect"))?;
    let updated = match current {
        MaskEffect::Fill { value } => MaskEffect::Fill { value: optional_f32(params, "value").unwrap_or(value).clamp(0.0, 1.0) },
        MaskEffect::Generator(mut generator) => {
            for param in generator.params() {
                if let Some(value) = optional_f32(params, param.key) {
                    generator = generator.set_param(param.key, value);
                }
            }
            MaskEffect::Generator(generator)
        }
        MaskEffect::Filter(mut filter) => {
            for param in filter.params() {
                if let Some(value) = optional_f32(params, param.key) {
                    filter = filter.set_param(param.key, value);
                }
            }
            MaskEffect::Filter(filter)
        }
        MaskEffect::MeshMap { kind, invert } => MaskEffect::MeshMap { kind, invert: params.get("invert").and_then(Value::as_bool).unwrap_or(invert) },
        MaskEffect::Image { image, projection, invert } => {
            MaskEffect::Image { image, projection, invert: params.get("invert").and_then(Value::as_bool).unwrap_or(invert) }
        }
    };
    if session.set_mask_effect(index, effect_index, updated) {
        Ok(CommandOutcome::message(format!("layer {index} mask effect {effect_index} set")))
    } else {
        Err(invalid("layer.set_mask_effect", &format!("no layer at index {index}")))
    }
}

fn remove_mask_effect(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.remove_mask_effect", params)?;
    let effect_index = params
        .get("effect_index")
        .and_then(Value::as_u64)
        .map(|value| value as usize)
        .ok_or_else(|| invalid("layer.remove_mask_effect", "expected an \"effect_index\" number"))?;
    if session.remove_mask_effect(index, effect_index) {
        Ok(CommandOutcome::message(format!("layer {index} mask effect {effect_index} removed")))
    } else {
        Err(invalid("layer.remove_mask_effect", &format!("no layer at index {index}")))
    }
}

/// Load an image for a mask effect.
fn load_mask_image(path: &str) -> Result<RasterImage, CommandError> {
    let loaded = substancecraft_codecs::load_rgba8(path).map_err(|err| invalid("layer.add_mask_effect", &format!("could not load image: {err}")))?;
    if loaded.width == 0 || loaded.height == 0 {
        return Err(invalid("layer.add_mask_effect", "the image is empty"));
    }
    let mut image = RasterImage::new(loaded.width, loaded.height);
    for (slot, value) in image.pixels.iter_mut().zip(loaded.pixels) {
        *slot = value;
    }
    Ok(image)
}

fn set_parts(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.set_parts", params)?;
    let parts = match params.get("parts") {
        None | Some(Value::Null) => None,
        Some(Value::Array(values)) => {
            let mut ids = Vec::with_capacity(values.len());
            for value in values {
                let id = value.as_u64().ok_or_else(|| invalid("layer.set_parts", "parts must be integers"))?;
                ids.push(id as u32);
            }
            ids.sort_unstable();
            ids.dedup();
            Some(ids)
        }
        _ => return Err(invalid("layer.set_parts", "expected a \"parts\" array or null")),
    };
    if session.set_layer_parts(index, parts) {
        Ok(CommandOutcome::message(format!("layer {index} parts set")))
    } else {
        Err(invalid("layer.set_parts", &format!("no layer at index {index}")))
    }
}

fn add_part_fill(session: &mut Session, params: &Value) -> CommandResult {
    let part = params.get("part").and_then(Value::as_u64).ok_or_else(|| invalid("layer.add_part_fill", "expected a \"part\" number"))? as u32;
    let name = params.get("name").and_then(Value::as_str).map(str::to_string).unwrap_or_else(|| format!("Part {part}"));
    let color = optional_color(params, "color").unwrap_or([0.8, 0.8, 0.8, 1.0]);
    let value = optional_f32(params, "value").unwrap_or(0.5).clamp(0.0, 1.0);
    let mut layer = Layer::fill(name.clone(), color, value);
    layer.parts = Some(vec![part]);
    session.add_layer(layer);
    Ok(CommandOutcome::message(format!("added {name}")))
}

fn mask_to_part(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.mask_to_part", params)?;
    let part = params.get("part").and_then(Value::as_u64).ok_or_else(|| invalid("layer.mask_to_part", "expected a \"part\" number"))? as u32;
    if session.set_layer_parts(index, Some(vec![part])) {
        Ok(CommandOutcome::message(format!("layer {index} masked to part {part}")))
    } else {
        Err(invalid("layer.mask_to_part", &format!("no layer at index {index}")))
    }
}

fn import_texture_set(session: &mut Session, params: &Value) -> CommandResult {
    let paths: Vec<PathBuf> = params
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| invalid("layer.import_texture_set", "expected a \"paths\" array"))?
        .iter()
        .filter_map(Value::as_str)
        .map(PathBuf::from)
        .collect();
    if paths.is_empty() {
        return Err(invalid("layer.import_texture_set", "paths must not be empty"));
    }
    let matched =
        substancecraft_io::import_texture_set(&paths).map_err(|err| invalid("layer.import_texture_set", &format!("import failed: {err}")))?;
    if matched.is_empty() {
        return Err(invalid("layer.import_texture_set", "no files matched a channel"));
    }
    let count = session.import_texture_channels(matched);
    session.set_status(format!("imported {count} texture maps"));
    Ok(CommandOutcome::message(format!("imported {count} channels")))
}

fn move_layer(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.move", params)?;
    let direction = params.get("direction").and_then(Value::as_str).ok_or_else(|| invalid("layer.move", "expected a \"direction\" of up|down"))?;
    let up = match direction {
        "up" => true,
        "down" => false,
        other => return Err(invalid("layer.move", &format!("unknown direction: {other}"))),
    };
    if session.move_layer(index, up) {
        Ok(CommandOutcome::message(format!("moved layer {index} {direction}")))
    } else {
        Err(invalid("layer.move", &format!("layer {index} cannot move {direction}")))
    }
}

fn group_layer(session: &mut Session, params: &Value) -> CommandResult {
    let index = required_index("layer.group", params)?;
    let name = params.get("name").and_then(Value::as_str).unwrap_or("Group").to_string();
    if session.group_layer(index, name.clone()) {
        Ok(CommandOutcome::message(format!("grouped layer {index} as {name}")))
    } else {
        Err(invalid("layer.group", &format!("no layer at index {index}")))
    }
}

fn undo(session: &mut Session, _params: &Value) -> CommandResult {
    if session.undo() {
        session.set_status("undo");
        Ok(CommandOutcome::message("undo"))
    } else {
        Err(CommandError::Disabled("nothing to undo".to_string()))
    }
}

fn redo(session: &mut Session, _params: &Value) -> CommandResult {
    if session.redo() {
        session.set_status("redo");
        Ok(CommandOutcome::message("redo"))
    } else {
        Err(CommandError::Disabled("nothing to redo".to_string()))
    }
}

fn undo_steps(session: &mut Session, params: &Value) -> CommandResult {
    let count = params.get("count").and_then(Value::as_u64).unwrap_or(1).min(1000) as usize;
    let applied = session.undo_steps(count);
    session.set_status(format!("undo x{applied}"));
    Ok(CommandOutcome::message(format!("undid {applied} steps")))
}

fn redo_steps(session: &mut Session, params: &Value) -> CommandResult {
    let count = params.get("count").and_then(Value::as_u64).unwrap_or(1).min(1000) as usize;
    let applied = session.redo_steps(count);
    session.set_status(format!("redo x{applied}"));
    Ok(CommandOutcome::message(format!("redid {applied} steps")))
}

fn history(session: &mut Session, _params: &Value) -> CommandResult {
    Ok(CommandOutcome::data(json!({ "undo": session.undo_depth(), "redo": session.redo_depth() })))
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

fn required_index(command: &str, params: &Value) -> Result<usize, CommandError> {
    params.get("index").and_then(Value::as_u64).map(|value| value as usize).ok_or_else(|| invalid(command, "expected an \"index\" number"))
}

fn optional_f32(params: &Value, key: &str) -> Option<f32> {
    params.get(key).and_then(Value::as_f64).map(|value| value as f32)
}

fn required_channel(command: &str, params: &Value) -> Result<Channel, CommandError> {
    let id = params.get("channel").and_then(Value::as_str).ok_or_else(|| invalid(command, "expected a \"channel\" string"))?;
    Channel::from_id(id).ok_or_else(|| invalid(command, &format!("unknown channel: {id}")))
}

fn optional_color(params: &Value, key: &str) -> Option<[f32; 4]> {
    let array = params.get(key)?.as_array()?;
    if array.len() < 3 {
        return None;
    }
    let mut color = [0.0f32, 0.0, 0.0, 1.0];
    for (index, value) in array.iter().take(4).enumerate() {
        color[index] = value.as_f64()? as f32;
    }
    Some(color)
}

fn required_color(command: &str, params: &Value) -> Result<[f32; 4], CommandError> {
    optional_color(params, "color").ok_or_else(|| invalid(command, "expected a \"color\" array of 3 or 4 numbers"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_list_layers() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        add_group(&mut session, &Value::Null).expect("group");
        let data = list(&mut session, &Value::Null).expect("list").data.expect("data");
        assert_eq!(data.as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn edits_validate_indices_and_params() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        assert!(set_opacity(&mut session, &json!({ "index": 0, "opacity": 0.5 })).is_ok());
        assert!(set_opacity(&mut session, &json!({ "index": 9, "opacity": 0.5 })).is_err());
        assert!(set_blend(&mut session, &json!({ "index": 0, "mode": "warp" })).is_err());
        assert!(set_blend(&mut session, &json!({ "index": 0, "mode": "multiply" })).is_ok());
        assert!(set_fill_color(&mut session, &json!({ "index": 0, "color": [1, 0, 0, 1] })).is_ok());
        assert!(set_fill_value(&mut session, &json!({ "index": 0, "value": 0.2 })).is_ok());
    }

    #[test]
    fn per_channel_opacity_round_trips() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        assert!(set_channel_opacity(&mut session, &json!({ "index": 0, "channel": "roughness", "opacity": 0.25 })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(0)).expect("layer");
        assert!((layer.channel_opacity(Channel::Roughness) - 0.25).abs() < 1e-6);
        assert!(set_channel_opacity(&mut session, &json!({ "index": 0, "channel": "nope", "opacity": 0.5 })).is_err());
        assert!(clear_channel_opacity(&mut session, &json!({ "index": 0, "channel": "roughness" })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(0)).expect("layer");
        assert_eq!(layer.channel_opacity(Channel::Roughness), 1.0);
    }

    #[test]
    fn fill_source_projection_and_strength_round_trip() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        assert!(set_fill_source(&mut session, &json!({ "index": 0, "source": "procedural", "generator": "perlin" })).is_ok());
        assert!(set_fill_projection(&mut session, &json!({ "index": 0, "kind": "triplanar", "scale_x": 2.0, "axis": 2 })).is_ok());
        assert!(set_fill_strength(&mut session, &json!({ "index": 0, "strength": 0.5 })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(0)).expect("layer");
        assert!(matches!(layer.fill_source(), Some(FillSource::Procedural { .. })));
        assert!((layer.fill_strength() - 0.5).abs() < 1e-6);
        assert!(matches!(&layer.kind, LayerKind::Fill { projection, .. } if projection.kind == ProjectionKind::TriPlanar && projection.axis == 2));
        assert!(set_fill_source(&mut session, &json!({ "index": 0, "source": "nope" })).is_err());
        assert!(set_fill_image(&mut session, &json!({ "index": 0, "path": "/does/not/exist.png" })).is_err());
    }

    #[test]
    fn mask_effect_stack_round_trips() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        assert!(add_mask_effect(&mut session, &json!({ "index": 0, "effect": "mesh_map", "mesh_map": "curvature" })).is_ok());
        assert!(add_mask_effect(&mut session, &json!({ "index": 0, "effect": "filter", "filter": "invert" })).is_ok());
        let mask = session.layers().and_then(|stack| stack.get(0)).and_then(|layer| layer.mask.as_ref()).expect("mask");
        assert!(mask.is_dynamic());
        assert!(matches!(mask, Mask::Stack { effects } if effects.len() == 2));
        assert!(set_mask_effect(&mut session, &json!({ "index": 0, "effect_index": 0, "invert": true })).is_ok());
        assert!(remove_mask_effect(&mut session, &json!({ "index": 0, "effect_index": 1 })).is_ok());
        let mask = session.layers().and_then(|stack| stack.get(0)).and_then(|layer| layer.mask.as_ref()).expect("mask");
        assert!(matches!(mask, Mask::Stack { effects } if effects.len() == 1));
        assert!(add_mask_effect(&mut session, &json!({ "index": 0, "effect": "nope" })).is_err());
    }

    #[test]
    fn layer_parts_restriction_round_trips() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        assert!(set_parts(&mut session, &json!({ "index": 0, "parts": [1, 0, 1] })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(0)).expect("layer");
        assert_eq!(layer.parts.as_deref(), Some(&[0u32, 1][..]));
        assert!(set_parts(&mut session, &json!({ "index": 0, "parts": null })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(0)).expect("layer");
        assert!(layer.parts.is_none());
        assert!(set_parts(&mut session, &json!({ "index": 0, "parts": "nope" })).is_err());
    }

    #[test]
    fn part_fill_and_mask_commands() {
        let mut session = Session::new("test");
        assert!(add_part_fill(&mut session, &json!({ "part": 2, "name": "Blade" })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(0)).expect("layer");
        assert_eq!(layer.parts.as_deref(), Some(&[2u32][..]));
        assert_eq!(layer.name, "Blade");
        add_fill(&mut session, &Value::Null).expect("add");
        assert!(mask_to_part(&mut session, &json!({ "index": 1, "part": 3 })).is_ok());
        let layer = session.layers().and_then(|stack| stack.get(1)).expect("layer");
        assert_eq!(layer.parts.as_deref(), Some(&[3u32][..]));
    }

    #[test]
    fn importing_a_texture_set_adds_channel_fills() {
        let dir = std::env::temp_dir().join(format!("sc_texset_{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("dir");
        let image = RasterImage::filled(2, 2, [0.5, 0.5, 0.5, 1.0]);
        let base = dir.join("body_basecolor.png");
        let rough = dir.join("body_roughness.png");
        substancecraft_codecs::save_png(&base, 2, 2, &image.to_rgba8_linear()).expect("save base");
        substancecraft_codecs::save_png(&rough, 2, 2, &image.to_rgba8_linear()).expect("save rough");

        let mut session = Session::new("test");
        let before = session.layers().map(|stack| stack.len()).unwrap_or(0);
        assert!(import_texture_set(&mut session, &json!({ "paths": [base.to_string_lossy(), rough.to_string_lossy()] })).is_ok());
        assert_eq!(session.layers().map(|stack| stack.len()).unwrap_or(0), before + 2);
        assert!(import_texture_set(&mut session, &json!({ "paths": [] })).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn history_depth_and_multi_step_jumps() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("a");
        add_fill(&mut session, &Value::Null).expect("b");
        assert!(session.undo_depth() >= 2);
        assert!(undo_steps(&mut session, &json!({ "count": 2 })).is_ok());
        assert_eq!(session.undo_depth(), 0);
        let data = history(&mut session, &Value::Null).expect("history").data.expect("data");
        assert_eq!(data.get("redo").and_then(Value::as_u64), Some(2));
        assert!(redo_steps(&mut session, &json!({ "count": 1 })).is_ok());
        assert!(session.undo_depth() >= 1);
    }

    #[test]
    fn undo_and_redo_restore_the_stack() {
        let mut session = Session::new("test");
        add_fill(&mut session, &Value::Null).expect("add");
        assert_eq!(session.layers().map(|stack| stack.len()), Some(1));
        undo(&mut session, &Value::Null).expect("undo");
        assert_eq!(session.layers().map(|stack| stack.len()), Some(0));
        redo(&mut session, &Value::Null).expect("redo");
        assert_eq!(session.layers().map(|stack| stack.len()), Some(1));
        assert!(undo(&mut session, &Value::Null).is_ok());
        assert!(redo(&mut session, &Value::Null).is_ok());
    }

    #[test]
    fn undo_on_empty_is_a_graceful_error() {
        let mut session = Session::new("test");
        assert!(matches!(undo(&mut session, &Value::Null), Err(CommandError::Disabled(_))));
    }

    #[test]
    fn masking_and_moving() {
        let mut session = Session::new("test");
        add_fill(&mut session, &json!({ "name": "a" })).expect("a");
        add_fill(&mut session, &json!({ "name": "b" })).expect("b");
        assert!(set_mask(&mut session, &json!({ "index": 0, "value": 0.5 })).is_ok());
        assert!(move_layer(&mut session, &json!({ "index": 1, "direction": "down" })).is_ok());
        assert!(move_layer(&mut session, &json!({ "index": 0, "direction": "down" })).is_err());
    }

    #[test]
    fn adding_generators_and_filters() {
        let mut session = Session::new("test");
        assert!(add_generator(&mut session, &json!({ "generator": "perlin", "seed": 7 })).is_ok());
        assert!(add_filter(&mut session, &json!({ "filter": "invert" })).is_ok());
        let stack = session.layers().expect("layers");
        assert!(matches!(stack.get(0).and_then(|l| l.effect_kind()), Some(Effect::Generator(_))));
        assert!(matches!(stack.get(1).and_then(|l| l.effect_kind()), Some(Effect::Filter(_))));
        assert!(add_generator(&mut session, &json!({ "generator": "plasma" })).is_err());
        assert!(add_filter(&mut session, &json!({ "filter": "warp" })).is_err());
    }

    #[test]
    fn editing_effect_parameters_clamps_and_validates() {
        let mut session = Session::new("test");
        add_generator(&mut session, &json!({ "generator": "perlin" })).expect("generator");
        assert!(set_effect(&mut session, &json!({ "index": 0, "scale": 16.0, "octaves": 2 })).is_ok());
        let effect = session.layers().and_then(|stack| stack.get(0)).and_then(|l| l.effect_kind()).expect("effect");
        assert_eq!(effect, Effect::Generator(Generator::Perlin { scale: 16.0, octaves: 2, seed: 0 }));
        assert!(set_effect(&mut session, &json!({ "index": 0, "scale": 9999.0 })).is_ok());
        let effect = session.layers().and_then(|stack| stack.get(0)).and_then(|l| l.effect_kind()).expect("effect");
        assert_eq!(effect, Effect::Generator(Generator::Perlin { scale: 256.0, octaves: 2, seed: 0 }));
        assert!(set_effect(&mut session, &json!({ "index": 0 })).is_err());
        assert!(set_effect(&mut session, &json!({ "index": 9, "scale": 2.0 })).is_err());
        add_fill(&mut session, &Value::Null).expect("fill");
        assert!(set_effect(&mut session, &json!({ "index": 1, "scale": 2.0 })).is_err());
    }

    #[test]
    fn listing_effects_includes_parameters() {
        let mut session = Session::new("test");
        add_generator(&mut session, &json!({ "generator": "voronoi", "seed": 3 })).expect("generator");
        let data = list(&mut session, &Value::Null).expect("list").data.expect("data");
        let entry = data.as_array().and_then(|layers| layers.first()).expect("entry");
        assert_eq!(entry.get("effect").and_then(Value::as_str), Some("voronoi"));
        assert_eq!(entry.get("effect_params").and_then(|p| p.get("seed")).and_then(Value::as_f64), Some(3.0));
    }
}
