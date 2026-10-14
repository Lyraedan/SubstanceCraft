//! Layer-stack commands: add, delete, reorder, group, and per-layer properties, plus undo/redo.

use serde_json::{Map, Value, json};
use substancecraft_doc::{BlendMode, Effect, Filter, Generator, Layer, LayerKind, Mask};

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
        CommandSpec::new("layer.set_name", "Rename Layer", "Window", "index: number, name: string", set_name),
        CommandSpec::new("layer.set_mask", "Set Layer Mask", "Window", "index: number, value: number|null", set_mask),
        CommandSpec::new("layer.set_fill_color", "Set Fill Color", "Window", "index: number, color: [r,g,b,a]", set_fill_color),
        CommandSpec::new("layer.set_fill_value", "Set Fill Value", "Window", "index: number, value: number", set_fill_value),
        CommandSpec::new("layer.move", "Move Layer", "Window", "index: number, direction: up|down", move_layer),
        CommandSpec::new("layer.group", "Group Layer", "Window", "index: number, name?: string", group_layer),
        CommandSpec::new("edit.undo", "Undo", "Edit", "no parameters", undo).with_shortcut("Ctrl+Z"),
        CommandSpec::new("edit.redo", "Redo", "Edit", "no parameters", redo).with_shortcut("Ctrl+Shift+Z"),
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
            entry.insert("mask".into(), json!(layer.mask.map(Mask::coverage)));
            if let LayerKind::Fill { color, value } = &layer.kind {
                entry.insert("color".into(), json!(color));
                entry.insert("value".into(), json!(value));
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

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

fn required_index(command: &str, params: &Value) -> Result<usize, CommandError> {
    params.get("index").and_then(Value::as_u64).map(|value| value as usize).ok_or_else(|| invalid(command, "expected an \"index\" number"))
}

fn optional_f32(params: &Value, key: &str) -> Option<f32> {
    params.get(key).and_then(Value::as_f64).map(|value| value as f32)
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
