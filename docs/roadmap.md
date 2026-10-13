# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M5 — Painting tools.**

M4 is complete (brush engine, projection painting, live composited base colour). M5 broadens the tool
set and the paint path:

1. Projection tool (paint an image through a placed plane), polygon fill, smudge and clone.
2. Particle / physics brushes.
3. Eraser tool and an active-layer target in the session.
4. Native pen pressure/tilt and screen-space brush sizing.
5. Streamed/GPU painting for large strokes.

See `docs/gaps.md` for the ranked work list.

## M4 deliverables (reference) — done

1. `paint`: brush engine (dabs, spacing, hardness, pressure, symmetry). ✅
2. `geom`: ray/triangle picking (`Ray`, `raycast`, `Hit`). ✅
3. `engine`: brush + stroke state, paint commands, per-stroke undo. ✅
4. `render`: camera picking ray and a composited base-colour texture sampled by UV. ✅
5. `ui`: Paint/Orbit tool, brush panel, pointer projection painting, live composited result. ✅

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
