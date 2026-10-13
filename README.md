# SubstanceCraft

A clean-room, native, Rust-only reimplementation of a Substance-3D-Painter-class 3D texture painting
tool. Paint PBR materials directly on 3D models through a non-destructive, layer-based, procedural
workflow.

- **Pure Rust** — `wgpu` + `egui`/`eframe` + `winit`. No webview, no C/C++ dependencies.
- **Clean-room** — built from public specifications (OpenPBR 1.1, glTF 2.0, MikkTSpace, UDIM).
  No Adobe code, shaders, profiles or assets.
- **Agent-drivable** — every action is a command; a JSON control channel drives the running app.

## Status

Milestone **M0 (bootstrap)** is complete: the workspace, full crate skeleton with enforced layering,
craft-libs integration, the golden-triangle app shell, the command registry and the control channel
all build and pass `cargo xtask ci`.

The next milestone is **M1: 3D viewport + PBR material preview**.

## Build and run

```sh
cargo run -p substancecraft            # the desktop app
cargo run -p substancecraft-cli -- commands
cargo xtask ci                         # layers, fmt, clippy, tests
cargo xtask build                      # release app copied into Build/
```

Requirements: Rust stable ≥ 1.95 (see `rust-toolchain.toml`), edition 2024.

## Documentation

Start with [`AGENTS.md`](AGENTS.md). Architecture and workflow are in [`docs/`](docs/); the plan is
in [`ROADMAP.md`](ROADMAP.md) and [`docs/roadmap.md`](docs/roadmap.md).

## License

MIT OR Apache-2.0. See `LICENSE-MIT`, `LICENSE-APACHE`, `NOTICE` and `ATTRIBUTION.md`.
