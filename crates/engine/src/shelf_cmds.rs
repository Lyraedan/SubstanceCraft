//! Content-library commands: material presets, smart materials and smart masks.

use serde_json::{Value, json};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The shelf commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("shelf.list", "List Shelf", "Window", "no parameters", list),
        CommandSpec::new("shelf.apply_material", "Apply Material Preset", "Window", "name: string", apply_material),
        CommandSpec::new("shelf.apply_smart_material", "Apply Smart Material", "Window", "name: string", apply_smart_material),
        CommandSpec::new("shelf.apply_smart_mask", "Apply Smart Mask", "Window", "layer: number, name: string", apply_smart_mask),
        CommandSpec::new("layer.save_smart_material", "Save as Smart Material", "Window", "index: number, name: string", save_smart_material),
        CommandSpec::new("layer.save_smart_mask", "Save Mask as Smart Mask", "Window", "index: number, name: string", save_smart_mask),
    ]
}

/// Register the shelf commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

fn name<'a>(command: &str, params: &'a Value) -> Result<&'a str, CommandError> {
    params
        .get("name")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid(command, "expected a non-empty \"name\" string"))
}

fn list(session: &mut Session, _params: &Value) -> CommandResult {
    Ok(CommandOutcome::data(json!({
        "materials": session.material_preset_names(),
        "smart_materials": session.smart_material_names(),
        "smart_masks": session.smart_mask_names(),
    })))
}

fn apply_material(session: &mut Session, params: &Value) -> CommandResult {
    let name = name("shelf.apply_material", params)?;
    if session.apply_material_preset(name) {
        Ok(CommandOutcome::message(format!("applied material {name}")))
    } else {
        Err(invalid("shelf.apply_material", &format!("no material preset named {name}")))
    }
}

fn apply_smart_material(session: &mut Session, params: &Value) -> CommandResult {
    let name = name("shelf.apply_smart_material", params)?;
    if session.apply_smart_material(name) {
        Ok(CommandOutcome::message(format!("applied smart material {name}")))
    } else {
        Err(invalid("shelf.apply_smart_material", &format!("no smart material named {name}")))
    }
}

fn apply_smart_mask(session: &mut Session, params: &Value) -> CommandResult {
    let layer = params.get("layer").and_then(Value::as_u64).ok_or_else(|| invalid("shelf.apply_smart_mask", "expected a \"layer\" number"))? as usize;
    let name = name("shelf.apply_smart_mask", params)?;
    if session.apply_smart_mask(layer, name) {
        Ok(CommandOutcome::message(format!("applied smart mask {name}")))
    } else {
        Err(invalid("shelf.apply_smart_mask", &format!("no smart mask named {name} for layer {layer}")))
    }
}

fn save_smart_material(session: &mut Session, params: &Value) -> CommandResult {
    let index =
        params.get("index").and_then(Value::as_u64).ok_or_else(|| invalid("layer.save_smart_material", "expected an \"index\" number"))? as usize;
    let name = name("layer.save_smart_material", params)?;
    if session.save_smart_material(index, name) {
        Ok(CommandOutcome::message(format!("saved smart material {name}")))
    } else {
        Err(invalid("layer.save_smart_material", &format!("no layer at index {index}")))
    }
}

fn save_smart_mask(session: &mut Session, params: &Value) -> CommandResult {
    let index = params.get("index").and_then(Value::as_u64).ok_or_else(|| invalid("layer.save_smart_mask", "expected an \"index\" number"))? as usize;
    let name = name("layer.save_smart_mask", params)?;
    if session.save_smart_mask(index, name) {
        Ok(CommandOutcome::message(format!("saved smart mask {name}")))
    } else {
        Err(invalid("layer.save_smart_mask", &format!("layer {index} has no mask stack")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_and_applies_presets() {
        let mut session = Session::new("test");
        let data = list(&mut session, &Value::Null).expect("list").data.expect("data");
        assert!(data.get("materials").and_then(Value::as_array).is_some_and(|names| !names.is_empty()));
        assert!(data.get("smart_materials").and_then(Value::as_array).is_some_and(|names| !names.is_empty()));

        assert!(apply_material(&mut session, &json!({ "name": "Gold" })).is_ok());
        assert!(session.layers().is_some_and(|stack| stack.len() == 3), "material preset should add three channel fills");
        assert!(apply_smart_material(&mut session, &json!({ "name": "Worn Metal" })).is_ok());
        assert!(apply_smart_mask(&mut session, &json!({ "layer": 0, "name": "Edges" })).is_ok());
        assert!(apply_material(&mut session, &json!({ "name": "Nope" })).is_err());
    }

    #[test]
    fn saving_and_reloading_user_assets() {
        let mut session = Session::new("test");
        session.add_layer(substancecraft_doc::Layer::fill("Base", [0.5, 0.4, 0.3, 1.0], 0.5));
        assert!(save_smart_material(&mut session, &json!({ "index": 0, "name": "My Metal" })).is_ok());
        assert!(session.smart_material_names().contains(&"My Metal".to_string()));
        assert!(apply_smart_material(&mut session, &json!({ "name": "My Metal" })).is_ok());
        assert!(save_smart_mask(&mut session, &json!({ "index": 0, "name": "Nope" })).is_err());
    }
}
