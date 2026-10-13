//! Project lifecycle commands: new, open, save, save-as, info and autosave/recovery.

use std::path::PathBuf;

use serde_json::{Value, json};

use crate::command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
use crate::session::Session;

/// The project commands.
pub fn specs() -> Vec<CommandSpec> {
    vec![
        CommandSpec::new("file.new", "New Project", "File", "no parameters", new_project).with_shortcut("Ctrl+N"),
        CommandSpec::new("file.open", "Open Project…", "File", "path: string", open).with_shortcut("Ctrl+O"),
        CommandSpec::new("file.save", "Save", "File", "no parameters", save).with_shortcut("Ctrl+S"),
        CommandSpec::new("file.save_as", "Save As…", "File", "path: string", save_as).with_shortcut("Ctrl+Shift+S"),
        CommandSpec::new("file.info", "Project Info", "File", "no parameters", info),
        CommandSpec::new("file.autosave", "Autosave", "File", "no parameters", autosave),
        CommandSpec::new("file.recover", "Recover Autosave", "File", "no parameters", recover),
    ]
}

/// Register the project commands.
pub fn register(registry: &mut CommandRegistry) -> Result<(), CommandError> {
    for spec in specs() {
        registry.register(spec)?;
    }
    Ok(())
}

fn new_project(session: &mut Session, _params: &Value) -> CommandResult {
    session.new_project();
    Ok(CommandOutcome::message("new project"))
}

fn open(session: &mut Session, params: &Value) -> CommandResult {
    let path = required_path("file.open", params)?;
    session.open(&path).map_err(CommandError::Failed)?;
    Ok(CommandOutcome::message(format!("opened {}", path.display())))
}

fn save(session: &mut Session, _params: &Value) -> CommandResult {
    session.save(None).map_err(CommandError::Failed)?;
    Ok(CommandOutcome::message("saved"))
}

fn save_as(session: &mut Session, params: &Value) -> CommandResult {
    let path = required_path("file.save_as", params)?;
    session.save(Some(path.clone())).map_err(CommandError::Failed)?;
    Ok(CommandOutcome::message(format!("saved {}", path.display())))
}

fn info(session: &mut Session, _params: &Value) -> CommandResult {
    Ok(CommandOutcome::data(json!({
        "path": session.file_path().map(|path| path.to_string_lossy().to_string()),
        "dirty": session.is_dirty(),
        "title": session.project_title(),
    })))
}

fn autosave(session: &mut Session, _params: &Value) -> CommandResult {
    let path = autosave_path();
    session.write_copy(&path).map_err(CommandError::Failed)?;
    // Autosave must not clear the dirty flag: the user has not saved yet.
    session.mark_dirty();
    session.set_status(format!("autosaved to {}", path.display()));
    Ok(CommandOutcome::message(format!("autosaved to {}", path.display())))
}

fn recover(session: &mut Session, _params: &Value) -> CommandResult {
    let path = autosave_path();
    if !path.exists() {
        return Err(CommandError::Failed("no autosave to recover".to_string()));
    }
    session.open(&path).map_err(CommandError::Failed)?;
    Ok(CommandOutcome::message(format!("recovered {}", path.display())))
}

/// The recoverable autosave path (a stable location in the temp directory).
fn autosave_path() -> PathBuf {
    std::env::temp_dir().join(format!("substancecraft_autosave.{}", substancecraft_format::EXTENSION))
}

fn required_path(command: &str, params: &Value) -> Result<PathBuf, CommandError> {
    let path = params.get("path").and_then(Value::as_str).ok_or_else(|| invalid(command, "expected a \"path\" string"))?;
    if path.trim().is_empty() {
        return Err(invalid(command, "path must not be empty"));
    }
    Ok(PathBuf::from(path))
}

fn invalid(command: &str, message: &str) -> CommandError {
    CommandError::InvalidParams { command: command.to_string(), message: message.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::Scene;
    use substancecraft_geom::primitives::uv_sphere;

    fn session_with_scene() -> Session {
        let mut session = Session::new("test");
        session.set_scene(Scene::from_mesh(uv_sphere(1.0, 6, 8).expect("sphere")));
        session
    }

    #[test]
    fn save_as_then_open_round_trips() {
        let mut session = session_with_scene();
        assert!(session.is_dirty());
        let path = std::env::temp_dir().join(format!("sc_proj_{}.{}", std::process::id(), substancecraft_format::EXTENSION));
        save_as(&mut session, &json!({ "path": path.to_string_lossy() })).expect("save");
        assert!(!session.is_dirty());
        assert_eq!(session.file_path(), Some(path.as_path()));

        let mut opened = Session::new("empty");
        open(&mut opened, &json!({ "path": path.to_string_lossy() })).expect("open");
        assert!(!opened.is_dirty());
        assert_eq!(opened.scene().mesh.triangle_count(), session.scene().mesh.triangle_count());

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn new_project_clears_path_and_dirty() {
        let mut session = session_with_scene();
        new_project(&mut session, &Value::Null).expect("new");
        assert!(session.file_path().is_none());
        assert!(!session.is_dirty());
    }

    #[test]
    fn open_missing_file_is_a_graceful_error() {
        let mut session = Session::new("test");
        assert!(matches!(open(&mut session, &json!({ "path": "/definitely/not/here.sccraft" })), Err(CommandError::Failed(_))));
        assert!(open(&mut session, &Value::Null).is_err());
    }

    #[test]
    fn autosave_then_recover() {
        let mut session = session_with_scene();
        let expected = session.scene().mesh.triangle_count();
        let auto = autosave_path();
        autosave(&mut session, &Value::Null).expect("autosave");
        assert!(session.is_dirty(), "autosave must not clear dirty");
        assert!(auto.exists());
        // Start over, then recover.
        new_project(&mut session, &Value::Null).expect("new");
        recover(&mut session, &Value::Null).expect("recover");
        assert_eq!(session.scene().mesh.triangle_count(), expected);
        let _ = std::fs::remove_file(&auto);
    }
}
