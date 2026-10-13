//! Viewport commands: display mode, exposure, tone mapping and framing.

use serde_json::Value;
use substancecraft_render::{DisplayMode, ToneMap};

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
}
