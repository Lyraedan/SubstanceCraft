//! Scene commands: import a mesh, load the demo model, clear the scene.

use serde_json::{Value, json};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The scene commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new(
            "file.import_mesh",
            "Import Mesh…",
            "File",
            "path: string — a .obj, .gltf, .glb, .stl, .ply, .dae or binary .fbx file",
            import_mesh,
        )
        .with_shortcut("Ctrl+O"),
        CommandSpec::new("scene.parts", "List Parts", "Window", "mode?: pieces|islands|materials", parts),
        CommandSpec::new("scene.pick_part", "Pick Part", "Window", "x: number, y: number, width: number, height: number", pick_part),
        CommandSpec::new("scene.isolate_part", "Isolate Part", "Window", "part: number|null", isolate_part),
        CommandSpec::new("scene.hide_part", "Hide Part", "Window", "part: number, hidden: bool", hide_part),
        CommandSpec::new("scene.show_all_parts", "Show All Parts", "Window", "no parameters", show_all_parts).with_shortcut("Esc"),
        CommandSpec::new("scene.load_default", "Load Demo Model", "File", "no parameters", load_default),
        CommandSpec::new("scene.clear", "Clear Scene", "File", "no parameters", |session, _params| {
            session.set_scene(substancecraft_doc::Scene::default());
            session.set_status("scene cleared");
            Ok(CommandOutcome::message("scene cleared"))
        }),
    ]
}

/// Register the scene commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn import_mesh(session: &mut Session, params: &Value) -> CommandResult {
    let Some(path) = params.get("path").and_then(Value::as_str) else {
        return Err(CommandError::InvalidParams { command: "file.import_mesh".to_string(), message: "expected a \"path\" string".to_string() });
    };
    if path.trim().is_empty() {
        return Err(CommandError::InvalidParams { command: "file.import_mesh".to_string(), message: "path must not be empty".to_string() });
    }

    let scene = substancecraft_io::load_scene(path).map_err(|err| CommandError::Failed(format!("could not import {path}: {err}")))?;
    let triangles = scene.mesh.triangle_count();
    session.camera_mut().focus(scene.mesh.bounds());
    session.set_scene(scene);
    session.set_status(format!("imported {path} ({triangles} triangles)"));
    Ok(CommandOutcome::message(format!("imported {triangles} triangles")))
}

fn parts(session: &mut Session, params: &Value) -> CommandResult {
    let mode = params.get("mode").and_then(Value::as_str).unwrap_or("pieces");
    let counts = session.part_counts(mode);
    let data: Vec<Value> = counts.iter().enumerate().map(|(id, triangles)| json!({ "id": id, "triangles": triangles })).collect();
    Ok(CommandOutcome::data(json!({ "mode": mode, "parts": data })))
}

fn pick_part(session: &mut Session, params: &Value) -> CommandResult {
    let number = |key: &str| params.get(key).and_then(Value::as_f64).map(|value| value as f32);
    let (Some(x), Some(y), Some(width), Some(height)) = (number("x"), number("y"), number("width"), number("height")) else {
        return Err(CommandError::InvalidParams {
            command: "scene.pick_part".to_string(),
            message: "expected numeric \"x\", \"y\", \"width\" and \"height\"".to_string(),
        });
    };
    let part = session.pick_part(x, y, width, height);
    Ok(CommandOutcome::data(json!({ "part": part })))
}

fn isolate_part(session: &mut Session, params: &Value) -> CommandResult {
    let part = params.get("part").and_then(Value::as_u64).map(|value| value as u32);
    session.set_isolate_part(part);
    Ok(CommandOutcome::message(match part {
        Some(part) => format!("isolated part {part}"),
        None => "isolation cleared".to_string(),
    }))
}

fn hide_part(session: &mut Session, params: &Value) -> CommandResult {
    let part = params
        .get("part")
        .and_then(Value::as_u64)
        .ok_or_else(|| CommandError::InvalidParams { command: "scene.hide_part".to_string(), message: "expected a \"part\" number".to_string() })?
        as u32;
    let hidden = params.get("hidden").and_then(Value::as_bool).unwrap_or(true);
    session.set_part_hidden(part, hidden);
    Ok(CommandOutcome::message(format!("part {part} {}", if hidden { "hidden" } else { "shown" })))
}

fn show_all_parts(session: &mut Session, _params: &Value) -> CommandResult {
    session.show_all_parts();
    Ok(CommandOutcome::message("all parts shown"))
}

fn load_default(session: &mut Session, _params: &Value) -> CommandResult {
    let scene = substancecraft_io::demo_scene();
    session.camera_mut().focus(scene.mesh.bounds());
    session.set_scene(scene);
    session.set_status("loaded the demo model");
    Ok(CommandOutcome::message("demo model loaded"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_default_populates_the_scene() {
        let mut session = Session::new("test");
        load_default(&mut session, &Value::Null).expect("load");
        assert!(!session.scene().is_empty());
        assert!(session.scene().mesh.triangle_count() > 0);
    }

    #[test]
    fn import_without_a_path_is_a_graceful_error() {
        let mut session = Session::new("test");
        let err = import_mesh(&mut session, &Value::Null).expect_err("missing path");
        assert!(matches!(err, CommandError::InvalidParams { .. }));
    }

    #[test]
    fn import_of_a_missing_file_is_an_error_not_a_panic() {
        let mut session = Session::new("test");
        let params = serde_json::json!({ "path": "/definitely/not/here.obj" });
        let err = import_mesh(&mut session, &params).expect_err("missing file");
        assert!(matches!(err, CommandError::Failed(_)));
    }

    #[test]
    fn part_visibility_commands_toggle_and_reset() {
        let mut session = Session::new("test");
        load_default(&mut session, &Value::Null).expect("load");
        assert!(session.part_count() >= 1);

        let data = parts(&mut session, &Value::Null).expect("parts").data.expect("data");
        assert_eq!(data.get("mode").and_then(Value::as_str), Some("pieces"));
        assert!(data.get("parts").and_then(Value::as_array).is_some_and(|parts| !parts.is_empty()));

        assert!(hide_part(&mut session, &json!({ "part": 0, "hidden": true })).is_ok());
        assert!(!session.part_visible(0));
        assert!(isolate_part(&mut session, &json!({ "part": 0 })).is_ok());
        assert!(session.part_visible(0));
        assert!(show_all_parts(&mut session, &Value::Null).is_ok());
        assert!(session.part_visible(0));
    }

    #[test]
    fn pick_part_returns_none_for_an_empty_scene() {
        let mut session = Session::new("test");
        assert!(pick_part(&mut session, &json!({ "x": 10.0, "y": 10.0, "width": 100.0, "height": 100.0 })).is_ok());
        assert!(pick_part(&mut session, &Value::Null).is_err());
    }
}
