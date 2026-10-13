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

## 2026-10-13 — Phase 3: assets shelf grid

- **Shelf header**: a library dropdown (`All Libraries` / `Project`), the search box, and an icon
  filter row (All / Materials / Brushes / Generators / Filters / Bake / Post) using `icons::glyph`.
  `Project` restricts generators/filters to the effect ids actually present in the active layer stack
  (`collect_effect_ids` walks groups recursively).
- **Thumbnail grid** (`Thumb` + `AssetTile` + `paint_thumb`): fixed 76px tiles with a caption under
  each, laid out with `ui.allocate_ui_with_layout` in a wrapped grid.
  - Generators render a real noise preview via `substancecraft-material::generate(64×64)`, cached in
    `StudioApp::generator_thumbs` (`ensure_generator_thumbs`, generated once).
  - Brushes are soft dabs in the current brush colour; filters a black→white gradient strip; the
    active material a base-colour swatch (clicking picks it into the brush); bake a glyph tile.
  - Clicking a tile dispatches the same command as before (`tool.set`, `layer.add_generator`,
    `layer.add_filter`, `brush.set_color`, `bake.run`).
- Bake settings and Post sliders remain below the grid when their tab (or `All`) is active.
- New dependency: `substancecraft-material` (L2) added to the UI crate (L6); `cargo xtask layers` green.

Verified: `cargo clippy -p substancecraft-ui --all-targets -D warnings` clean; captured
`Build/ui_phase3.png` — the material swatch, five brush dabs, four generated noise thumbnails, five
filter gradients and the bake tile all render, with the library dropdown and icon filter row.

Deferred: material *presets* (more than the active surface) and brush presets. Next: Phase 4
(viewport overlay/gizmo, cache usage in the status bar, panel show/hide commands).

## 2026-10-13 — Phase 4: overlays, cache readout, workspace commands

- **Workspace commands** (new engine module `workspace.rs` + `workspace_cmds.rs`): `ui.panel_state`,
  `ui.set_panel {panel, visible}`, `ui.toggle_panel {panel}`, `ui.set_right_tab {tab}`. The runtime
  source of truth is now `Session::workspace()` (`engine::Workspace`: five panel flags + `RightTab`);
  the shell reads it each frame and mirrors it to `ui::state::UiState` for persistence (seed on start,
  sync in `logic()`). `RightTab` moves to the engine and is re-exported by `ui::state`. The `Window`
  menu now dispatches `ui.set_panel`; the Layers/Settings tab dispatches `ui.set_right_tab` — so the
  menu and the control channel do exactly the same thing.
- **Viewport overlays** (`ui::viewport`): a translucent shortcut-hint block bottom-left
  (paint vs orbit variants), an axis gizmo bottom-right (world X/Y/Z rotated by the camera view),
  and the resolution/tool readout moved to the top-left. The material dropdown stays top-right.
- **Status bar** (`panels::status_bar`): added a `Cache: N.N MB (P%)` readout from
  `Session::cache_bytes()` (paint images + baked maps, recursing groups; % of a 4 GiB budget).

Verified over the control channel: `ui.panel_state` returns the flags; `ui.toggle_panel {assets}`
hid the shelf (captured in `Build/ui_phase4_hidden.png`, and the change persisted across a restart);
`ui.set_right_tab {settings}` switched the dock (seen in the same shot). `Build/ui_phase4_full.png`
shows the full layout with hints, gizmo and the cache readout. `cargo xtask ci` green; engine 64 +
ui 11 tests pass.

Next: Phase 5 (original SVG icon catalog + optional neutral-gray theme).

## 2026-10-13 — Phase 5: original SVG icons + gray theme

- **Icon catalog**: authored 21 original monochrome SVGs in `docs/brand/icons/` (brush, eraser,
  smudge, clone, polyfill, orbit, add, delete, up, down, group, undo, redo, visibility, hidden,
  search, layers, material, generator, filter, assets), aliased to 23 names (`paint`→brush,
  `effect`→generator). `ui::icons` embeds them with `include_bytes!`, prepares them through
  `craft_ui::icons::SvgIconSet`, and exposes `icons::image(name, size)` + `install_loaders(ctx)`
  (egui_extras SVG loader, installed in `StudioApp::new`). The placeholder glyph registry is gone;
  the tool rail, layer toolbar, asset filter row, search field and bake tile now draw the SVGs via
  `Button::image` / `Button::image_and_text` / `Image::paint_at`. New deps: `egui_extras` (svg).
- **Gray theme**: added `Tokens::GRAY` (neutral chrome, amber accent) and `ThemeChoice::{Dark, Gray}`
  with `tokens()`; the theme is persisted and switchable from `Window > Theme`
  (`StudioApp::set_theme`), applied on startup from prefs.

Verified: `cargo clippy -p substancecraft-ui --all-targets -D warnings` clean; 11 ui tests pass
(`every_referenced_icon_exists`). Captured `Build/ui_phase5_dark.png` (icons across the rail, layers
toolbar, filter row, bake tile) and `Build/ui_phase5_gray.png` (gray + amber, switched via the
persisted `config.toml`).

Phase 5 completes the workspace redesign (`UI_roadmap.md`). Open follow-ups live on the individual
phase entries (per-tile UDIM naming, collapsible sets, material/brush presets).
