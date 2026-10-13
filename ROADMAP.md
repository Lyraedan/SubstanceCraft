# SubstanceCraft roadmap

SubstanceCraft is a clean-room, Rust-only reimplementation of a Substance-3D-Painter-class 3D
texture painter. This is the one-page summary; details live in `docs/roadmap.md`.

## Status

**M0 — Bootstrap: complete.** Workspace, 18 layered crates, craft-libs integration, app shell,
command registry and JSON control channel; `cargo xtask ci` green.

**M1 — 3D viewport + PBR preview: core complete.** glTF/OBJ import with tangents, orbit camera,
metal/rough PBR with hemisphere + key IBL, tone mapping and exposure, display modes, a wgpu
viewport hosted in egui, and offscreen snapshot. Loose ends: HDRI/EXR environment, grid/floor,
wireframe/orthographic, `cargo xtask assets`, live `ui.screenshot`.

## Milestones

| # | Milestone | Focus |
|---|---|---|
| **M0** | Bootstrap | workspace, crates, layering, app shell, commands, control channel ✅ |
| **M1** | Viewport + PBR preview | glTF/OBJ import, camera, metal/rough PBR, IBL, tone mapping, display modes ◐ |
| M2 | Texture sets & UV tiles | split by material id, per-set shaders/channels, UDIM |
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
