# SubstanceCraft roadmap

SubstanceCraft is a clean-room, Rust-only reimplementation of a Substance-3D-Painter-class 3D
texture painter. This is the one-page summary; details live in `docs/roadmap.md`.

## Status

**M0 — Bootstrap: complete.** Workspace, 18 layered crates, craft-libs integration, app shell,
command registry and JSON control channel.

**M1 — 3D viewport + PBR preview: complete.** glTF/OBJ import with tangents; orbit/pan/dolly/focus
camera with perspective and orthographic projection; metal/rough PBR with HDRI image-based lighting
(`.hdr`/`.exr`) and a hemisphere fallback; tone mapping and exposure; display modes (base colour,
normal, roughness, metalness, UV checker, wireframe); ground grid and sky/environment background; a
wgpu viewport hosted in egui; offscreen snapshot plus live `ui.screenshot`; pinned CC0 assets via
`cargo xtask assets`.

**M8 — Export: complete.** Channel export (PNG 8/16-bit, TGA, EXR), export presets
(OpenPBR/glTF/Unreal/Unity/Spec-Gloss), channel packing, naming templates and UDIM naming, a round-trip
test, and an Export button. 70 commands.

## Milestones

| # | Milestone | Focus |
|---|---|---|
| **M0** | Bootstrap | workspace, crates, layering, app shell, commands, control channel ✅ |
| **M1** | Viewport + PBR preview | import, camera, PBR, IBL, tone mapping, display modes, grid ✅ |
| **M2** | Texture sets & UV tiles | split by material, per-set channels/resolution/UDIM, solo ✅ |
| **M3** | Layer stack | fill/group layers, masks, blend modes, CPU + GPU compositor, undo ✅ |
| **M4** | Painting | brush engine, projection painting, live composited base colour ✅ |
| **M5** | Painting tools | brush/eraser/smudge/clone/polyfill, screen sizing, active layer ✅ |
| **M6** | Procedural content | generators, filters, effect layers, shelf ✅ |
| **M7** | Baking | BVH mesh-map bakers, high-poly import, bake panel + log ✅ |
| **M8** | Export | formats, presets, packing, naming, UDIM, round-trip ✅ |
| M9 | Workflow depth | OpenPBR/ASM shaders, shader params, post-effects, anchors, instancing |
| M3 | Layer stack | doc model, fill/mask/group/blend, CPU oracle + GPU compositor, undo |
| M4 | Painting | brush engine, tablet pressure, projection painting |
| M5 | Painting tools | projection, polyfill, smudge, clone, particle/physics brushes |
| M6 | Procedural content | generators/filters, noise library, smart materials, shelf |
| M7 | Baking | mesh-map bakers + cage generation, baking mode UI, bake log |
| M8 | Export | presets, channel config, naming conventions, UDIM, OpenPBR mapping |
| M9 | Workflow depth | OpenPBR/ASM shaders, shader params, post-effects, anchors, instancing |
| M10 | Project lifecycle | `.sccraft` save/load, templates, autosave/recovery |
| M11 | Automation | full command coverage, MCP, headless CLI, Rust scripting |
| M12 | Polish/release | scorecard, perf budgets, signed packages (macOS/Windows/Linux) |

## Principles

Pure Rust; clean-room; never crash; everything is a command; thin data-driven UI; layered crates;
tests are the gate; performance is a feature.
