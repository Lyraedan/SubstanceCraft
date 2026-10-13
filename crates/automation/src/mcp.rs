//! A Model Context Protocol (MCP) server over stdio.
//!
//! Exposes the command registry as MCP tools: `tools/list` lists every command and `tools/call`
//! dispatches it against the shared session. Messages are newline-delimited JSON-RPC 2.0, the MCP
//! stdio transport.

use std::io::{self, BufRead, Write};
use std::sync::Arc;

use serde_json::{Value, json};
use substancecraft_engine::command::CommandRegistry;
use substancecraft_engine::session::{SharedSession, lock};

/// The MCP protocol version this server speaks.
pub const PROTOCOL_VERSION: &str = "2024-11-05";
/// The server name reported to clients.
pub const SERVER_NAME: &str = "substancecraft";
/// The server version.
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");

/// A command id to an MCP tool name (MCP names allow only alphanumerics, `_` and `-`).
pub fn tool_name(id: &str) -> String {
    id.replace('.', "_")
}

/// Every command as an MCP tool descriptor.
fn tools(registry: &CommandRegistry) -> Vec<Value> {
    registry
        .describe()
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let id = entry.get("id")?.as_str()?.to_string();
            let label = entry.get("label").and_then(Value::as_str).unwrap_or("");
            let menu = entry.get("menu_path").and_then(Value::as_str).unwrap_or("");
            let params = entry.get("params").and_then(Value::as_str).unwrap_or("");
            let description = format!("{label} ({menu}) — params: {params}");
            Some(json!({
                "name": tool_name(&id),
                "description": description,
                "inputSchema": { "type": "object", "additionalProperties": true, "description": params },
            }))
        })
        .collect()
}

/// Handle one JSON-RPC message, returning the response (or `None` for notifications).
pub fn handle(registry: &CommandRegistry, session: &SharedSession, request: &Value) -> Option<Value> {
    let method = request.get("method").and_then(Value::as_str)?;

    // Notifications carry no id and must not be answered.
    if let Some(id) = request.get("id").cloned() {
        match method {
            "initialize" => Some(result(
                id,
                json!({
                    "protocolVersion": PROTOCOL_VERSION,
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": SERVER_NAME, "version": SERVER_VERSION },
                }),
            )),
            "ping" => Some(result(id, json!({}))),
            "tools/list" => Some(result(id, json!({ "tools": tools(registry) }))),
            "tools/call" => Some(tools_call(registry, session, id, request.get("params"))),
            _ => Some(error(id, -32601, "method not found")),
        }
    } else {
        // Notifications (`notifications/initialized`, etc.) are acknowledged silently.
        None
    }
}

fn tools_call(registry: &CommandRegistry, session: &SharedSession, id: Value, params: Option<&Value>) -> Value {
    let Some(params) = params else {
        return error(id, -32602, "missing params");
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return error(id, -32602, "missing tool name");
    };
    let Some(command_id) = registry.ids().find(|command| tool_name(command) == name) else {
        return error(id, -32602, &format!("no tool named {name}"));
    };
    let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

    let mut guard = lock(session);
    match registry.run(command_id, &mut guard, &arguments) {
        Ok(outcome) => {
            let text = match (outcome.message, outcome.data) {
                (Some(message), Some(data)) => format!("{message}\n{}", serde_json::to_string_pretty(&data).unwrap_or_default()),
                (Some(message), None) => message,
                (None, Some(data)) => serde_json::to_string_pretty(&data).unwrap_or_default(),
                (None, None) => "ok".to_string(),
            };
            result(id, json!({ "content": [ { "type": "text", "text": text } ], "isError": false }))
        }
        Err(err) => result(id, json!({ "content": [ { "type": "text", "text": err.to_string() } ], "isError": true })),
    }
}

fn result(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn error(id: Value, code: i64, message: &str) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
}

/// Serve MCP over stdin/stdout until EOF.
pub fn serve_stdio(registry: Arc<CommandRegistry>, session: SharedSession) -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(request) => {
                if let Some(response) = handle(&registry, &session, &request) {
                    writeln!(stdout, "{response}")?;
                    stdout.flush()?;
                }
            }
            Err(err) => {
                let response = error(Value::Null, -32700, &format!("parse error: {err}"));
                writeln!(stdout, "{response}")?;
                stdout.flush()?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_engine::session::shared_session;

    fn context() -> (CommandRegistry, SharedSession) {
        (substancecraft_engine::registry(), shared_session("mcp-test"))
    }

    #[test]
    fn initialize_reports_capabilities() {
        let (registry, session) = context();
        let response = handle(&registry, &session, &json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize" })).expect("response");
        assert_eq!(response["result"]["serverInfo"]["name"], SERVER_NAME);
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn lists_tools_and_round_trips_names() {
        let (registry, session) = context();
        let response = handle(&registry, &session, &json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })).expect("response");
        let tools = response["result"]["tools"].as_array().expect("tools");
        assert!(!tools.is_empty());
        assert!(tools.iter().any(|tool| tool["name"] == "app_version"));
        // Names must be MCP-legal.
        for tool in tools {
            let name = tool["name"].as_str().expect("name");
            assert!(name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'));
        }
    }

    #[test]
    fn calls_a_tool() {
        let (registry, session) = context();
        let request = json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "app_version" } });
        let response = handle(&registry, &session, &request).expect("response");
        assert_eq!(response["result"]["isError"], false);
        assert!(response["result"]["content"][0]["text"].as_str().unwrap_or_default().contains("SubstanceCraft"));
    }

    #[test]
    fn calling_an_unknown_tool_is_an_error_not_a_panic() {
        let (registry, session) = context();
        let request = json!({ "jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": { "name": "does_not_exist" } });
        let response = handle(&registry, &session, &request).expect("response");
        assert_eq!(response["error"]["code"], -32602);
    }

    #[test]
    fn unknown_method_and_notifications() {
        let (registry, session) = context();
        let response = handle(&registry, &session, &json!({ "jsonrpc": "2.0", "id": 5, "method": "wat" })).expect("response");
        assert_eq!(response["error"]["code"], -32601);
        assert!(handle(&registry, &session, &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" })).is_none());
    }
}
