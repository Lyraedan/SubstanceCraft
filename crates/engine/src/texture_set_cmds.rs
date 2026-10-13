//! Texture-set commands: list, select, rename, channels, resolution and UV tiles.

use serde_json::{Map, Value, json};
use substancecraft_doc::{Channel, Shader, UvTile};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The texture-set commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("texture_set.list", "List Texture Sets", "Window", "no parameters", list),
        CommandSpec::new("texture_set.select", "Select Texture Set", "Window", "index: number, or name: string", select),
        CommandSpec::new("texture_set.rename", "Rename Texture Set", "Window", "name: string (renames the active set)", rename),
        CommandSpec::new("texture_set.set_resolution", "Set Texture Resolution", "Window", "resolution: number (32..8192)", set_resolution),
        CommandSpec::new("texture_set.set_channel", "Set Channel", "Window", "channel: string, enabled: bool", set_channel),
        CommandSpec::new("texture_set.add_uv_tile", "Add UV Tile", "Window", "udim: number (>= 1001)", add_uv_tile),
        CommandSpec::new("texture_set.remove_uv_tile", "Remove UV Tile", "Window", "udim: number (>= 1001)", remove_uv_tile),
        CommandSpec::new("texture_set.set_solo", "Solo Texture Set", "Window", "enabled: bool", set_solo),
        CommandSpec::new("texture_set.set_shader", "Set Shader", "Window", "shader: openpbr | metal_rough | spec_gloss", set_shader),
        CommandSpec::new("shader.set", "Set Shader Parameters", "Window", "metallic?, roughness?, specular?, tint?: [r,g,b]", set_shader_params),
    ]
}

/// Register the texture-set commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn list(session: &mut Session, _params: &Value) -> CommandResult {
    let project = session.project();
    let sets: Vec<Value> = project
        .texture_sets
        .iter()
        .enumerate()
        .map(|(index, set)| {
            let mut entry = Map::new();
            entry.insert("index".into(), json!(index));
            entry.insert("name".into(), json!(set.name));
            entry.insert("resolution".into(), json!(set.resolution));
            entry.insert("active".into(), json!(index == project.active_texture_set));
            entry.insert("channels".into(), json!(set.channels.iter().map(|c| c.id()).collect::<Vec<_>>()));
            entry.insert("materials".into(), json!(set.materials));
            entry.insert("udim".into(), json!(set.uv_tiles.iter().map(|t| t.udim()).collect::<Vec<_>>()));
            entry.insert("triangles".into(), json!(project.triangles_for_set(index)));
            Value::Object(entry)
        })
        .collect();
    Ok(CommandOutcome::data(Value::Array(sets)))
}

fn select(session: &mut Session, params: &Value) -> CommandResult {
    let index = if let Some(index) = params.get("index").and_then(Value::as_u64) {
        index as usize
    } else if let Some(name) = params.get("name").and_then(Value::as_str) {
        session.texture_sets().iter().position(|set| set.name == name).ok_or_else(|| CommandError::InvalidParams {
            command: "texture_set.select".to_string(),
            message: format!("no texture set named {name}"),
        })?
    } else {
        return Err(CommandError::InvalidParams {
            command: "texture_set.select".to_string(),
            message: "expected an \"index\" number or a \"name\" string".to_string(),
        });
    };

    if session.select_texture_set(index) {
        let name = session.texture_sets().get(index).map(|set| set.name.clone()).unwrap_or_default();
        session.set_status(format!("texture set: {name}"));
        Ok(CommandOutcome::message(format!("selected {name}")))
    } else {
        Err(CommandError::InvalidParams { command: "texture_set.select".to_string(), message: format!("index {index} is out of range") })
    }
}

fn rename(session: &mut Session, params: &Value) -> CommandResult {
    let name = params.get("name").and_then(Value::as_str).ok_or_else(|| CommandError::InvalidParams {
        command: "texture_set.rename".to_string(),
        message: "expected a \"name\" string".to_string(),
    })?;
    if name.trim().is_empty() {
        return Err(CommandError::InvalidParams { command: "texture_set.rename".to_string(), message: "name must not be empty".to_string() });
    }
    session.rename_texture_set(name.to_string());
    session.set_status(format!("renamed texture set to {name}"));
    Ok(CommandOutcome::message(format!("renamed to {name}")))
}

