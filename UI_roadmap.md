# UI roadmap

Goal: make SubstanceCraft's workspace look and behave like Substance 3D Painter's — **layout
parity, original theme and icons** (clean-room: study Painter for behaviour/look only, never copy
assets). The shell is `crates/ui`; it stays thin and dispatches commands by id.

Status legend: ✅ done · ◐ in progress · ☐ not started.

## Target layout (Painter → SubstanceCraft)

```text
┌ menu bar (File Edit View Bake Window Help) ───────────────────────────────┐
├ tool properties bar (Size Flow Stroke-opacity Spacing, view controls) ─────┤
│ ┌rail┐ ┌──────────── viewport ────────────┐ ┌ right dock ──────────────┐ │
│ │    │ │            (material ▾)           │ │ Texture Set List (tree)  │ │
│ │    │ │                                  │ │ Layers | Texture Set Set.│ │
│ │    │ │                                  │ │ Properties — Paint       │ │
│ └────┘ └──────────────────────────────────┘ └──────────────────────────┘ │
├ assets shelf (search · filters · thumbnails) ─────────────────────────────┤
└ status bar (message ······································ version) ──────┘
```

| Painter region | SubstanceCraft | Status |
|---|---|---|
| Menu bar | `ui::menu` from the command registry + shell `Window` toggles | ✅ |
| Tool properties bar | `ui::toolbar` (brush params + view controls) | ✅ |
| Left tool rail | `ui::tool_rail` | ✅ |
| Right: Texture Set List | `panels::texture_set_list_panel` (tree) | ✅ |
| Right: Layers / Texture Set Settings tabs | `panels::right_section` | ✅ |
| Right: Properties — Paint | `panels::properties_panel` | ✅ |
| Bottom: Assets shelf | `panels::assets_panel` | ✅ |
| Viewport overlays | `ui::viewport` (material dropdown, shortcut hints, axis gizmo) | ✅ |
| Status bar | `panels::status_bar` (message, cache usage, version) | ✅ |
| Icons | `ui::icons` original SVG catalog (`docs/brand/icons`) | ✅ |

Progress: all five phases done. See [`UI_devlog.md`](UI_devlog.md).

## Phases

### Phase 1 — Shell skeleton ✅
- [x] In-window menu bar from the registry; shell `Window` toggles; dialog-aware command runner.
- [x] Contextual tool properties bar + right-aligned viewport controls.
- [x] Left icon tool rail (paint/orbit + every tool).
- [x] Right dock skeleton (list, Layers/Settings tabs, Properties stub).
- [x] Full-width bottom assets shelf (search + filter tabs).
- [x] Status bar with version.
- [x] Material dropdown viewport overlay.
- [x] `UiState` extended (`show_tool_properties`, `show_tool_rail`, `right_tab`), persisted.
- [x] Verified via control-channel screenshot.

### Phase 2 — Right column depth ✅ (core; two follow-ups deferred)
- [x] Texture Set List as a tree: set row (name · resolution · shader) with UV tiles nested under the
      active set (`udim · name · resolution`, "No name" when unnamed).
- [x] Texture Set Settings tab holds Name/Resolution/UV Tiles/Channels/Shader (list keeps the tree + solo).
- [x] Properties — Paint: material (composited) + mask preview thumbnails over the brush controls.
- [ ] Per-tile name/resolution editing (needs `doc` fields + commands).
- [ ] Collapsible sets (expand any set, not just the active one).

### Phase 3 — Assets shelf ✅
- [x] Library dropdown (`All Libraries` / `Project`; Project filters generators/filters to those used
      in the active layer stack).
- [x] Icon filter row (All/Materials/Brushes/Generators/Filters/Bake/Post) and a thumbnail grid.
- [x] Thumbnails: real procedural generator previews (`substancecraft-material`, cached), brush dabs,
      filter gradients, the active material swatch and a bake glyph.
- [x] Search spans all asset kinds; click-to-add dispatches the same commands as before.
- [ ] Material *presets* (material spheres beyond the active surface) and brush presets.

### Phase 4 — Overlays & chrome ✅
- [x] Viewport shortcut-hint overlay (bottom-left) and axis gizmo (bottom-right).
- [x] Cache-usage readout in the status bar (`Session::cache_bytes`, % of a 4 GiB budget).
- [x] Panel show/hide + dock-tab commands: `ui.panel_state`, `ui.set_panel`, `ui.toggle_panel`,
      `ui.set_right_tab`. Workspace state lives on the `Session` so the control channel/MCP drive it;
      the shell persists it to prefs and the `Window` menu dispatches the same commands.

### Phase 5 — Icons & theme ✅
- [x] Original monochrome SVG catalog in `docs/brand/icons/` (21 drawings, aliased to 23 names),
      wired through `craft_ui::icons::SvgIconSet` + `egui_extras`' SVG loader.
- [x] Neutral studio-gray token set (`Tokens::GRAY`, amber accent) selectable from `Window > Theme`.
- [x] No third-party assets: the icons are original, so no `ATTRIBUTION.md` rows are needed.

## Guardrails
- Never crash: the crate denies `unwrap/expect/panic/unsafe`; menu/tab code tolerates empty input.
- One colour source: views read `theme::Tokens`; no literals.
- Everything is a command: new behaviour becomes a command, not shell-only logic (except pure view
  state such as panel visibility and dock selection, which stays in `ui::state`).
