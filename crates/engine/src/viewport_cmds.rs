//! Viewport commands: display mode, exposure, tone mapping and framing.

use serde_json::Value;
use substancecraft_render::{DisplayMode, Projection, ToneMap};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The viewport commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new(
            "viewport.set_display_mode",
            "Set Display Mode",
            "View > Display",
            "mode: base_color | normal | roughness | metalness | uv_checker",
            set_display_mode,
        ),
        CommandSpec::new("viewport.set_exposure", "Set Exposure", "View", "stops: number — exposure in stops", set_exposure),
        CommandSpec::new(
            "viewport.set_tone_map",
            "Set Tone Mapping",
            "View > Tone Mapping",
            "mode: none | reinhard | aces | agx | neutral",
            set_tone_map,
        ),
        CommandSpec::new("viewport.frame_all", "Frame All", "View", "no parameters", frame_all).with_shortcut("F"),
        CommandSpec::new("viewport.set_projection", "Set Projection", "View", "mode: perspective | orthographic", set_projection),
        CommandSpec::new("viewport.set_grid", "Toggle Grid", "View", "enabled: bool", set_grid),
        CommandSpec::new("viewport.set_background", "Toggle Background", "View", "enabled: bool", set_background),
        CommandSpec::new(
            "viewport.set_environment",
            "Open Environment…",
            "View > Environment",
            "path: string — a .hdr or .exr file",
            set_environment,
        ),
        CommandSpec::new("viewport.use_demo_environment", "Demo Environment", "View > Environment", "no parameters", use_demo_environment),
        CommandSpec::new("viewport.clear_environment", "Clear Environment", "View > Environment", "no parameters", clear_environment),
    ]
}

/// Register the viewport commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn set_display_mode(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("mode").and_then(Value::as_str).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_display_mode".to_string(),
        message: "expected a \"mode\" string".to_string(),
    })?;
    let mode = DisplayMode::from_id(id).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_display_mode".to_string(),
        message: format!("unknown display mode: {id}"),
    })?;
    session.settings_mut().display = mode;
    session.touch();
    session.set_status(format!("display: {id}"));
    Ok(CommandOutcome::message(format!("display mode: {id}")))
}

fn set_exposure(session: &mut Session, params: &Value) -> CommandResult {
    let stops = params.get("stops").and_then(Value::as_f64).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_exposure".to_string(),
        message: "expected a numeric \"stops\"".to_string(),
    })?;
    if !stops.is_finite() {
        return Err(CommandError::InvalidParams { command: "viewport.set_exposure".to_string(), message: "stops must be finite".to_string() });
    }
    session.settings_mut().exposure_stops = (stops as f32).clamp(-20.0, 20.0);
    session.touch();
    Ok(CommandOutcome::message(format!("exposure: {stops:.2}")))
}

fn set_tone_map(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("mode").and_then(Value::as_str).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_tone_map".to_string(),
        message: "expected a \"mode\" string".to_string(),
    })?;
    let tone_map = match id {
        "none" => ToneMap::None,
        "reinhard" => ToneMap::Reinhard,
        "aces" => ToneMap::Aces,
        "agx" => ToneMap::Agx,
        "neutral" => ToneMap::Neutral,
        other => {
            return Err(CommandError::InvalidParams { command: "viewport.set_tone_map".to_string(), message: format!("unknown tone map: {other}") });
        }
    };
    session.settings_mut().tone_map = tone_map;
    session.touch();
    Ok(CommandOutcome::message(format!("tone map: {id}")))
}

fn frame_all(session: &mut Session, _params: &Value) -> CommandResult {
    let bounds = session.scene().mesh.bounds();
    session.camera_mut().focus(bounds);
    session.touch();
    Ok(CommandOutcome::message("framed the scene"))
}

fn set_projection(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("mode").and_then(Value::as_str).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_projection".to_string(),
        message: "expected a \"mode\" string".to_string(),
    })?;
    let projection = Projection::from_id(id).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_projection".to_string(),
        message: format!("unknown projection: {id}"),
    })?;
    session.camera_mut().projection = projection;
    session.touch();
    Ok(CommandOutcome::message(format!("projection: {id}")))
}

fn set_grid(session: &mut Session, params: &Value) -> CommandResult {
    let enabled = params.get("enabled").and_then(Value::as_bool).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_grid".to_string(),
        message: "expected an \"enabled\" bool".to_string(),
    })?;
    session.settings_mut().show_grid = enabled;
    session.touch();
    Ok(CommandOutcome::message(if enabled { "grid on" } else { "grid off" }))
}

