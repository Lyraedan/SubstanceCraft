# Contributing

Read `AGENTS.md` first. The rules below are the ones reviewers check.

## Clean-room

SubstanceCraft is a clean-room reimplementation of a Substance-3D-Painter-class tool. Study Painter
for behaviour and look only. Implement from public specifications (OpenPBR 1.1, glTF 2.0,
MikkTSpace, UDIM, standard PBR literature) and from observation. Never copy Adobe code, shaders,
profiles, icons or assets. All icons and brand art are original and live in `docs/brand/`. Every
third-party asset keeps its license file and gets an `ATTRIBUTION.md` row in the same change.

## Adding a command (the checklist)

1. Create or extend a focused module `crates/engine/src/<area>_cmds.rs` exporting a `specs()` that
   returns `Vec<CommandSpec>`.
2. Give the command a stable dotted id, a label, a menu path, an optional shortcut and a one-line
   **params** doc.
3. Implement `run(&mut Session, &Value) -> CommandResult`. Validate parameters, check bounds, reject
   absurd sizes; return `Err`, never panic. Add `enabled` when the command is state-dependent.
4. Register it in `crates/engine/src/commands.rs`.
5. Add tests: a happy path and a graceful-failure test (bad/missing/out-of-range params → `Err`).
6. Surface it in the UI by id; the control channel and CLI pick it up automatically.

## Layering

`cargo xtask layers` must stay green. A crate may depend only on same-or-lower layers. Nothing below
`ui`/`automation` may use egui/winit/native-dialog crates. Register a new crate in
`xtask/src/layers.rs` and `[workspace.dependencies]`.

## Tests are the gate

Every change ships tests. Format codecs use round-trip, synthetic-generator and fuzz tests. The GPU
compositor is validated against the CPU oracle. Commands get a graceful-failure test.

## Before you finish

```sh
cargo xtask ci        # layers + fmt --check + clippy -D warnings + tests
cargo xtask perf      # if you touched a hot path (compose, bake, format)
cargo xtask scorecard # if you changed the tree or moved a perf number
```

New behaviour is a command (a new `crates/engine/src/<area>_cmds.rs` module) so the UI, CLI, control
channel and MCP server all pick it up automatically. The existing command modules are `scene_cmds`,
`project_cmds`, `texture_set_cmds`, `layer_cmds`, `paint_cmds`, `bake_cmds`, `export_cmds`,
`viewport_cmds`, `info_cmds` and `ui_cmds`.

## Never crash

No `unwrap`/`expect`/`panic`/`unsafe` in non-test code (see `AGENTS.md`). Handle lock poisoning with
`PoisonError::into_inner`; bound recursion; validate input-derived indices; cap input-sized
allocations.

## Style

`cargo fmt --all` (max width 150). `cargo clippy --workspace --all-targets -- -D warnings`. Prefer
clear, documented public items. No comments that restate the code.

## Commits

Small, focused, building at every step. Describe what changed and why. Never commit secrets, corpora
or build output. Update the relevant docs and append a terse `log/devlog.md` entry.
