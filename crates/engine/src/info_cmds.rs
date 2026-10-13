//! Introspection commands: report session, scene and viewport state to agents.

use serde_json::{Value, json};

use crate::command::{CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The introspection commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("session.info", "Session Info", "Window", "no parameters", session_info),
        CommandSpec::new("scene.info", "Scene Info", "Window", "no parameters", scene_info),
        CommandSpec::new("viewport.state", "Viewport State", "View", "no parameters", viewport_state),
        CommandSpec::new("app.capabilities", "Capabilities", "Help", "no parameters", capabilities),
    ]
}

/// Register the introspection commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), crate::command::CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn session_info(session: &mut Session, _params: &Value) -> CommandResult {
    Ok(CommandOutcome::data(json!({
        "label": session.label(),
        "revision": session.revision(),
        "dirty": session.is_dirty(),
        "path": session.file_path().map(|path| path.to_string_lossy().to_string()),
        "title": session.project_title(),
    })))
}

fn scene_info(session: &mut Session, _params: &Value) -> CommandResult {
    let mesh = &session.scene().mesh;
    let bounds = mesh.bounds();
    Ok(CommandOutcome::data(json!({
        "vertices": mesh.vertex_count(),
        "triangles": mesh.triangle_count(),
        "materials": session.scene().materials.len(),
        "texture_sets": session.texture_sets().len(),
        "bounds": {
            "min": bounds.min.to_array(),
            "max": bounds.max.to_array(),
            "radius": bounds.radius(),
        },
    })))
}

fn viewport_state(session: &mut Session, _params: &Value) -> CommandResult {
    let camera = *session.camera();
    let settings = *session.settings();
    Ok(CommandOutcome::data(json!({
        "projection": camera.projection.id(),
        "display": settings.display.id(),
        "exposure_stops": settings.exposure_stops,
        "tone_map": format!("{:?}", settings.tone_map),
        "show_grid": settings.show_grid,
        "show_background": settings.show_background,
        "isolate_active_set": settings.isolate_active_set,
        "vignette": settings.vignette,
        "bloom": settings.bloom,
        "camera": {
            "target": camera.target.to_array(),
            "distance": camera.distance,
            "yaw": camera.yaw,
            "pitch": camera.pitch,
        },
        "active_texture_set": session.active_texture_set(),
        "tool": session.tool().id(),
    })))
}

fn capabilities(session: &mut Session, _params: &Value) -> CommandResult {
    Ok(CommandOutcome::data(json!({
        "name": "SubstanceCraft",
        "version": env!("CARGO_PKG_VERSION"),
        "active_texture_set": session.active_texture_set(),
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_state() {
        let mut session = Session::new("test");
        for command in ["session.info", "scene.info", "viewport.state", "app.capabilities"] {
            let spec = specs().into_iter().find(|spec| spec.id == command).expect("spec");
            let outcome = (spec.run)(&mut session, &Value::Null).expect("run");
            assert!(outcome.data.is_some(), "{command} produced no data");
        }
    }
}
