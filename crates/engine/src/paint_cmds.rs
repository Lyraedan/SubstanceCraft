//! Paint commands: stroke begin/extend/end and brush settings.

use serde_json::Value;
use substancecraft_paint::Symmetry;

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The paint commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("paint.stroke_begin", "Begin Stroke", "Brush", "u: number, v: number, pressure?: number", stroke_begin),
        CommandSpec::new("paint.stroke_extend", "Extend Stroke", "Brush", "u: number, v: number, pressure?: number", stroke_extend),
        CommandSpec::new("paint.stroke_end", "End Stroke", "Brush", "no parameters", stroke_end),
        CommandSpec::new("brush.set_radius", "Set Brush Radius", "Brush", "radius: number (texture pixels)", set_radius),
        CommandSpec::new("brush.set_hardness", "Set Brush Hardness", "Brush", "hardness: number (0..1)", set_hardness),
        CommandSpec::new("brush.set_spacing", "Set Brush Spacing", "Brush", "spacing: number (0.01..1)", set_spacing),
        CommandSpec::new("brush.set_opacity", "Set Brush Opacity", "Brush", "opacity: number (0..1)", set_opacity),
        CommandSpec::new("brush.set_color", "Set Brush Color", "Brush", "color: [r,g,b,a]", set_color),
        CommandSpec::new("brush.set_symmetry", "Set Brush Symmetry", "Brush", "mode: none | horizontal | vertical | both", set_symmetry),
        CommandSpec::new("brush.set_pressure", "Set Brush Pressure", "Brush", "size?: bool, opacity?: bool", set_pressure),
    ]
}

/// Register the paint commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn required_uv(command: &str, params: &Value) -> Result<(f32, f32), CommandError> {
    let u = params.get("u").and_then(Value::as_f64);
    let v = params.get("v").and_then(Value::as_f64);
    match (u, v) {
        (Some(u), Some(v)) if u.is_finite() && v.is_finite() => Ok((u as f32, v as f32)),
        _ => Err(invalid(command, "expected finite \"u\" and \"v\" numbers")),
    }
}

fn pressure(params: &Value) -> f32 {
    params.get("pressure").and_then(Value::as_f64).map(|value| (value as f32).clamp(0.0, 1.0)).unwrap_or(1.0)
}

fn stroke_begin(session: &mut Session, params: &Value) -> CommandResult {
    let (u, v) = required_uv("paint.stroke_begin", params)?;
    session.begin_stroke(u, v, pressure(params));
    Ok(CommandOutcome::message("stroke begin"))
}

fn stroke_extend(session: &mut Session, params: &Value) -> CommandResult {
    let (u, v) = required_uv("paint.stroke_extend", params)?;
    session.extend_stroke(u, v, pressure(params));
    Ok(CommandOutcome::message("stroke extend"))
}

fn stroke_end(session: &mut Session, _params: &Value) -> CommandResult {
    session.end_stroke();
    Ok(CommandOutcome::message("stroke end"))
}

fn number(command: &str, params: &Value, key: &str) -> Result<f32, CommandError> {
    match params.get(key).and_then(Value::as_f64) {
        Some(value) if value.is_finite() => Ok(value as f32),
        _ => Err(invalid(command, &format!("expected a finite \"{key}\" number"))),
    }
}

fn set_radius(session: &mut Session, params: &Value) -> CommandResult {
    let radius = number("brush.set_radius", params, "radius")?.clamp(1.0, 2048.0);
    session.brush_mut().radius_px = radius;
    Ok(CommandOutcome::message(format!("brush radius: {radius:.0}")))
}

fn set_hardness(session: &mut Session, params: &Value) -> CommandResult {
    let hardness = number("brush.set_hardness", params, "hardness")?.clamp(0.0, 1.0);
    session.brush_mut().hardness = hardness;
    Ok(CommandOutcome::message(format!("brush hardness: {hardness:.2}")))
}

fn set_spacing(session: &mut Session, params: &Value) -> CommandResult {
    let spacing = number("brush.set_spacing", params, "spacing")?.clamp(0.01, 1.0);
    session.brush_mut().spacing = spacing;
    Ok(CommandOutcome::message(format!("brush spacing: {spacing:.2}")))
}

