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
| Bottom: Assets shelf | `panels::assets_panel` | ◐ |
| Viewport overlays | `ui::viewport` (material dropdown; hints/gizmo pending) | ◐ |
| Status bar | `panels::status_bar` (version; cache usage pending) | ◐ |
| Icons | `ui::icons` placeholder glyphs → original SVG catalog | ◐ |

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

### Phase 3 — Assets shelf ☐
- [ ] Library dropdown (project/substance/…).
- [ ] Icon filter row and a thumbnail grid (material spheres, generators, filters, brushes).
- [ ] Search spans all asset kinds.

### Phase 4 — Overlays & chrome ☐
- [ ] Viewport shortcut-hint overlay (bottom-left) and axis gizmo.
- [ ] Cache-usage readout in the status bar.
- [ ] Panel show/hide commands so the control channel can drive the workspace.

### Phase 5 — Icons & theme ☐
- [ ] Original monochrome SVG catalog in `docs/brand/`, wired through `craft_ui::icons::SvgIconSet`.
- [ ] Optional neutral studio-gray token set.
- [ ] `ATTRIBUTION.md` rows for any third-party assets.

## Guardrails
- Never crash: the crate denies `unwrap/expect/panic/unsafe`; menu/tab code tolerates empty input.
- One colour source: views read `theme::Tokens`; no literals.
- Everything is a command: new behaviour becomes a command, not shell-only logic (except pure view
  state such as panel visibility and dock selection, which stays in `ui::state`).
