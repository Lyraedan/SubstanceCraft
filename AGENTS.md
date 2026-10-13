# AGENTS.md: guide for AI agents and contributors

SubstanceCraft is an open-source, native, Substance-3D-Painter-comparable 3D texture painting
application written in **Rust only**. **No Tauri, Electron or webview shells:** the desktop app is
native eframe/egui on wgpu. The product name is always written **SubstanceCraft** (`{Function}Craft`
in PascalCase, like its siblings). Machine names stay lowercase: crates (`substancecraft-*`),
binaries, file names, ids (`ai.storyteller.substancecraft`).

It is a **clean-room** reimplementation: we study Substance 3D Painter for *behaviour and look only*
and implement from public specifications (OpenPBR, glTF 2.0, MikkTSpace, UDIM) and observation.
Never copy Adobe code, shaders, profiles, icons or other assets.

Shared standards live in [craftrules](https://github.com/storytold/craftrules) (`../craftrules` on a
dev machine); shared Rust components live in [storytold/craft-libs](https://github.com/storytold/craft-libs)
and are pinned by immutable git revision in the root `Cargo.toml`.

## 1. Orientation (5 minutes)

| Read | Why |
|---|---|
| `docs/architecture.md` | Crate map, dependency layers, the engine/UI seam, document model |
| `docs/development.md` | Build, test, run, package into `Build/`, debug tricks |
| `docs/contributing.md` | Rules: clean-room, tests, layering, style, commits |
| `docs/control-protocol.md` | JSON control channel: how agents drive and inspect the running app |
| `docs/ui-design.md` | Design tokens, themes, widgets |
| `ROADMAP.md`, `docs/roadmap.md` | Milestones M0–M12 and the current focus |
| `docs/target-app-parity.md`, `docs/gaps.md` | Where we stand against Painter, ranked work list |
| `docs/*-parity.md`, `docs/scorecard.md` | Per-dimension checklists and measured numbers |

## 2. Workspace map

```text
crates/
  geom color raster codecs     L0 foundation (meshes + tangents, colour, pixel formats + blend math, codecs)
  tablet                        L0 isolated pen-tablet input (the only crate that may use `unsafe`)
  doc format                    L1 document model (pure data) + native .sccraft
  algo paint material bake       L2 imaging algorithms, brush engine, procedural content, bakers
  compose gpu render             L3 CPU oracle, wgpu compositor, viewport PBR renderer
  io                            L4 mesh/texture import-export, export presets
  engine                        L5 Session + command registry (every action is a command)
  ui automation                 L6 egui shell (thin) + JSON control server / MCP
apps/
  substancecraft                desktop app (eframe/wgpu) + TCP control server
  substancecraft-cli            headless CLI (commands/run/script/serve/mcp/info)
Build/                          final built apps only (gitignored; produced by `cargo xtask build`)
xtask/                          cargo xtask layers | stats | build | ci | assets | perf | scorecard
```

**Layering is enforced** by `cargo xtask layers`. A crate may depend only on crates at the same or a
lower layer; nothing below `ui`/`automation` may use egui/winit/rfd. A new crate must be registered
in `xtask/src/layers.rs` **and** `[workspace.dependencies]`.

## 3. Golden rules

### Never crash (outranks feature work)

A malformed file, a bad command param, a corrupt settings file, an odd keystroke or a full disk must
produce an actionable error, never a panic. The shared standard is `../craftrules/standards/never-crash.md`.

- **Non-test code never panics.** No `unwrap()`, `expect()`, `panic!`, `unreachable!`, `todo!` or
  `unimplemented!`. Return the crate's error type and propagate with `?`; use `ok_or(..)?`,
  `let .. else { return Err(..) }`, or `unwrap_or*` where a fallback is truly correct. The only
  exception is a provably infallible literal (`#[allow(clippy::expect_used)]` + `.expect("why")`).
- Every crate's `lib.rs` starts with
  `#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]`.
  `clippy.toml` allows these in tests only.
- **No `unsafe`.** The workspace sets `unsafe_code = "forbid"`. The one exception is
  `substancecraft-tablet` (winit drops pen pressure/tilt), which does not inherit the workspace lints,
  keeps `unsafe_code = "deny"` crate-wide and allows each block with a `SAFETY:` comment.
- **Input-derived numbers are hostile.** Use `get()` not `[i]`/`[a..b]` for file/param-derived
  indices; slice at char boundaries; use `checked_*`/`saturating_*`; guard division by zero and
  NaN/inf; cap allocations sized by input (see the control server's 1 MiB request limit).
- **Bound recursion** with depth limits or seen-sets (documents can be deep or cyclic).
- **Don't cascade.** Handle lock poisoning with `PoisonError::into_inner`; treat thread joins as `Result`s.
- **Last-resort guard.** The app shell catches escaped panics around command dispatch and file
  import/export, reports an error and keeps the document. Keep `panic = "unwind"`.
- **Prove it.** Every crash fix comes with a small synthetic regression test that panicked before.

### 1. Everything is a command

New user-visible behaviour = a command in `crates/engine/src/*_cmds.rs` (registered in
`commands.rs`) with id, label, menu path, shortcut, params doc, `enabled` and `run`, plus tests. The
UI, CLI, control channel and MCP all dispatch by id. Only pure view state (zoom, panel visibility)
belongs to the shell. Commands must never panic: validate params, check bounds, reject absurd sizes,
return `Err`.

### 2. No format or colour assumptions

Bit depth (8/16/32f) and colour model are runtime data; never introduce a `u8`-only pixel path in a
public API. Never assume sRGB: transfers and tone mapping go through `substancecraft-color`. Test at
several depths.

### 3. Clean-room

Study Painter for behaviour/look only. Implement from public specs (OpenPBR 1.1, glTF 2.0,
MikkTSpace, UDIM, the metal/rough + spec/gloss PBR papers). Never copy code, shaders, profiles or
assets. Third-party and original assets keep their license next to them and get an `ATTRIBUTION.md`
row in the same change. Original icons live in `docs/brand/`.

### 4. Tests are the gate

Every change comes with tests. Format codecs use round-trip, synthetic-generator and fuzz tests. The
GPU compositor is validated against the CPU oracle. Keep `cargo xtask layers` green.

### 5. The UI is thin and data-driven

UI state lives in `crates/ui/src/state.rs` (serde) so the control channel can read and drive it.
Colours and radii come from `theme::Tokens`, never hard-coded.

### 6. Verify UI changes visually

Render offscreen (no window, no focus stealing) or launch with `--control` and screenshot. Look at
the PNG. Demo assets must be public-domain / CC0, never personal work.

### 7. Performance is a feature

Benchmark heavy operations in release. Work per tile in parallel (rayon), skip empty tiles, never
scan a full surface per frame (cache by `Session::revision`), and record before/after timings.

## 4. Before you finish a task

```sh
cargo test -p <crate you touched>
cargo clippy -p <crate> --all-targets -- -D warnings
cargo xtask layers
cargo fmt --all -- --check
```

Or the whole gate at once: `cargo xtask ci`. To produce the runnable app in `Build/`: `cargo xtask build`.
If you touched a hot path (compose, bake, format) run `cargo xtask perf`; if you changed the tree or
moved a number run `cargo xtask scorecard` (it regenerates `docs/scorecard.md`).

Then append a terse entry to `log/devlog.md` (what landed, numbers, what's still open). Keep the tree
building at every step.

## 5. Parallel agents

- Use your own target dir (`CARGO_TARGET_DIR=target/agent-<name>`) to avoid the Cargo build lock, and
  edit only the files you own. Shared files (`engine/src/lib.rs`, the registration list in
  `engine/src/commands.rs`, `ui/src/app.rs`) get small, surgical edits; re-read before editing.
- Put new commands in a **new module** (`engine/src/<area>_cmds.rs`), not a shared file.
- Keep every `Cargo.toml` valid at all times; create or rewrite manifests atomically (write a temp
  file outside `crates/`, then `mv` it into place).

## 6. Where things are tracked

- `ROADMAP.md` / `docs/roadmap.md`: milestones and current focus.
- `docs/target-app-parity.md`, `docs/gaps.md`, `docs/*-parity.md`: Painter parity assessment.
- `docs/scorecard.md`: generated by `cargo xtask scorecard` (workspace metrics + perf baseline).
- `perf/budgets.toml`, `perf/baseline.json`: performance budgets and the recorded baseline.
- `plan/` and `log/`: local, gitignored working notes.
