# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M11 — Automation.**

M10 is complete (native `.sccraft` project format, save/open/new, autosave/recovery). M11 makes the
whole app agent-drivable end to end:

1. Finish command coverage so every user-visible action is a command with an id, menu path and tests.
2. A JSON control-protocol server shared with the live UI session (currently the app server and the
   UI both hold the session; expose viewport/scene introspection).
3. An MCP server exposing the command registry as tools.
4. The headless CLI: open a project, run commands, bake, export, screenshot.
5. Optional Rust scripting over the command API.

See `docs/gaps.md` for the ranked work list.

## M10 deliverables (reference) — done

1. `format`: versioned `.sccraft` RON, `craft-atomic` writes, `save`/`load`/`to_bytes`/`from_bytes`. ✅
2. serde across `geom`/`raster`/`doc` (glam serde). ✅
3. `engine`: path + dirty state, `save`/`open`/`new_project`/`write_copy`; `project_cmds`. ✅
4. `ui`: File buttons, window title + dirty, autosave timer. ✅
5. Round-trip, version-rejection and autosave/recover tests. ✅

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