fn set_resolution(session: &mut Session, params: &Value) -> CommandResult {
    let resolution = params.get("resolution").and_then(Value::as_u64).ok_or_else(|| CommandError::InvalidParams {
        command: "texture_set.set_resolution".to_string(),
        message: "expected a numeric \"resolution\"".to_string(),
    })?;
    if !(32..=8192).contains(&resolution) {
        return Err(CommandError::InvalidParams {
            command: "texture_set.set_resolution".to_string(),
            message: "resolution must be in 32..=8192".to_string(),
        });
    }
    session.set_texture_resolution(resolution as u32);
    session.set_status(format!("texture resolution: {resolution}"));
    Ok(CommandOutcome::message(format!("resolution {resolution}")))
}

fn set_channel(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("channel").and_then(Value::as_str).ok_or_else(|| CommandError::InvalidParams {
        command: "texture_set.set_channel".to_string(),
        message: "expected a \"channel\" string".to_string(),
    })?;
    let channel = Channel::from_id(id)
        .ok_or_else(|| CommandError::InvalidParams { command: "texture_set.set_channel".to_string(), message: format!("unknown channel: {id}") })?;
    let enabled = params.get("enabled").and_then(Value::as_bool).ok_or_else(|| CommandError::InvalidParams {
        command: "texture_set.set_channel".to_string(),
        message: "expected an \"enabled\" bool".to_string(),
    })?;
    session.set_texture_channel(channel, enabled);
    session.set_status(format!("{} {}", channel.label(), if enabled { "on" } else { "off" }));
    Ok(CommandOutcome::message(format!("{} {}", channel.label(), if enabled { "on" } else { "off" })))
}

fn add_uv_tile(session: &mut Session, params: &Value) -> CommandResult {
    let tile = parse_udim("texture_set.add_uv_tile", params)?;
    session.add_uv_tile(tile);
    session.set_status(format!("added UDIM {}", tile.udim()));
    Ok(CommandOutcome::message(format!("added UDIM {}", tile.udim())))
}

fn remove_uv_tile(session: &mut Session, params: &Value) -> CommandResult {
    let tile = parse_udim("texture_set.remove_uv_tile", params)?;
    let before = session.active_texture_set();
    let count = session.texture_sets().get(before).map(|set| set.uv_tiles.len()).unwrap_or(0);
    session.remove_uv_tile(tile);
    Ok(CommandOutcome::message(format!("removed UDIM {} (had {count})", tile.udim())))
}

fn set_solo(session: &mut Session, params: &Value) -> CommandResult {
    let enabled = params.get("enabled").and_then(Value::as_bool).ok_or_else(|| CommandError::InvalidParams {
        command: "texture_set.set_solo".to_string(),
        message: "expected an \"enabled\" bool".to_string(),
    })?;
    session.settings_mut().isolate_active_set = enabled;
    session.touch();
    Ok(CommandOutcome::message(if enabled { "solo on" } else { "solo off" }))
}

fn parse_udim(command: &str, params: &Value) -> Result<UvTile, CommandError> {
    let udim = params
        .get("udim")
        .and_then(Value::as_u64)
        .ok_or_else(|| CommandError::InvalidParams { command: command.to_string(), message: "expected a numeric \"udim\"".to_string() })?;
    UvTile::from_udim(udim as u32)
        .ok_or_else(|| CommandError::InvalidParams { command: command.to_string(), message: format!("invalid UDIM: {udim}") })
}

fn set_shader(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("shader").and_then(Value::as_str).ok_or_else(|| invalid("texture_set.set_shader", "expected a \"shader\" string"))?;
    let shader = Shader::from_id(id).ok_or_else(|| invalid("texture_set.set_shader", &format!("unknown shader: {id}")))?;
    session.set_shader(shader);
    session.set_status(format!("shader: {}", shader.label()));
    Ok(CommandOutcome::message(shader.label()))
}

