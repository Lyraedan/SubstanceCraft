//! Baking commands: import a high-poly, set bake parameters, run bakes and save maps.

use serde_json::{Value, json};
use substancecraft_bake::BakeMap;

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The bake commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("file.import_highpoly", "Import High-Poly…", "Bake", "path: string — a mesh file", import_highpoly),
        CommandSpec::new("bake.clear_highpoly", "Clear High-Poly", "Bake", "no parameters", clear_highpoly),
        CommandSpec::new("bake.set_resolution", "Set Bake Resolution", "Bake", "resolution: number (16..2048)", set_resolution),
        CommandSpec::new("bake.set_samples", "Set AO Samples", "Bake", "samples: number (1..256)", set_samples),
        CommandSpec::new("bake.run", "Bake Mesh Maps", "Bake", "no parameters", run),
        CommandSpec::new("bake.list", "List Baked Maps", "Bake", "no parameters", list),
        CommandSpec::new("bake.save", "Save Baked Map…", "Bake", "map: string, path: string", save),
    ]
}

/// Register the bake commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn import_highpoly(session: &mut Session, params: &Value) -> CommandResult {
    let path = params.get("path").and_then(Value::as_str).ok_or_else(|| invalid("file.import_highpoly", "expected a \"path\" string"))?;
    if path.trim().is_empty() {
        return Err(invalid("file.import_highpoly", "path must not be empty"));
    }
    let scene = substancecraft_io::load_scene(path).map_err(|err| CommandError::Failed(format!("could not import {path}: {err}")))?;
    let triangles = scene.mesh.triangle_count();
    session.set_high_poly(Some(scene.mesh));
    session.set_status(format!("high-poly: {path} ({triangles} triangles)"));
    Ok(CommandOutcome::message(format!("imported high-poly ({triangles} triangles)")))
}

fn clear_highpoly(session: &mut Session, _params: &Value) -> CommandResult {
    session.set_high_poly(None);
    Ok(CommandOutcome::message("high-poly cleared"))
}

fn set_resolution(session: &mut Session, params: &Value) -> CommandResult {
    let resolution =
        params.get("resolution").and_then(Value::as_u64).ok_or_else(|| invalid("bake.set_resolution", "expected a numeric \"resolution\""))?;
    if !(16..=2048).contains(&resolution) {
        return Err(invalid("bake.set_resolution", "resolution must be in 16..=2048"));
    }
    session.bake_settings_mut().resolution = resolution as u32;
    Ok(CommandOutcome::message(format!("bake resolution: {resolution}")))
}

fn set_samples(session: &mut Session, params: &Value) -> CommandResult {
    let samples = params.get("samples").and_then(Value::as_u64).ok_or_else(|| invalid("bake.set_samples", "expected a numeric \"samples\""))?;
    if !(1..=256).contains(&samples) {
        return Err(invalid("bake.set_samples", "samples must be in 1..=256"));
    }
    session.bake_settings_mut().samples = samples as u32;
    Ok(CommandOutcome::message(format!("AO samples: {samples}")))
}

fn run(session: &mut Session, _params: &Value) -> CommandResult {
    if session.scene().is_empty() {
        return Err(CommandError::Failed("no mesh to bake".to_string()));
    }
    session.run_bake();
    Ok(CommandOutcome::message("bake complete"))
}

fn list(session: &mut Session, _params: &Value) -> CommandResult {
    let maps: Vec<Value> = session
        .baked_maps()
        .iter()
        .map(|(map, image)| json!({ "id": map.id(), "label": map.label(), "width": image.width, "height": image.height }))
        .collect();
    Ok(CommandOutcome::data(Value::Array(maps)))
}

fn save(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("map").and_then(Value::as_str).ok_or_else(|| invalid("bake.save", "expected a \"map\" string"))?;
    let path = params.get("path").and_then(Value::as_str).ok_or_else(|| invalid("bake.save", "expected a \"path\" string"))?;
    let map = BakeMap::from_id(id).ok_or_else(|| invalid("bake.save", &format!("unknown map: {id}")))?;
    session.save_baked(map, path).map_err(CommandError::Failed)?;
    Ok(CommandOutcome::message(format!("saved {id} to {path}")))
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_geom::glam::Vec2;
    use substancecraft_geom::{Mesh, Submesh};

    fn quad_scene() -> Mesh {
        Mesh {
            positions: vec![
                substancecraft_geom::glam::Vec3::new(0.0, 0.0, 0.0),
                substancecraft_geom::glam::Vec3::new(1.0, 0.0, 0.0),
                substancecraft_geom::glam::Vec3::new(1.0, 1.0, 0.0),
                substancecraft_geom::glam::Vec3::new(0.0, 1.0, 0.0),
            ],
            normals: vec![substancecraft_geom::glam::Vec3::Z; 4],
            uvs: vec![Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(1.0, 1.0), Vec2::new(0.0, 1.0)],
            tangents: Vec::new(),
            indices: vec![0, 1, 2, 0, 2, 3],
            submeshes: vec![Submesh { material: 0, first_index: 0, index_count: 6 }],
        }
    }

    #[test]
    fn bake_produces_all_maps() {
        let mut session = Session::new("test");
        session.set_scene(substancecraft_doc::Scene::from_mesh(quad_scene()));
        session.bake_settings_mut().resolution = 32;
        run(&mut session, &Value::Null).expect("bake");
        assert_eq!(session.baked_maps().len(), BakeMap::ALL.len());
        let data = list(&mut session, &Value::Null).expect("list").data.expect("data");
        assert_eq!(data.as_array().map(Vec::len), Some(BakeMap::ALL.len()));
    }

    #[test]
    fn bake_on_empty_scene_is_a_graceful_error() {
        let mut session = Session::new("test");
        assert!(matches!(run(&mut session, &Value::Null), Err(CommandError::Failed(_))));
    }

    #[test]
    fn parameters_validate() {
        let mut session = Session::new("test");
        assert!(set_resolution(&mut session, &json!({ "resolution": 512 })).is_ok());
        assert!(set_resolution(&mut session, &json!({ "resolution": 9 })).is_err());
        assert!(set_samples(&mut session, &json!({ "samples": 32 })).is_ok());
        assert!(set_samples(&mut session, &json!({ "samples": 0 })).is_err());
    }

    #[test]
    fn save_without_a_bake_is_a_graceful_error() {
        let mut session = Session::new("test");
        let err = save(&mut session, &json!({ "map": "curvature", "path": "/tmp/x.png" })).expect_err("no bake");
        assert!(matches!(err, CommandError::Failed(_)));
        assert!(save(&mut session, &json!({ "map": "nope", "path": "/tmp/x.png" })).is_err());
    }
}
