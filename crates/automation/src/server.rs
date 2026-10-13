//! The control server: one request per line, one response per line, no panics.

use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;

use substancecraft_engine::command::CommandRegistry;
use substancecraft_engine::session::{SharedSession, lock};

use crate::protocol::{Request, Response};

/// The largest request line accepted, to bound allocations from hostile input.
const MAX_REQUEST_BYTES: u64 = 1024 * 1024;

/// Bind a control listener at `addr`.
pub fn bind(addr: &str) -> std::io::Result<TcpListener> {
    TcpListener::bind(addr)
}

/// Serve requests forever, dispatching each to `registry` against `session`.
///
/// Returns only if the listener fails; client errors are reported per-connection.
pub fn serve(listener: TcpListener, registry: Arc<CommandRegistry>, session: SharedSession) -> std::io::Result<()> {
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let registry = Arc::clone(&registry);
                let session = SharedSession::clone(&session);
                std::thread::spawn(move || {
                    if let Err(err) = handle(stream, &registry, &session) {
                        eprintln!("control: connection ended: {err}");
                    }
                });
            }
            Err(err) => eprintln!("control: accept failed: {err}"),
        }
    }
    Ok(())
}

fn handle(stream: TcpStream, registry: &CommandRegistry, session: &SharedSession) -> std::io::Result<()> {
    let writer = stream.try_clone()?;
    let reader = BufReader::new(stream);
    run_stream(reader, writer, registry, session)
}

/// Process newline-delimited requests from `reader`, writing one response line per request to
/// `writer`. Used by the TCP server and by the CLI script runner.
pub fn run_stream<R: BufRead, W: Write>(reader: R, mut writer: W, registry: &CommandRegistry, session: &SharedSession) -> std::io::Result<()> {
    for line in reader.take(MAX_REQUEST_BYTES).lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let response = respond(registry, session, &line);
        let encoded = serde_json::to_string(&response).unwrap_or_else(|_| "{\"ok\":false,\"error\":\"failed to encode response\"}".to_string());
        writer.write_all(encoded.as_bytes())?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }
    Ok(())
}

fn respond(registry: &CommandRegistry, session: &SharedSession, line: &str) -> Response {
    let request: Request = match serde_json::from_str(line) {
        Ok(request) => request,
        Err(err) => return Response::error(format!("invalid request: {err}")),
    };

    match request.command.as_str() {
        crate::PING => Response::ok(Some("pong".to_string()), None),
        crate::LIST_COMMANDS => Response::ok(None, Some(registry.describe())),
        "system.describe" => {
            let id = request.params.get("command").and_then(serde_json::Value::as_str).unwrap_or("");
            match registry.get(id) {
                Some(spec) => Response::ok(
                    None,
                    Some(serde_json::json!({
                        "id": spec.id,
                        "label": spec.label,
                        "menu_path": spec.menu_path,
                        "shortcut": spec.shortcut,
                        "params": spec.params,
                    })),
                ),
                None => Response::error(format!("no such command: {id}")),
            }
        }
        id => {
            let mut session = lock(session);
            match registry.run(id, &mut session, &request.params) {
                Ok(outcome) => Response::ok(outcome.message, outcome.data),
                Err(err) => Response::error(err.to_string()),
            }
        }
    }
}

/// A bound address for tests and callers that want the OS to pick a port.
pub fn local_ephemeral() -> std::io::Result<SocketAddr> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let addr = listener.local_addr()?;
    drop(listener);
    Ok(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_engine::session::shared_session;

    #[test]
    fn pings_without_a_registry() {
        let registry = CommandRegistry::new();
        let session = shared_session("test");
        let response = respond(&registry, &session, "{\"command\":\"system.ping\"}");
        assert!(response.ok);
    }

    #[test]
    fn invalid_json_is_an_error_not_a_panic() {
        let registry = CommandRegistry::new();
        let session = shared_session("test");
        let response = respond(&registry, &session, "not json");
        assert!(!response.ok);
        assert!(response.error.is_some());
    }

    #[test]
    fn lists_commands_as_data() {
        let registry = substancecraft_engine::registry();
        let session = shared_session("test");
        let response = respond(&registry, &session, "{\"command\":\"system.list_commands\"}");
        assert!(response.ok);
        assert!(response.data.is_some());
    }
}
