# SubstanceCraft

A clean-room, native, Rust-only reimplementation of a Substance-3D-Painter-class 3D texture painting
tool. Paint PBR materials directly on 3D models through a non-destructive, layer-based, procedural
workflow.

- **Pure Rust** — `wgpu` + `egui`/`eframe` + `winit`, no webview and no C/C++ dependencies.
- **Clean-room** — built from public specifications (OpenPBR 1.1, glTF 2.0, MikkTSpace, UDIM).
  No Adobe code, shaders, profiles or assets.
- **Agent-drivable** — every action is a command; a JSON control channel and an MCP server drive the
  running app.

  <img width="2880" height="1556" alt="m12_render2" src="https://github.com/user-attachments/assets/2e83db2e-522e-4b59-a6eb-de25d6359fd4" />

## Features

- **Viewport** — glTF/OBJ import, orbit/pan/dolly/focus camera (perspective + orthographic), metal/
  rough PBR with HDRI image-based lighting (`.hdr`/`.exr`), tone mapping, display modes, ground grid,
  wireframe, and post-effects (bloom, vignette).
- **Texture sets** — the mesh is split by material id; per-set resolution, OpenPBR channel set,
  UDIM/UV-tile detection and solo isolation.
- **Layer stack** — non-destructive fill/paint/effect/group layers with masks, opacity, 8 blend modes
  and undo/redo; a CPU compositor oracle and a GPU compositor.
- **Painting** — brush, eraser, smudge, clone and UV-island polyfill with screen-space sizing,
  pressure, symmetry and jitter; projection painting through BVH raycasts.
- **Procedural content** — Perlin/Simplex/Voronoi/white generators and levels/blur/invert/contrast/
  brightness filters as effect layers, with a shelf.
- **Baking** — BVH-accelerated bakers (position, world/tangent normal, AO, thickness, curvature,
  material ID) with high-poly import.
- **Export** — per-channel PNG 8/16-bit, TGA and EXR with presets (OpenPBR/glTF/Unreal/Unity/Spec-
  Gloss), channel packing, naming templates and UDIM naming.
- **Projects** — a crash-safe, versioned `.sccraft` format with save/open/as-new, autosave and recovery.
- **Automation** — 85 commands, a JSON control channel, a Model Context Protocol server, and a
  headless CLI with a script runner.

## Status

All twelve milestones have landed. M9 (workflow depth) and M12 (polish) are partial: distinct
OpenPBR/spec-gloss shading models, anchors/instancing, native pen pressure and release packaging
remain. See [`docs/gaps.md`](docs/gaps.md) for the ranked work list and
[`docs/target-app-parity.md`](docs/target-app-parity.md) for where we stand against Painter.

## Build and run

```sh
cargo run -p substancecraft                 # the desktop app
./Build/substancecraft --control 127.0.0.1:8765   # app + JSON control server
cargo run -p substancecraft-cli -- commands      # list commands
cargo run -p substancecraft-cli -- script run.jsonl
cargo run -p substancecraft-cli -- mcp           # MCP over stdio
cargo run -p substancecraft-ui --example snapshot -- Build/shot.png base_color perspective
cargo xtask ci                               # layers, fmt, clippy, tests
cargo xtask build                            # release app copied into Build/
cargo xtask perf                             # time scenarios, write perf/baseline.json
cargo xtask scorecard                        # regenerate docs/scorecard.md
```

Requirements: Rust stable ≥ 1.95 (see `rust-toolchain.toml`), edition 2024.

## Documentation

Start with [`AGENTS.md`](AGENTS.md). Architecture and workflow are in [`docs/`](docs/); the plan and
status are in [`ROADMAP.md`](ROADMAP.md) and [`docs/roadmap.md`](docs/roadmap.md).

## License

MIT OR Apache-2.0. See `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and `ATTRIBUTION.md`.