fn set_shader_params(session: &mut Session, params: &Value) -> CommandResult {
    let mut current = session.project().active_set().map(|set| set.shader_params).unwrap_or_default();
    if let Some(value) = params.get("metallic").and_then(Value::as_f64) {
        current.metallic = (value as f32).clamp(0.0, 1.0);
    }
    if let Some(value) = params.get("roughness").and_then(Value::as_f64) {
        current.roughness = (value as f32).clamp(0.0, 1.0);
    }
    if let Some(value) = params.get("specular").and_then(Value::as_f64) {
        current.specular = (value as f32).clamp(0.0, 1.0);
    }
    if let Some(array) = params.get("tint").and_then(Value::as_array) {
        if array.len() < 3 {
            return Err(invalid("shader.set", "tint needs 3 numbers"));
        }
        for (index, value) in array.iter().take(3).enumerate() {
            current.base_color_tint[index] = value.as_f64().ok_or_else(|| invalid("shader.set", "tint values must be numbers"))? as f32;
        }
    }
    session.set_shader_params(current);
    Ok(CommandOutcome::message("shader parameters updated"))
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_the_default_set() {
        let mut session = Session::new("test");
        let outcome = list(&mut session, &Value::Null).expect("list");
        let data = outcome.data.expect("data");
        let sets = data.as_array().expect("array");
        assert_eq!(sets.len(), 1);
        assert_eq!(sets[0]["active"], json!(true));
    }

    #[test]
    fn selects_by_index_and_name_and_rejects_bad_input() {
        let mut session = Session::new("test");
        assert!(select(&mut session, &json!({ "index": 0 })).is_ok());
        assert!(select(&mut session, &json!({ "name": "nope" })).is_err());
        assert!(select(&mut session, &Value::Null).is_err());
        assert!(select(&mut session, &json!({ "index": 5 })).is_err());
    }

    #[test]
    fn resolution_and_channel_edits_validate() {
        let mut session = Session::new("test");
        assert!(set_resolution(&mut session, &json!({ "resolution": 4096 })).is_ok());
        assert_eq!(session.texture_sets()[0].resolution, 4096);
        assert!(set_resolution(&mut session, &json!({ "resolution": 9000 })).is_err());
        assert!(set_channel(&mut session, &json!({ "channel": "emissive", "enabled": true })).is_ok());
        assert!(set_channel(&mut session, &json!({ "channel": "warp", "enabled": true })).is_err());
    }

    #[test]
    fn uv_tile_edits_validate_udim() {
        let mut session = Session::new("test");
        assert!(add_uv_tile(&mut session, &json!({ "udim": 1002 })).is_ok());
        assert!(session.texture_sets()[0].uv_tiles.contains(&UvTile { u: 1, v: 0 }));
        assert!(add_uv_tile(&mut session, &json!({ "udim": 999 })).is_err());
    }

    #[test]
    fn rename_rejects_empty() {
        let mut session = Session::new("test");
        assert!(rename(&mut session, &json!({ "name": "" })).is_err());
        assert!(rename(&mut session, &json!({ "name": "Body" })).is_ok());
        assert_eq!(session.texture_sets()[0].name, "Body");
    }

    #[test]
    fn shader_commands_validate_and_apply() {
        let mut session = Session::new("test");
        assert!(set_shader(&mut session, &json!({ "shader": "metal_rough" })).is_ok());
        assert_eq!(session.project().active_set().map(|set| set.shader), Some(Shader::MetalRough));
        assert!(set_shader(&mut session, &json!({ "shader": "toon" })).is_err());
        assert!(set_shader_params(&mut session, &json!({ "metallic": 0.9, "roughness": 0.2 })).is_ok());
        let params = session.project().active_set().map(|set| set.shader_params).unwrap_or_default();
        assert!((params.metallic - 0.9).abs() < 1e-6 && (params.roughness - 0.2).abs() < 1e-6);
        assert!(set_shader_params(&mut session, &json!({ "tint": [0.5, 0.5] })).is_err());
    }
}
