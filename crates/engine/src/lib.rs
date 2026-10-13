//! SubstanceCraft engine: the `Session` and the command registry.
//!
//! Golden rule: **everything is a command.** New user-visible behaviour is a command with an
//! id, label, menu path, shortcut, parameter documentation and a `run` closure. The UI, the
//! CLI, the control channel and the MCP server all dispatch commands by id, so behaviour is
//! identical everywhere and drivable by agents.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod bake_cmds;
pub mod command;
pub mod commands;
pub mod layer_cmds;
pub mod paint_cmds;
pub mod scene_cmds;
pub mod session;
pub mod texture_set_cmds;
pub mod ui_cmds;
pub mod viewport_cmds;

pub use command::{CommandError, CommandOutcome, CommandRegistry, CommandResult, CommandSpec};
pub use session::Session;

/// Build the registry with every built-in command registered.
pub fn registry() -> CommandRegistry {
    let mut registry = CommandRegistry::new();
    commands::register_builtin(&mut registry);
    registry
}
