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

## 2026-10-13 — Phase 6: dockable, tabbed panels

Replaced the fixed `egui::Panel` right/bottom layout with `craft_ui::docking` so panels behave like
Painter's dockers.

- New `ui::dock`: `DockPanel` (`Viewport`, `TextureSetList`, `Layers`, `Settings`, `Properties`,
  `Assets`) with ids/labels and permissions (Viewport = `PROTECTED`, tab-less; the rest fully
  dockable), plus `default_layout()` reproducing Painter (viewport left, Texture Set List over the
  Layers/Settings tabs over Properties on the right, Assets along the bottom).
- `StudioApp::ui` now renders one `DockArea` in the central area (menu bar, tool properties bar, tool
  rail and status bar stay fixed chrome). Panels are draggable, tabbable, splittable, floatable,
  resizable and closable; `apply_dock_action` applies the renderer's `Action`s transactionally and
  remembers closed-panel `Location`s so a reopen restores its place.
- Layout + closed locations are persisted in `UiState` (`craft-ui` `serde` feature enabled). Two-way
  sync with the session workspace: `apply_workspace_to_dock` reconciles `ui.set_panel` / `ui.
  toggle_panel` (control channel) into the dock, and `sync_workspace_from_dock` mirrors user
  open/close gestures back into the workspace flags. Removed the now-unused `right_section` and the
  panels' own title headers (the tabs label them).

Verified: `cargo xtask ci` green (18 crates, 92 commands); `Build/ui_dock_full.png` shows the tabbed
right column (Texture Set List / Layers|Texture Set Settings / Properties) and the Assets bottom tab
around the tab-less viewport. Over the control channel, `ui.set_panel {assets:false}` removed the
Assets tab (`Build/ui_dock_noassets.png`) and `{assets:true}` restored it — proving the dock/workspace
sync. Dragging/grouping/floating is provided and tested by `craft_ui` upstream.

## 2026-10-13 — Bake and Post split into their own dock tabs

- `DockPanel` gained `Bake` and `Post`; the default bottom group is now `Assets | Bake | Post` tabs.
- `panels::assets_panel` lost its Bake/Post sections and the "Bake Maps" tile; the asset filters are
  now All/Materials/Brushes/Generators/Filters. New `panels::bake_panel` (resolution, AO samples,
  high-poly import/clear, Bake Mesh Maps, preview, per-map save, log) and `panels::post_panel`
  (vignette, bloom) are their own render bodies.
- `StudioApp::ensure_all_dock_panels` docks any panel missing from a restored layout (unless the user
  closed it), so existing saved layouts pick up the new tabs. Bake/Post prefer the Assets group.
- Build output lives in `Build/` (== `build/` on this case-insensitive filesystem).

Verified: `cargo xtask ci` green (18 crates, 92 commands); `Build/ui_bakepost.png` shows the
`Assets | Bake | Post` tab strip; release rebuilt into `build/`.

## 2026-10-14 — Effect parameter editor in the Layers panel

- The selected layer's controls now show an effect layer's parameters below the mask: float sliders
  and integer drag controls (octaves/seed/radius), driven by `doc::Param` ranges (`LayerView` gained
  `effect: Option<Effect>`). Edits dispatch `layer.set_effect`, so the UI and the control channel do
  the same thing. Invert (no parameters) shows only its id.
- The Layers panel switched from a two-column list|properties split to a Painter-style vertical
  layout (list on top, selected-layer properties below) inside one `ScrollArea`, so effect parameters
  have the full panel width and overflow scrolls instead of clipping. The default Layers height rose
  to 300.

Verified: `cargo clippy -p substancecraft-ui --all-targets -D warnings` clean; 13 ui tests pass; a
Perlin layer's scale/octaves/seed round-trip through `layer.set_effect` / `layer.list`; screenshot
`Build/ui_effect_params.png` shows the layer row, Blend/Opacity/Mask and the `perlin parameters`
section with the Scale slider.

## 2026-10-14 — App/window icon

- `assets/icon.png` is decoded at startup (`substancecraft-codecs::decode_rgba8_bytes`, new) and set
  as the eframe window icon (`with_icon`). A decode failure warns and falls back to the default, never
  panics. No new asset licence (original icon).
