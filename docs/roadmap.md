# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M12 — Polish / release.**

M11 is complete (control channel, MCP server, headless CLI). M12 prepares a release:

1. A performance harness (`cargo xtask perf`) with budgets and recorded baselines; make the viewport
   render on revision change rather than every frame.
2. A generated scorecard (`cargo xtask scorecard`) with measured numbers.
3. Packaging: `cargo xtask build` producing a macOS `.app`, a Windows installer and a Linux tarball,
   codesigned for distribution.
4. Final parity sweep across `docs/*-parity.md` and the gap list.

See `docs/gaps.md` for the ranked work list.

## M11 deliverables (reference) — done

1. `automation::mcp`: MCP over stdio (`initialize`/`tools/list`/`tools/call`). ✅
2. `automation::server`: shared `run_stream` + `system.describe`. ✅
3. `engine::info_cmds`: `session.info`, `scene.info`, `viewport.state`, `app.capabilities`. ✅
4. `substancecraft-cli`: `script <file>` and `mcp` subcommands. ✅
5. `docs/control-protocol.md` covers the channel, MCP and scripting. ✅

Acceptance: `cargo run -p substancecraft` opens the window, imports a sample glTF/OBJ, orbits with a
real camera, PBR-shades it with IBL and tone mapping, switches display modes — all via commands; the
headless snapshot matches; `cargo xtask ci` is green.

## Later milestones

- **M2 Texture sets & UV tiles** — split the mesh by material id, texture-set list UI, per-set
  channel configuration and shader assignment, UDIM handling.
- **M3 Layer stack** — document model, fill layers, masks, groups and blend modes; the CPU oracle and
  the wgpu compositor agree; layer-stack UI; undo/redo through commands.
- **M4 Painting** — brush engine (dabs, spacing, pressure/tilt), projection painting into the active
  layer's channel targets, eraser, colour picker, brush presets, symmetry.
- **M5 Painting tools** — projection, polyfill, smudge, clone, particle/physics brushes.
- **M6 Procedural content** — generators and filters, the noise/grunge library, smart materials and
  smart masks, material shelf.
- **M7 Baking** — mesh-map bakers (normal, AO, curvature, position, thickness, WS-normal, ID), cage
  generation, baking-mode UI, bake log.
- **M8 Export** — export presets, channel configuration, naming conventions, UDIM export, OpenPBR
  material mapping.
- **M9 Workflow depth** — OpenPBR and legacy shaders, shader parameters, post-effects, display
  settings, anchors, layer instancing, physical size.
- **M10 Project lifecycle** — `.sccraft` save/load, project templates, autosave and recovery.
- **M11 Automation** — full command coverage, MCP server, headless CLI workflows, Rust scripting.
- **M12 Polish/release** — performance budgets and scorecard, parity docs, signed packages for
  macOS/Windows/Linux.
