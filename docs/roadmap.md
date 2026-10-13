# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M6 — Procedural content.**

M5 is complete (brush/eraser/smudge/clone/polyfill, screen-space sizing, active-layer target). M6 adds
procedural content and the material shelf:

1. Generators and filters (`substancecraft-material`): noise (Perlin/Simplex/Voronoi), grunge,
   levels, blur, distance, curvature.
2. Effect layers that apply a generator/filter to the layers below (currently pass-through).
3. A material/noise shelf UI with previews.
4. Smart materials and smart masks (saved layer-stack snippets).

See `docs/gaps.md` for the ranked work list.

## M5 deliverables (reference) — done

1. `paint`: brush/eraser/smudge/clone stamps and UV-island `polyfill`; `ToolKind`. ✅
2. `geom`: `uv_density` for screen-space sizing. ✅
3. `engine`: tool state, active-layer target, per-tool strokes, `paint.polyfill`, tool/brush commands. ✅
4. `ui`: tool palette, per-tool brush controls, screen-space sizing, polyfill dispatch. ✅

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
