//! Scene commands: import a mesh, load the demo model, clear the scene.

use serde_json::Value;

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The scene commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("file.import_mesh", "Import Mesh…", "File", "path: string — a .obj, .gltf or .glb file", import_mesh)
            .with_shortcut("Ctrl+O"),
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
}
