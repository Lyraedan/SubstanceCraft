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

## Example session

```sh
printf '%s\n' \
  '{"command":"system.ping"}' \
  '{"command":"system.list_commands"}' \
  '{"command":"app.version"}' | nc 127.0.0.1:8765
```

## Roadmap

The channel grows with the milestones: document and viewport inspection, command dispatch against
the live UI session, offscreen screenshots (`ui.screenshot`) and an MCP server.
