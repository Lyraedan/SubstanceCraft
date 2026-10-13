# Development

## Requirements

- Rust stable, version ≥ 1.95 (pinned by `rust-toolchain.toml`; edition 2024, resolver 3).
  Install with `rustup` if missing:
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  ```
- A GPU with Vulkan (Linux/Windows), Metal (macOS) or DX12 (Windows) support for the viewport.

## Build and run

```sh
cargo run -p substancecraft                        # the desktop app
./Build/substancecraft --control 127.0.0.1:8765    # app + JSON control server
cargo run -p substancecraft-cli -- commands        # list command ids
cargo run -p substancecraft-cli -- run app.version
cargo run -p substancecraft-cli -- script run.jsonl
cargo run -p substancecraft-cli -- mcp             # MCP over stdio
```

Offscreen render to a PNG (no window, no focus stealing):

```sh
# snapshot [output] [display-mode] [projection] [env: synthetic|<path>] [model: sphere|torus|<path>]
cargo run -p substancecraft-ui --example snapshot -- Build/shot.png base_color perspective
cargo run -p substancecraft-ui --example snapshot -- Build/hero.png base_color perspective \
    corpus/hdri/studio_small_09_1k.hdr corpus/models/Avocado.glb
```

## The gate

```sh
cargo xtask ci        # layers + fmt --check + clippy -D warnings + test
```

Or individually:

```sh
cargo xtask layers
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Performance and scorecard

```sh
cargo xtask perf        # time scenarios, compare perf/budgets.toml, write perf/baseline.json
cargo xtask scorecard   # regenerate docs/scorecard.md from the tree + baseline
```

Run `perf` from a release build for representative numbers:
`cargo run --release -p xtask -- perf`.

## Test assets

`cargo xtask assets` downloads pinned, SHA-256-verified CC0 assets (a Poly Haven HDRI and the Khronos
Avocado glTF) into the gitignored `corpus/`. Nothing there is committed.

## Packaging

```sh
cargo xtask build     # cargo build --release, then copy the apps into Build/
```

`Build/` is gitignored and holds only the finished applications. Signed per-platform packages are not
produced yet (see `docs/releasing.md`).

## Faster loops

- Check a single crate: `cargo check -p substancecraft-<crate>`.
- Run one test file: `cargo test -p <crate> --test <file>`.
- Parallel agents: use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`).

## Debugging

- `cargo xtask stats` prints crate, source and registered-command counts.
- The control channel (`docs/control-protocol.md`) lists and runs commands, reports state
  (`session.info`, `scene.info`, `viewport.state`) and takes screenshots, so the UI can be driven and
  inspected without a human at the keyboard.
- `RUST_BACKTRACE=1` for a backtrace on an unexpected error (which should never be a panic).
