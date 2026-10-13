# Control protocol

SubstanceCraft exposes a newline-delimited JSON control channel. One request object per line, one
response object per line. It is how agents and the CLI inspect and drive the running editor: the
same commands the UI dispatches, addressed by id.

## Starting the channel

```sh
# The desktop app, with the control channel on a background thread:
./Build/substancecraft --control 127.0.0.1:8765

# Or headless:
substancecraft-cli serve --addr 127.0.0.1:8765
```

`--control` with no address defaults to `127.0.0.1:8765`.

## Requests

```json
{ "command": "app.version", "params": null }
```

`command` is a command id (or a `system.*` pseudo-command). `params` is any JSON value the command
documents; it defaults to `null`. A request line is capped at 1 MiB.

## Responses

```json
{ "ok": true, "message": "done", "data": { "name": "SubstanceCraft", "version": "0.1.0" } }
{ "ok": false, "error": "no such command: bogus" }
```

`message` and `data` are omitted when absent; `error` is present only on failure. A malformed request
or an unknown command is an error response, never a crash.

## Pseudo-commands

| Command | Returns |
|---|---|
| `system.ping` | `{ "ok": true, "message": "pong" }` |
| `system.list_commands` | `data`: an array of `{ id, label, menu_path, params, shortcut? }` |
| `system.describe` | `params: { "command": "<id>" }` → metadata for one command |

## Introspection commands

| Command | Returns |
|---|---|
| `session.info` | label, revision, dirty, path, title |
| `scene.info` | vertices, triangles, materials, texture sets, bounds |
| `viewport.state` | camera, projection, display mode, tone map, exposure, grid/background, post-effects |
| `app.capabilities` | name, version, active texture set |

## Example session

```sh
printf '%s\n' \
  '{"command":"system.ping"}' \
  '{"command":"system.list_commands"}' \
  '{"command":"scene.load_default"}' \
  '{"command":"scene.info"}' \
  '{"command":"app.version"}' | nc 127.0.0.1:8765
```

## Headless script runner

`substancecraft-cli script <file>` (or stdin) executes newline-delimited requests against one
session and prints one response per line — useful for reproducible, scriptable workflows:

```sh
substancecraft-cli script build/scene.jsonl
printf '{"command":"bake.run"}\n{"command":"bake.save","params":{"map":"curvature","path":"out/curv.png"}}\n' \
  | substancecraft-cli script
```

## Model Context Protocol (MCP)

`substancecraft-cli mcp` serves MCP over stdin/stdout: newline-delimited JSON-RPC 2.0 with
`initialize`, `tools/list` and `tools/call`. Every command is a tool (id `.` becomes `_`, e.g.
`file.save` → `file_save`); `tools/call` dispatches it against the session.

```sh
printf '%s\n' \
  '{"jsonrpc":"2.0","id":1,"method":"initialize"}' \
  '{"jsonrpc":"2.0","id":2,"method":"tools/list"}' \
  '{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"app_version"}}' \
  | substancecraft-cli mcp
```

## Roadmap

The channel grows with the milestones: live UI screenshots (`ui.screenshot`) are already available
via the app's `--control` server.
