# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M8 — Export.**

M7 is complete (BVH bakers, high-poly import, bake panel). M8 exports the composited texture set:

1. `substancecraft-io`: export each channel of a texture set to 8/16-bit PNG/TGA and 16/32-bit EXR at
   a chosen resolution.
2. Export presets (Unreal, Unity, glTF, OpenPBR, spec/gloss) and per-channel packing.
3. Naming conventions (`$textureSet_$channel`) and output templates/subfolders.
4. UDIM / UV-tile export.
5. A round-trip test: exported maps re-import to identical values.

See `docs/gaps.md` for the ranked work list.

## M7 deliverables (reference) — done

1. `geom`: `Bvh` (median-split build, stack traversal) with a brute-force cross-check. ✅
2. `bake`: position, world/tangent normal, AO, thickness, curvature, material ID. ✅
3. `engine`: high-poly import, bake settings, `bake.run/list/save`. ✅
4. `ui`: bake panel (resolution, samples, high-poly, run, per-map save) and log. ✅

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
