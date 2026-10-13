# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M7 — Baking.**

M6 is complete (generators/filters, effect layers, shelf). M7 adds mesh-map baking:

1. `substancecraft-bake`: ray-based bakers onto the low-poly UV layout — tangent-space normal, ambient
   occlusion, curvature, position, thickness, world-space normal, material ID.
2. Cage generation (implicit) for high-to-low projection.
3. Baking-mode UI (baker selection, settings, bake log) and a `file.import_mesh` high-poly slot.
4. Validated against the CPU oracle; reproducible at several resolutions.

See `docs/gaps.md` for the ranked work list.

## M6 deliverables (reference) — done

1. `doc`: `Generator`, `Filter`, `Effect` data + `LayerKind::Effect`. ✅
2. `material`: Perlin/Simplex/Voronoi/White generators; levels/blur/invert/contrast/brightness filters. ✅
3. `compose`: effect layers (generator fill, filter transform) with opacity. ✅
4. `engine`: `layer.add_generator` / `layer.add_filter`. ✅
5. `ui`: shelf panel with generators and filters. ✅

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