fn set_opacity(session: &mut Session, params: &Value) -> CommandResult {
    let opacity = number("brush.set_opacity", params, "opacity")?.clamp(0.0, 1.0);
    session.brush_mut().opacity = opacity;
    Ok(CommandOutcome::message(format!("brush opacity: {opacity:.2}")))
}

fn set_color(session: &mut Session, params: &Value) -> CommandResult {
    let array = params.get("color").and_then(Value::as_array).ok_or_else(|| invalid("brush.set_color", "expected a \"color\" array"))?;
    if array.len() < 3 {
        return Err(invalid("brush.set_color", "color needs 3 or 4 numbers"));
    }
    let mut color = [0.0f32, 0.0, 0.0, 1.0];
    for (index, value) in array.iter().take(4).enumerate() {
        color[index] = value.as_f64().ok_or_else(|| invalid("brush.set_color", "color values must be numbers"))? as f32;
    }
    session.brush_mut().color = color;
    Ok(CommandOutcome::message("brush color set"))
}

fn set_symmetry(session: &mut Session, params: &Value) -> CommandResult {
    let id = params.get("mode").and_then(Value::as_str).ok_or_else(|| invalid("brush.set_symmetry", "expected a \"mode\" string"))?;
    let mode = Symmetry::from_id(id).ok_or_else(|| invalid("brush.set_symmetry", &format!("unknown symmetry: {id}")))?;
    session.brush_mut().symmetry = mode;
    Ok(CommandOutcome::message(format!("symmetry: {id}")))
}

fn set_pressure(session: &mut Session, params: &Value) -> CommandResult {
    if let Some(size) = params.get("size").and_then(Value::as_bool) {
        session.brush_mut().size_pressure = size;
    }
    if let Some(opacity) = params.get("opacity").and_then(Value::as_bool) {
        session.brush_mut().opacity_pressure = opacity;
    }
    Ok(CommandOutcome::message("pressure settings updated"))
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use substancecraft_doc::LayerKind;

    fn top_paint_image(session: &Session) -> Option<&substancecraft_raster::RasterImage> {
        session.layers()?.layers.last().and_then(|layer| match &layer.kind {
            LayerKind::Paint { image } => image.as_ref(),
            _ => None,
        })
    }

    #[test]
    fn a_stroke_paints_into_a_new_paint_layer() {
        let mut session = Session::new("test");
        stroke_begin(&mut session, &json!({ "u": 0.5, "v": 0.5 })).expect("begin");
        stroke_extend(&mut session, &json!({ "u": 0.6, "v": 0.5 })).expect("extend");
        stroke_end(&mut session, &Value::Null).expect("end");
        let image = top_paint_image(&session).expect("paint image");
        let center = image.pixel(image.width / 2, image.height / 2);
        assert!(center[3] > 0.0, "no paint at the centre: {center:?}");
    }

    #[test]
    fn invalid_uv_is_a_graceful_error() {
        let mut session = Session::new("test");
        assert!(stroke_begin(&mut session, &Value::Null).is_err());
        assert!(stroke_begin(&mut session, &json!({ "u": "x", "v": 0.5 })).is_err());
    }

    #[test]
    fn brush_settings_are_clamped() {
        let mut session = Session::new("test");
        set_radius(&mut session, &json!({ "radius": 99999 })).expect("radius");
        assert_eq!(session.brush().radius_px, 2048.0);
        set_hardness(&mut session, &json!({ "hardness": 5 })).expect("hardness");
        assert_eq!(session.brush().hardness, 1.0);
        assert!(set_color(&mut session, &json!({ "color": [0.1, 0.2] })).is_err());
        assert!(set_symmetry(&mut session, &json!({ "mode": "diagonal" })).is_err());
        set_symmetry(&mut session, &json!({ "mode": "both" })).expect("symmetry");
        assert_eq!(session.brush().symmetry, Symmetry::Both);
    }

    #[test]
    fn a_stroke_is_one_undo_step() {
        let mut session = Session::new("test");
        let before = session.layers().map(|stack| stack.len()).unwrap_or(0);
        stroke_begin(&mut session, &json!({ "u": 0.5, "v": 0.5 })).expect("begin");
        stroke_end(&mut session, &Value::Null).expect("end");
        assert!(session.layers().map(|stack| stack.len()).unwrap_or(0) > before);
        assert!(session.undo());
        assert_eq!(session.layers().map(|stack| stack.len()).unwrap_or(0), before);
    }
}
