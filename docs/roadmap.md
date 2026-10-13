# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M3 — Layer stack.**

M2 is complete (mesh split into texture sets by material; list/select/rename; per-set resolution,
channels and UV tiles; solo isolation). M3 builds the non-destructive layer system:

1. Extend the document model (`substancecraft-doc`) with a per-texture-set **layer stack**: fill,
   paint, effect and group nodes, each with an optional mask.
2. **Blend modes** and per-layer opacity, evaluated per channel.
3. A **CPU compositor** (`substancecraft-compose`) as the oracle, and the **GPU compositor**
   (`substancecraft-gpu`) with wgpu render targets; validate the GPU against the CPU.
4. A **layer-stack UI** (add/remove/reorder/group/visibility/opacity/blend).
5. **Undo/redo** through commands.

See `docs/gaps.md` for the ranked work list.

## M2 deliverables (reference) — done

1. `Project`/`TextureSet`/`Channel`/`UvTile` in `doc`; split by material id. ✅
2. Per-set resolution, channel set and UDIM tile detection/editing. ✅
3. Texture-set commands (list/select/rename/resolution/channel/uv-tile/solo). ✅
4. Texture-set list UI with selection, rename, channels, resolution, tiles, solo. ✅
5. Viewport isolation of the active set (solo). ✅

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