fn set_background(session: &mut Session, params: &Value) -> CommandResult {
    let enabled = params.get("enabled").and_then(Value::as_bool).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_background".to_string(),
        message: "expected an \"enabled\" bool".to_string(),
    })?;
    session.settings_mut().show_background = enabled;
    session.touch();
    Ok(CommandOutcome::message(if enabled { "background on" } else { "background off" }))
}

fn set_environment(session: &mut Session, params: &Value) -> CommandResult {
    let path = params.get("path").and_then(Value::as_str).ok_or_else(|| CommandError::InvalidParams {
        command: "viewport.set_environment".to_string(),
        message: "expected a \"path\" string".to_string(),
    })?;
    if path.trim().is_empty() {
        return Err(CommandError::InvalidParams { command: "viewport.set_environment".to_string(), message: "path must not be empty".to_string() });
    }
    let image =
        substancecraft_codecs::load_environment(path).map_err(|err| CommandError::Failed(format!("could not load environment {path}: {err}")))?;
    let size = format!("{}x{}", image.width, image.height);
    session.set_environment(Some(image));
    session.set_status(format!("environment: {path} ({size})"));
    Ok(CommandOutcome::message(format!("environment loaded ({size})")))
}

fn use_demo_environment(session: &mut Session, _params: &Value) -> CommandResult {
    let image = substancecraft_render::synthetic_environment(512, 256);
    session.set_environment(Some(image));
    session.set_status("environment: synthetic demo");
    Ok(CommandOutcome::message("demo environment loaded"))
}

fn clear_environment(session: &mut Session, _params: &Value) -> CommandResult {
    session.set_environment(None);
    session.set_status("environment cleared");
    Ok(CommandOutcome::message("environment cleared"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn sets_a_valid_display_mode() {
        let mut session = Session::new("test");
        set_display_mode(&mut session, &json!({ "mode": "normal" })).expect("set");
        assert_eq!(session.settings().display, DisplayMode::Normal);
    }

    #[test]
    fn unknown_display_mode_is_a_graceful_error() {
        let mut session = Session::new("test");
        let err = set_display_mode(&mut session, &json!({ "mode": "hologram" })).expect_err("bad mode");
        assert!(matches!(err, CommandError::InvalidParams { .. }));
    }

    #[test]
    fn exposure_rejects_non_finite() {
        let mut session = Session::new("test");
        let err = set_exposure(&mut session, &json!({ "stops": null })).expect_err("bad exposure");
        assert!(matches!(err, CommandError::InvalidParams { .. }));
    }

    #[test]
    fn tone_map_ids_round_trip() {
        let mut session = Session::new("test");
        for (id, expected) in
            [("none", ToneMap::None), ("reinhard", ToneMap::Reinhard), ("aces", ToneMap::Aces), ("agx", ToneMap::Agx), ("neutral", ToneMap::Neutral)]
        {
            set_tone_map(&mut session, &json!({ "mode": id })).expect("set");
            assert_eq!(session.settings().tone_map, expected);
        }
    }

    #[test]
    fn sets_projection_and_rejects_unknown() {
        let mut session = Session::new("test");
        set_projection(&mut session, &json!({ "mode": "orthographic" })).expect("set");
        assert_eq!(session.camera().projection, Projection::Orthographic);
        assert!(matches!(set_projection(&mut session, &json!({ "mode": "fish_eye" })).expect_err("bad"), CommandError::InvalidParams { .. }));
    }

    #[test]
    fn grid_and_background_toggles() {
        let mut session = Session::new("test");
        set_grid(&mut session, &json!({ "enabled": false })).expect("grid");
        assert!(!session.settings().show_grid);
        set_background(&mut session, &json!({ "enabled": false })).expect("background");
        assert!(!session.settings().show_background);
        assert!(matches!(set_grid(&mut session, &json!({ "enabled": "yes" })).expect_err("bad"), CommandError::InvalidParams { .. }));
    }

    #[test]
    fn demo_environment_sets_and_clears() {
        let mut session = Session::new("test");
        let before = session.environment_revision();
        use_demo_environment(&mut session, &Value::Null).expect("demo env");
        assert!(session.environment().is_some());
        assert!(session.environment_revision() > before);
        clear_environment(&mut session, &Value::Null).expect("clear");
        assert!(session.environment().is_none());
    }

    #[test]
    fn environment_missing_file_is_a_graceful_error() {
        let mut session = Session::new("test");
        let err = set_environment(&mut session, &json!({ "path": "/definitely/not/here.hdr" })).expect_err("missing");
        assert!(matches!(err, CommandError::Failed(_)));
        assert!(matches!(set_environment(&mut session, &Value::Null).expect_err("no path"), CommandError::InvalidParams { .. }));
    }
}
