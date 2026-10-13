# UI dev log

Terse entries for the Painter-parity workspace work: what landed, how it was verified, what's open.
Newest last. The plan is in [`UI_roadmap.md`](UI_roadmap.md).

## 2026-10-13 — Phase 1: shell skeleton

Landed the Painter frame; the flat toolbar/left-sets/bottom-layers shell is gone.

- `ui::menu` — in-window menu bar built from the command registry by `menu_path` (File, Edit, View,
  Bake, Window, Help) + a shell-owned `Window` menu of panel toggles. `run_menu_command` opens the
  file dialogs (Open/Save As/Import/Export/Screenshot/Environment) and toggles grid/sky. Low-level
  (`layer.*`, `brush.*`) and introspection commands are hidden here but stay on the control channel.
- `ui::toolbar` — contextual properties bar: Size/Hardness/Spacing/Opacity (+ smudge strength, clone
  offset), colour, symmetry, and right-aligned display-mode/projection/grid/sky/frame.
- `ui::tool_rail` — fixed left icon rail (paint/orbit + every `ToolKind`).
- `ui::panels` — relocated Texture Set List, a Layers / Texture Set Settings tab strip, a Properties
  stub, the bottom Assets shelf (search + All/Generators/Filters/Bake/Post), and the status bar.
- `ui::viewport` — material dropdown overlay; new `ViewportAction::SelectTextureSet`.
- `ui::icons` — placeholder glyph registry. Codepoints were checked against the bundled
  `epaint_default_fonts` cmaps so nothing renders as tofu (naive symbols like 🖌/🧽 were dropped).
- `UiState` gained `show_tool_properties`, `show_tool_rail`, `right_tab` (old configs load via serde
  defaults).

Verified: `cargo xtask ci` green (18 crates, 88 commands); `Build/ui_phase1c.png` captured by driving
the running app over the control channel — menu + tool bar + rail + right dock + full-width shelf +
status bar all render.

Open: Phase 2 right-column depth, Phase 3 shelf grid, Phase 4 overlays, Phase 5 icons/theme.

## 2026-10-13 — Phase 2: right column depth

- **Texture Set List is a tree** (`panels::texture_set_list_panel`): each set row shows its name and,
  right-aligned, the shader label and resolution (`2048²`); the active set expands (via `ui.indent`)
  to `UV Tiles (N) · N tris` and one row per tile (`1001`, `No name`, resolution). Solo moved to the
  foot of the list. A `SetRow` view struct flattens the data; selection dispatches `texture_set.select`.
- **Texture Set Settings** (`panels::texture_set_settings_panel`) now owns identity: Name field
  (rename buffer keyed to the active set), resolution, channel toggles, UV-tile add/remove, shader and
  its parameters. The list panel is now purely the tree + solo.
- **Properties — Paint** (`panels::properties_panel`): a `Properties | Paint` header, then Material
  (composited base colour) and Mask preview thumbnails (mask = the selected layer's coverage swatch,
  "No mask" when absent), over the Paint Brush controls.

Verified: `cargo clippy -p substancecraft-ui --all-targets -D warnings` clean; captured
`Build/ui_phase2.png` (added two fill layers over the control channel) — the tree, the Layers tab and
the Properties previews all render.

Deferred: per-tile name/resolution (needs `doc::TextureSet` fields + commands) and collapsible
non-active sets. Next: Phase 3 (assets shelf grid).
