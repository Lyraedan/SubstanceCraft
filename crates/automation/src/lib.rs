//! SubstanceCraft automation: the machine-driving side of "everything is a command".
//!
//! A newline-delimited JSON control channel lets agents and the CLI list commands and dispatch
//! them by id against the running session, exactly as the UI does.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod protocol;
pub mod server;

pub use protocol::{Request, Response};
pub use server::{bind, serve};

/// The pseudo-command that lists every registered command. Not a real command id, so it
/// cannot collide with one.
pub const LIST_COMMANDS: &str = "system.list_commands";

/// The pseudo-command that reports whether the control channel is healthy.
pub const PING: &str = "system.ping";
