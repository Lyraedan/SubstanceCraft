# Gaps

Every known shortfall, ranked. The work list to pick from. New entries go at the top of their tier.

_Last reviewed: 2026-10-13 (M12 complete)._

## Release (deferred)

1. **No packaging/signing.** `cargo xtask build` copies the raw binaries into `Build/`; a signed
   macOS `.app`, Windows installer and Linux tarball are not produced.
2. **A single perf baseline** is recorded on the dev machine; budgets are not enforced in CI yet
   (`enforce = false`).

## Automation polish (deferred from M11)

1. **No typed Rust scripting layer** over the command API (only the JSON/Script/MCP surfaces).
2. **MCP exposes tools only** — no resources or prompts.
3. **The control channel does not stream events** (only request/response).

## Project-format polish

6. **RON text is large** (paint images); no binary container. No templates/New Project window or
   startup recovery prompt; environment/high-poly/baked maps are not saved.

## Workflow-depth / export / baking / painting polish

7. **One shading model**; specular parameter unused; no anchors/instancing/physical size; cheap post-effects.
8. **Per-tile UDIM content**; no USD/OpenPBR material export, saved presets, batch export, edge padding.
9. **No cage mesh/skew**; per-map settings; baked maps not wired into generators; CPU baker.
10. **No smart materials/masks**; filter/generator params not editable in the UI.
11. **No native pen pressure/tilt; no physics brushes; CPU painting; not UDIM-aware.**
12. **GPU compositor limited to flat fill stacks; masks are constants; undo snapshots the whole set list.**
13. **AO/height display modes; grid alignment; camera persistence; prefiltered IBL; selection outline.**

## Documentation

14. Parity matrices beyond the tracked dimensions are still partial.
