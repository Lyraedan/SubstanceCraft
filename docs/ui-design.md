# UI design

The shell is `crates/ui` (egui/eframe), built on `craft-ui` (shared Craft controls) and driven
entirely by commands. Views read design tokens; they never hard-code colours.

## Tokens

`substancecraft_ui::theme::Tokens` is the single source of truth: window/panel/surface colours, an
accent, text colours, borders, the viewport letterbox colour and a corner radius. `Tokens::DARK` is
the default studio theme. `Tokens::apply(ctx)` installs it: it sets egui visuals, maps the widget
palette through `craft_ui::theme::WidgetPalette` and sets spacing.

To add a theme: add a `Tokens` constant and a `ThemeChoice` variant in `state.rs`; the toolbar
switches between them. No colour literal belongs in a view.

## State

All view state lives in `substancecraft_ui::state::UiState` (serde) and is persisted through
`craft-settings` (`config.toml`, atomic writes, migrations, recovery). Panels (Texture Sets, Shelf,
Layer Stack) toggle from the toolbar and are remembered. Document state never lives here.

## Layout

The shell mirrors Substance 3D Painter's frame. `StudioApp::ui` composes egui 0.36 `Panel`s in this
order (bottom/left/right panels are added after the top bars so the assets shelf spans the full
width, as in Painter):

1. **Menu bar** (`top`, `menu.rs`) — built from the command registry grouped by `menu_path`
   (`File`, `Edit`, `View`, `Bake`, `Window`, `Help`) plus a shell-owned `Window` menu of panel
   toggles. Commands needing a parameter or file dialog are handled by `StudioApp::run_menu_command`;
   low-level and introspection commands are hidden here (they stay reachable via panels/automation).
   Deviation from Painter: there is no `Mode`/`JavaScript`/`Python` menu.
2. **Tool properties bar** (`top`, `toolbar.rs`) — the active tool's Size/Hardness/Spacing/Opacity
   (plus tool-specific smudge strength and clone offset), colour and symmetry, with quick viewport
   controls (display mode, projection, grid, sky, frame) right-aligned.
3. **Status bar** (`bottom`) — latest status message and the product version.
4. **Assets shelf** (`bottom`, `panels::assets_panel`) — a search box, filter tabs (All/Generators/
   Filters/Bake/Post) and the generators, filters, bake panel and post-effects.
5. **Tool rail** (`left`, `tool_rail.rs`) — a fixed icon column for paint/orbit and every paint tool.
6. **Right dock** (`right`, `panels.rs`) — the Texture Set List (top), a `Layers` /
   `Texture Set Settings` tab strip (`state::RightTab`) and a `Properties` section (composited
   preview plus the active brush).
7. **Viewport** (`CentralPanel`) — the 3D renderer with a material dropdown overlay (top-right).

Panels are resizable; visibility is persisted in `UiState` and toggled from the `Window` menu.

## Icons

`icons.rs` maps a stable icon name to a placeholder glyph. The glyphs are chosen from codepoints
that the bundled egui fonts actually cover (verified against `epaint_default_fonts`), so nothing
renders as tofu. They stand in for the original SVG catalog (see `docs/brand/`); when the SVGs land,
only `icons.rs` changes. No Adobe icons or assets are used.

## Performance

The viewport re-composites and re-renders only when a signature of scene/camera/settings/size changes,
so idle frames are cheap.

## Verifying UI changes

Render offscreen (no window, no focus stealing) or launch with `--control` and screenshot; look at
the PNG. Demo assets must be public-domain/CC0.
