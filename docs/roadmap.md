# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**Post-M12 backlog.**

All twelve milestones have landed (M9 and M12 in part). The engine is complete enough to paint,
bake and export; the remaining work is polish, depth and release:

1. **Release/packaging** — signed macOS `.app`, Windows installer, Linux tarball; `docs/releasing.md`.
2. **Workflow depth** — distinct OpenPBR/spec-gloss shading models, anchors, layer instancing,
   physical size.
3. **Painting depth** — native pen pressure/tilt, physics brushes, GPU/streamed painting, UDIM-aware
   painting, active-layer eraser.
4. **Content** — smart materials/masks, a grunge library, editable filter/generator parameters.
5. **Performance** — GPU compositor for full stacks, BVH everywhere, a binary project container.

See `docs/gaps.md` for the ranked work list and `perf/budgets.toml` for the enforced budgets.

## M12 deliverables (reference) — done (engine)

1. `cargo xtask perf` — times compose/bake/project round-trip, compares budgets, writes `perf/baseline.json`. ✅
2. `cargo xtask scorecard` — generates `docs/scorecard.md` from the tree + baseline. ✅
3. Viewport re-composites only when the scene/camera/settings/size signature changes. ✅
4. Release packaging/signing. ⏳ (deferred)

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
