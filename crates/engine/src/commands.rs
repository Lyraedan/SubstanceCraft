//! Built-in commands. Each area of the app adds its commands in a focused module here as
//! milestones land; this file only wires them together.
//!
//! For now it holds the handful of application-level commands that let the golden triangle
//! (window + command dispatch) work end to end.

use serde_json::json;

use crate::command::{CommandOutcome, CommandRegistry, CommandSpec};

/// Register every built-in command. Registration errors indicate a programming mistake
/// (a duplicate id) and are surfaced as `Err`, never a panic.
pub fn register_builtin(registry: &mut CommandRegistry) {
    // Built-in ids are unique by construction; if that ever changes, the test below fails
    // before users see it.
    let _ = register(registry);
}

fn register(registry: &mut CommandRegistry) -> Result<(), crate::command::CommandError> {
    crate::scene_cmds::register(registry)?;
    crate::viewport_cmds::register(registry)?;
    crate::texture_set_cmds::register(registry)?;
    crate::layer_cmds::register(registry)?;
    crate::paint_cmds::register(registry)?;
    crate::ui_cmds::register(registry)?;

    registry.register(
        CommandSpec::new("app.version", "About SubstanceCraft", "Help", "no parameters", |_session, _params| {
            Ok(CommandOutcome::data(json!({
                "name": "SubstanceCraft",
                "version": env!("CARGO_PKG_VERSION"),
            })))
        })
        .with_shortcut("F1"),
    )?;

    registry.register(CommandSpec::new("app.quit", "Quit", "File", "no parameters", |_session, _params| {
        Err(crate::command::CommandError::Unsupported("quit is handled by the app shell".into()))
    }))?;

    registry.register(CommandSpec::new("viewport.status", "Report Status", "Help", "no parameters", |session, _params| {
        Ok(CommandOutcome::message(format!("{} @ r{}", session.label(), session.revision())))
    }))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Session;
    use serde_json::Value;

    #[test]
    fn builtin_ids_are_unique_and_registered() {
        let mut registry = CommandRegistry::new();
        register(&mut registry).expect("unique builtins");
        assert!(registry.get("app.version").is_some());
        assert!(registry.get("app.quit").is_some());
        assert!(registry.get("viewport.status").is_some());
    }

    #[test]
    fn version_reports_the_product_name() {
        let registry = crate::registry();
        let mut session = Session::new("test");
        let outcome = registry.run("app.version", &mut session, &Value::Null).expect("version");
        let data = outcome.data.expect("data");
        assert_eq!(data["name"], "SubstanceCraft");
    }
}
