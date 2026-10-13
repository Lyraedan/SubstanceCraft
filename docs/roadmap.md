# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M4 — Painting.**

M3 is complete (non-destructive layer stack, CPU/GPU compositors, layer UI, undo/redo). M4 adds the
brush engine and paints into layer targets:

1. Brush engine in `substancecraft-paint`: dabs, spacing, size/flow/opacity, pressure and tilt
   curves, jitter, symmetry.
2. Projection painting: map screen strokes through the mesh UVs into the active layer's channel
   target.
3. Tablet input (`substancecraft-tablet`) for pressure/tilt; wire it into the viewport.
4. Feed the composited channels into the 3D viewport material so the painted result shows live.
5. Painterly controls: eraser, colour picker, stroke smoothing, brush presets.

See `docs/gaps.md` for the ranked work list.

## M3 deliverables (reference) — done

1. `doc`: `Layer`, `LayerKind` (fill/paint/effect/group), `Mask`, `BlendMode`, `LayerStack`. ✅
2. CPU compositor oracle (per channel) with tests. ✅
3. GPU compositor for flat fill stacks, validated against the CPU. ✅
4. Layer commands (add/delete/move/group/property/undo/redo). ✅
5. Layer-stack UI with a live composite preview. ✅

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
