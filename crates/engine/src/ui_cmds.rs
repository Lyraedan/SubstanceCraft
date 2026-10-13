//! Shell-facing commands that the UI fulfils: window screenshots.

use serde_json::Value;

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The shell commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![CommandSpec::new("ui.screenshot", "Save Screenshot…", "File", "path: string — the output PNG path", screenshot)]
}

/// Register the shell commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn screenshot(session: &mut Session, params: &Value) -> CommandResult {
    let path = params
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| CommandError::InvalidParams { command: "ui.screenshot".to_string(), message: "expected a \"path\" string".to_string() })?;
    if path.trim().is_empty() {
        return Err(CommandError::InvalidParams { command: "ui.screenshot".to_string(), message: "path must not be empty".to_string() });
    }
    session.request_screenshot(path.to_string());
    session.set_status(format!("screenshot queued: {path}"));
    Ok(CommandOutcome::message(format!("screenshot queued: {path}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn queues_a_screenshot_with_a_path() {
        let mut session = Session::new("test");
        screenshot(&mut session, &json!({ "path": "Build/shot.png" })).expect("queue");
        assert_eq!(session.take_screenshot_request().as_deref(), Some("Build/shot.png"));
        assert!(session.take_screenshot_request().is_none());
    }

    #[test]
    fn missing_path_is_a_graceful_error() {
        let mut session = Session::new("test");
        let err = screenshot(&mut session, &Value::Null).expect_err("no path");
        assert!(matches!(err, CommandError::InvalidParams { .. }));
    }
}
