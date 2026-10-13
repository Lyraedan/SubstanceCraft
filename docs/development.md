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
cargo run -p substancecraft              # the desktop app
cargo run -p substancecraft-ui --example ...   # offscreen renders (added in M1)
cargo run -p substancecraft-cli -- commands
cargo run -p substancecraft-cli -- run app.version
./Build/substancecraft --control 127.0.0.1:8765   # app with the JSON control server
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

## Packaging

```sh
cargo xtask build     # cargo build --release, then copy the apps into Build/
```

`Build/` is gitignored and holds only the finished applications. Signed per-platform packages arrive
with the release milestone (`docs/releasing.md`).

## Faster loops

- Check a single crate: `cargo check -p substancecraft-<crate>`.
- Run one test file: `cargo test -p <crate> --test <file>`.
- Parallel agents: use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`).

## Debugging

- `cargo xtask stats` prints crate, source and registered-command counts.
- The control channel (`docs/control-protocol.md`) lists and runs commands and later takes
  screenshots, so the UI can be driven and inspected without a human at the keyboard.
- `RUST_BACKTRACE=1` for a backtrace on an unexpected error (which should never be a panic).
