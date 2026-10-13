//! The command registry: metadata plus a `run` closure per command.

use std::collections::BTreeMap;

use serde_json::{Map, Value};
use thiserror::Error;

use crate::session::Session;

/// What can go wrong dispatching a command. Every variant maps to an actionable message;
/// no command failure is allowed to panic.
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("no such command: {0}")]
    Unknown(String),
    #[error("command {0} is already registered")]
    Duplicate(String),
    #[error("invalid parameters for {command}: {message}")]
    InvalidParams { command: String, message: String },
    #[error("command {0} is disabled in the current state")]
    Disabled(String),
    #[error("not supported yet: {0}")]
    Unsupported(String),
    #[error("{0}")]
    Failed(String),
}

/// The data a successful command may return to its caller.
#[derive(Debug, Clone, Default)]
pub struct CommandOutcome {
    /// A short human-readable note (shown in the status bar / control response).
    pub message: Option<String>,
    /// Optional structured payload (returned over the control channel).
    pub data: Option<Value>,
}

impl CommandOutcome {
    /// A plain acknowledgement.
    pub fn ok() -> Self {
        Self::default()
    }

    /// An acknowledgement with a message.
    pub fn message(message: impl Into<String>) -> Self {
        Self { message: Some(message.into()), data: None }
    }

    /// An acknowledgement with structured data.
    pub fn data(data: Value) -> Self {
        Self { message: None, data: Some(data) }
    }

    /// An acknowledgement with both.
    pub fn message_and_data(message: impl Into<String>, data: Value) -> Self {
        Self { message: Some(message.into()), data: Some(data) }
    }
}

/// A command's result. Commands return `Err`, never panic, for any parameters or state.
pub type CommandResult = Result<CommandOutcome, CommandError>;

type RunFn = Box<dyn Fn(&mut Session, &Value) -> CommandResult + Send + Sync>;
type EnabledFn = Box<dyn Fn(&Session) -> bool + Send + Sync>;

/// One command: stable id, display metadata and the behaviour.
pub struct CommandSpec {
    /// Stable, dotted, lowercase id, e.g. `viewport.focus_selection`. Never localised.
    pub id: &'static str,
    /// Human-readable label for menus and toolbars.
    pub label: &'static str,
    /// Menu path, e.g. `View > Display`. Empty for commands with no menu home.
    pub menu_path: &'static str,
    /// Keyboard shortcut, in the app's accelerator syntax. `None` if unbound.
    pub shortcut: Option<&'static str>,
    /// One-line documentation of the accepted parameters.
    pub params: &'static str,
    /// Whether the command is currently available. Defaults to always.
    pub enabled: Option<EnabledFn>,
    /// The behaviour.
    pub run: RunFn,
}

impl CommandSpec {
    /// A command that is always available, built from an infallible closure.
    pub fn new(
        id: &'static str,
        label: &'static str,
        menu_path: &'static str,
        params: &'static str,
        run: impl Fn(&mut Session, &Value) -> CommandResult + Send + Sync + 'static,
    ) -> Self {
        Self { id, label, menu_path, shortcut: None, params, enabled: None, run: Box::new(run) }
    }

    /// Mark the command as enabled only when `enabled` returns true.
    pub fn with_enabled(mut self, enabled: impl Fn(&Session) -> bool + Send + Sync + 'static) -> Self {
        self.enabled = Some(Box::new(enabled));
        self
    }

    /// Bind a keyboard shortcut.
    pub fn with_shortcut(mut self, shortcut: &'static str) -> Self {
        self.shortcut = Some(shortcut);
        self
    }

    /// Whether the command can run in the given session.
    pub fn is_enabled(&self, session: &Session) -> bool {
        self.enabled.as_ref().is_none_or(|enabled| enabled(session))
    }
}

/// The registry of all commands, keyed by id.
#[derive(Default)]
pub struct CommandRegistry {
    commands: BTreeMap<&'static str, CommandSpec>,
}

impl CommandRegistry {
    /// An empty registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a command. Duplicate ids are rejected rather than silently overwriting.
    pub fn register(&mut self, spec: CommandSpec) -> Result<(), CommandError> {
        if self.commands.contains_key(spec.id) {
            return Err(CommandError::Duplicate(spec.id.to_string()));
        }
        self.commands.insert(spec.id, spec);
        Ok(())
    }

    /// Look up a command by id.
    pub fn get(&self, id: &str) -> Option<&CommandSpec> {
        self.commands.get(id)
    }

    /// Every command id, in lexicographic order.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.commands.keys().copied()
    }

    /// The number of registered commands.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Whether no commands are registered.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }

    /// Whether a command exists and is enabled for the session.
    pub fn is_enabled(&self, id: &str, session: &Session) -> bool {
        self.commands.get(id).is_some_and(|spec| spec.is_enabled(session))
    }

    /// Dispatch a command by id. Unknown ids and disabled commands return an error; the
    /// command's own `run` decides the rest.
    pub fn run(&self, id: &str, session: &mut Session, params: &Value) -> CommandResult {
        let spec = self.commands.get(id).ok_or_else(|| CommandError::Unknown(id.to_string()))?;
        if !spec.is_enabled(session) {
            return Err(CommandError::Disabled(id.to_string()));
        }
        (spec.run)(session, params)
    }

    /// List every command's metadata as JSON, for the control channel and menus.
    pub fn describe(&self) -> Value {
        let entries: Vec<Value> = self
            .commands
            .values()
            .map(|spec| {
                let mut entry = Map::new();
                entry.insert("id".into(), Value::String(spec.id.to_string()));
                entry.insert("label".into(), Value::String(spec.label.to_string()));
                entry.insert("menu_path".into(), Value::String(spec.menu_path.to_string()));
                entry.insert("params".into(), Value::String(spec.params.to_string()));
                if let Some(shortcut) = spec.shortcut {
                    entry.insert("shortcut".into(), Value::String(shortcut.to_string()));
                }
                Value::Object(entry)
            })
            .collect();
        Value::Array(entries)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registers_and_runs_a_command() {
        let mut registry = CommandRegistry::new();
        registry.register(CommandSpec::new("test.noop", "No-op", "", "", |_session, _params| Ok(CommandOutcome::message("done")))).expect("register");
        let mut session = Session::new("test");
        let outcome = registry.run("test.noop", &mut session, &Value::Null).expect("run");
        assert_eq!(outcome.message.as_deref(), Some("done"));
    }

    #[test]
    fn duplicate_ids_are_rejected() {
        let mut registry = CommandRegistry::new();
        let make = || CommandSpec::new("test.dup", "Dup", "", "", |_s, _p| Ok(CommandOutcome::ok()));
        registry.register(make()).expect("first");
        let err = registry.register(make()).expect_err("second must fail");
        assert!(matches!(err, CommandError::Duplicate(_)));
    }

    #[test]
    fn unknown_command_is_an_error_not_a_panic() {
        let registry = CommandRegistry::new();
        let mut session = Session::new("test");
        let err = registry.run("does.not.exist", &mut session, &Value::Null).expect_err("unknown");
        assert!(matches!(err, CommandError::Unknown(_)));
    }

    #[test]
    fn disabled_command_is_rejected() {
        let mut registry = CommandRegistry::new();
        registry
            .register(CommandSpec::new("test.disabled", "Disabled", "", "", |_s, _p| Ok(CommandOutcome::ok())).with_enabled(|_s| false))
            .expect("register");
        let mut session = Session::new("test");
        let err = registry.run("test.disabled", &mut session, &Value::Null).expect_err("disabled");
        assert!(matches!(err, CommandError::Disabled(_)));
    }
}
