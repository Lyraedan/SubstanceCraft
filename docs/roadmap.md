# Roadmap (detailed)

Milestones and the current focus. The one-page summary is `ROADMAP.md`.

## Current focus

**M9 — Workflow depth.**

M8 is complete (channel export with presets/packing/naming/UDIM). M9 deepens the material workflow:

1. OpenPBR and legacy (metal-rough / spec-gloss / ASM-style) shaders with editable parameters,
   selected per texture set (shader instances).
2. Shader parameters feeding the viewport and export.
3. Post-effects (tone mapping variants, bloom, vignette) as a post-process chain.
4. Anchor points and layer instancing across texture sets.
5. Physical-size scaling for UV projection.

See `docs/gaps.md` for the ranked work list.

## M8 deliverables (reference) — done

1. `codecs`: PNG 8-bit/16-bit, TGA, EXR writers and a raw RGBA loader. ✅
2. `io::export`: formats, presets, packing, naming, UDIM naming, `export_set`. ✅
3. `engine`: `file.export_textures`, `export.presets`, `export.formats`. ✅
4. `ui`: Export button. ✅
5. Round-trip test. ✅

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
