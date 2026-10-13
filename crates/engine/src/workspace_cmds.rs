//! Workspace commands: panel visibility and dock selection, so the control channel, CLI and MCP
//! can show/hide panels and drive the right dock.

use serde_json::{Value, json};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;
use crate::workspace::{Panel, RightTab};

/// Register the workspace commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    registry.register(CommandSpec::new("ui.panel_state", "Workspace Panels", "Window", "no parameters", panel_state))?;
    registry.register(CommandSpec::new("ui.set_panel", "Set Panel Visibility", "Window", "panel: string, visible: bool", set_panel))?;
    registry.register(CommandSpec::new("ui.toggle_panel", "Toggle Panel", "Window", "panel: string", toggle_panel))?;
    registry.register(CommandSpec::new("ui.set_right_tab", "Set Right Tab", "Window", "tab: layers | settings", set_right_tab))?;
    Ok(())
}

/// The panel ids, for error messages.
fn panel_hint() -> String {
    Panel::ALL.iter().map(|panel| panel.id()).collect::<Vec<_>>().join(" | ")
}

fn panel_param(params: &Value) -> Result<Panel, CommandError> {
    params
        .get("panel")
        .and_then(Value::as_str)
        .and_then(Panel::from_id)
        .ok_or_else(|| CommandError::InvalidParams { command: "ui.set_panel".into(), message: format!("panel must be {}", panel_hint()) })
}

fn panel_state(session: &mut Session, _params: &Value) -> CommandResult {
    let workspace = session.workspace();
    Ok(CommandOutcome::data(json!({
        "texture_sets": workspace.texture_sets,
        "layers": workspace.layers,
        "assets": workspace.assets,
        "tool_properties": workspace.tool_properties,
        "tool_rail": workspace.tool_rail,
        "right_tab": workspace.right_tab.id(),
    })))
}

fn set_panel(session: &mut Session, params: &Value) -> CommandResult {
    let panel = panel_param(params)?;
    let visible = params
        .get("visible")
        .and_then(Value::as_bool)
        .ok_or_else(|| CommandError::InvalidParams { command: "ui.set_panel".into(), message: "visible must be a bool".into() })?;
    session.set_panel(panel, visible);
    Ok(CommandOutcome::message(format!("{} {}", panel.label(), if visible { "shown" } else { "hidden" })))
}

fn toggle_panel(session: &mut Session, params: &Value) -> CommandResult {
    let panel = panel_param(params)?;
    let visible = session.toggle_panel(panel);
    Ok(CommandOutcome::message(format!("{} {}", panel.label(), if visible { "shown" } else { "hidden" })))
}

fn set_right_tab(session: &mut Session, params: &Value) -> CommandResult {
    let tab = params
        .get("tab")
        .and_then(Value::as_str)
        .and_then(RightTab::from_id)
        .ok_or_else(|| CommandError::InvalidParams { command: "ui.set_right_tab".into(), message: "tab must be layers | settings".into() })?;
    session.set_right_tab(tab);
    Ok(CommandOutcome::message(format!("right dock: {}", tab.label())))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggling_a_panel_flips_its_visibility() {
        let mut session = Session::new("test");
        let outcome = toggle_panel(&mut session, &json!({ "panel": "assets" })).expect("toggle");
        assert!(outcome.message.unwrap_or_default().contains("hidden"));
        assert!(!session.workspace().assets);
    }

    #[test]
    fn setting_a_panel_and_tab_persists() {
        let mut session = Session::new("test");
        set_panel(&mut session, &json!({ "panel": "tool_rail", "visible": false })).expect("set");
        assert!(!session.workspace().tool_rail);
        set_right_tab(&mut session, &json!({ "tab": "settings" })).expect("tab");
        assert_eq!(session.workspace().right_tab, RightTab::Settings);
    }

    #[test]
    fn bad_params_are_errors_not_panics() {
        let mut session = Session::new("test");
        assert!(set_panel(&mut session, &json!({ "panel": "nope", "visible": true })).is_err());
        assert!(set_panel(&mut session, &json!({ "panel": "assets" })).is_err());
        assert!(toggle_panel(&mut session, &Value::Null).is_err());
        assert!(set_right_tab(&mut session, &json!({ "tab": "nope" })).is_err());
    }
}
