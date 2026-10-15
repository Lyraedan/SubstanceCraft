//! Export commands: write the active texture set's channels using presets and packing.

use serde_json::{Value, json};
use substancecraft_io::{ExportFormat, ExportSettings};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The export commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("export.presets", "List Export Presets", "File", "no parameters", list_presets),
        CommandSpec::new("export.formats", "List Export Formats", "File", "no parameters", list_formats),
        CommandSpec::new(
            "file.export_textures",
            "Export Textures…",
            "File",
            "dir: string, preset?: string, resolution?: number, format?: string, naming?: string, udim?: bool, height_to_normal?: bool, normal_strength?: number, directx_normal?: bool",
            export_textures,
        )
        .with_shortcut("Ctrl+Shift+E"),
    ]
}

/// Register the export commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn list_presets(_session: &mut Session, _params: &Value) -> CommandResult {
    let presets: Vec<Value> = substancecraft_io::presets()
        .into_iter()
        .map(|preset| json!({ "id": preset.id, "label": preset.label, "format": preset.format.id(), "channels": preset.channels.iter().map(|c| c.id()).collect::<Vec<_>>() }))
        .collect();
    Ok(CommandOutcome::data(Value::Array(presets)))
}

fn list_formats(_session: &mut Session, _params: &Value) -> CommandResult {
    let formats: Vec<Value> = ExportFormat::ALL.iter().map(|format| json!({ "id": format.id(), "label": format.label() })).collect();
    Ok(CommandOutcome::data(Value::Array(formats)))
}

fn export_textures(session: &mut Session, params: &Value) -> CommandResult {
    let dir = params.get("dir").and_then(Value::as_str).ok_or_else(|| invalid("file.export_textures", "expected a \"dir\" string"))?;
    if dir.trim().is_empty() {
        return Err(invalid("file.export_textures", "dir must not be empty"));
    }

    let preset = params.get("preset").and_then(Value::as_str).unwrap_or("openpbr").to_string();
    if substancecraft_io::preset(&preset).is_none() {
        return Err(invalid("file.export_textures", &format!("unknown preset: {preset}")));
    }
    let format = match params.get("format").and_then(Value::as_str) {
        Some(id) => Some(ExportFormat::from_id(id).ok_or_else(|| invalid("file.export_textures", &format!("unknown format: {id}")))?),
        None => None,
    };
    let resolution = params.get("resolution").and_then(Value::as_u64).map(|value| value as u32).unwrap_or(2048);
    let naming = params.get("naming").and_then(Value::as_str).unwrap_or("").to_string();
    let udim = params.get("udim").and_then(Value::as_bool).unwrap_or(false);
    let height_to_normal = params.get("height_to_normal").and_then(Value::as_bool).unwrap_or(false);
    let normal_strength = params.get("normal_strength").and_then(Value::as_f64).map(|value| (value as f32).clamp(0.0, 8.0)).unwrap_or(1.0);
    let directx_normal = params.get("directx_normal").and_then(Value::as_bool).unwrap_or(false);

    let (set, material) = {
        let project = session.project();
        let set = project.active_set().cloned().ok_or_else(|| CommandError::Failed("no texture set to export".to_string()))?;
        let material = set.materials.first().and_then(|id| project.scene.materials.get(*id as usize)).cloned().unwrap_or_default();
        (set, material)
    };

    let settings = ExportSettings { resolution, format, naming, preset, udim, height_to_normal, normal_strength, directx_normal };
    let surface = if substancecraft_compose::stack_needs_surface(&set.layers) { session.eval_surface(resolution, resolution) } else { None };
    let mesh_maps = session.eval_mesh_maps();
    let context = substancecraft_compose::EvalContext { surface: surface.as_ref(), mesh_maps: &mesh_maps };
    let written = substancecraft_io::export_set_ctx(&set, &material, &settings, std::path::Path::new(dir), &context)
        .map_err(|err| CommandError::Failed(format!("export failed: {err}")))?;
    let files: Vec<Value> = written.iter().map(|path| json!(path.to_string_lossy())).collect();
    session.set_status(format!("exported {} files to {dir}", written.len()));
    Ok(CommandOutcome::message_and_data(format!("exported {} files", written.len()), Value::Array(files)))
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::{Layer, Scene};
    use substancecraft_geom::primitives::uv_sphere;

    fn session_with_scene() -> Session {
        let mut session = Session::new("test");
        session.set_scene(Scene::from_mesh(uv_sphere(1.0, 8, 12).expect("sphere")));
        session
    }

    #[test]
    fn exports_and_reports_files() {
        let mut session = session_with_scene();
        session.add_layer(Layer::fill("base", [0.8, 0.1, 0.1, 1.0], 0.5));
        let dir = std::env::temp_dir().join(format!("sc_engine_export_{}", std::process::id()));
        let params = json!({ "dir": dir.to_string_lossy(), "preset": "openpbr", "resolution": 8 });
        let outcome = export_textures(&mut session, &params).expect("export");
        let files = outcome.data.expect("data");
        assert!(files.as_array().is_some_and(|list| !list.is_empty()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn validates_preset_and_dir() {
        let mut session = session_with_scene();
        assert!(export_textures(&mut session, &Value::Null).is_err());
        assert!(export_textures(&mut session, &json!({ "dir": "/tmp/x", "preset": "nope" })).is_err());
        assert!(export_textures(&mut session, &json!({ "dir": "/tmp/x", "format": "bmp" })).is_err());
    }

    #[test]
    fn lists_presets_and_formats() {
        let mut session = Session::new("test");
        assert!(list_presets(&mut session, &Value::Null).is_ok());
        assert!(list_formats(&mut session, &Value::Null).is_ok());
    }
}
