# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M2 — Texture sets & UV tiles.**

M1 is complete (import, camera, PBR + HDRI IBL, tone mapping, display modes, grid, wireframe,
offscreen and live screenshots, pinned assets). Next:

1. Split the imported mesh into **texture sets** by material id (and, for UDIM projects, group or
   separate UV tiles).
2. A texture-set list UI that switches the active set and drives per-set channel configuration.
3. Per-set channel selection (base colour, roughness, metalness, normal, height, AO…) using the
   OpenPBR channel set, and per-set shader assignment.
4. UDIM / UV-tile handling so painting can later span tiles seamlessly.

See `docs/gaps.md` for the ranked work list.

## M1 deliverables (reference) — done

1. `substancecraft-geom`: mesh types, OBJ + glTF import, tangents, bounds and focus math. ✅
2. `substancecraft-render` + `substancecraft-gpu`: metal/rough PBR, HDRI IBL, tone mapping, exposure. ✅
3. `substancecraft-color`: colour pipeline and tone-mapping tests. ✅
4. Camera: orbit / pan / dolly / focus; perspective and orthographic; grid. ✅
5. Display modes: base colour, normal, roughness, metalness, UV checker, wireframe. ✅
6. Commands: `file.import_mesh`, `scene.*`, `viewport.*`, `ui.screenshot`. ✅
7. UI: viewport panel hosting the render target; toolbar. ✅
8. Control channel drives the live viewport; `--control`. ✅
9. Offscreen snapshot example plus a smoke test. ✅
10. `cargo xtask assets`: pinned, SHA-256-verified CC0 assets. ✅

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
