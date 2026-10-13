# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M10 — Project lifecycle.**

M9 is partially complete (per-set shaders + params, post-effects; anchors/instancing/physical-size
deferred). M10 adds the native project format:

1. `substancecraft-format`: serialise the whole `Project` (scene, texture sets, layer stacks, shader,
   paint images) to a crash-safe `.sccraft` file via `craft-atomic`.
2. Save / open / save-as commands; the app title and dirty state follow the file.
3. Project templates and a New Project window.
4. Autosave and recovery of a damaged/partial file.

See `docs/gaps.md` for the ranked work list.

## M9 deliverables (reference) — partial

1. `doc`: `Shader`, `ShaderParams` on `TextureSet`. ✅
2. `engine`: `texture_set.set_shader`, `shader.set`, `viewport.set_vignette/bloom`. ✅
3. `render`: material override (per-set shader params) + a post-effects pass (vignette + bloom). ✅
4. `ui`: shader combo + params; Post sliders. ✅
5. Anchors, layer instancing, physical size, distinct shading models. ⏳

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
