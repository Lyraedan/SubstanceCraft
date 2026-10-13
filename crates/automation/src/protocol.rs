//! Wire types for the control channel: one JSON object per line, both ways.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// One request from a client.
#[derive(Debug, Clone, Deserialize)]
pub struct Request {
    /// The command id, or one of the `system.*` pseudo-commands.
    pub command: String,
    /// The command parameters. Absent means `null`.
    #[serde(default)]
    pub params: Value,
}

/// One response to a client. Errors are data, never a crash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    /// Whether the command succeeded.
    pub ok: bool,
    /// A short human-readable note on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    /// The structured payload on success.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
    /// The error message on failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl Response {
    /// A success carrying a message and/or data.
    pub fn ok(message: Option<String>, data: Option<Value>) -> Self {
        Self { ok: true, message, data, error: None }
    }

    /// A failure carrying a message.
    pub fn error(message: impl Into<String>) -> Self {
        Self { ok: false, message: None, data: None, error: Some(message.into()) }
    }
}
