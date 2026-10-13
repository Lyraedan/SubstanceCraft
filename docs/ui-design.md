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
6. **Dockable workspace** (`CentralPanel`, `craft_ui::docking`) — the remaining area is a `DockArea`
   of [`dock::DockPanel`]s: the **Viewport** (a protected, tab-less panel), the **Texture Set List**,
   the **Layers** / **Texture Set Settings** tab group, **Properties**, and the bottom **Assets**
   shelf. Panels can be dragged, tabbed together, split, floated, resized and closed; the viewport
   stays put. The arrangement ([`Layout<DockPanel>`](dock.rs)) is persisted in `UiState` and the
   closed-panel locations are remembered so a reopen restores its place.
   - The default arrangement mirrors Painter ([`dock::default_layout`]).
   - `StudioApp` applies the renderer's docking `Action`s and keeps the session's workspace commands
     in sync two-way, so `ui.set_panel` / `ui.toggle_panel` (control channel) close and reopen
     dock panels exactly like the `Window` menu.

Panels are resizable; visibility is persisted in `UiState` and toggled from the `Window` menu.

## Icons

The workspace icons are original monochrome SVGs in `docs/brand/icons/` (see that directory's
README for the format). `crates/ui/src/icons.rs` embeds them with `include_bytes!` and prepares them
through `craft_ui::icons::SvgIconSet`; `icons::install_loaders(ctx)` installs egui's SVG loader at
startup. Views ask for `icons::image("name", size)` and use `egui::Button::image` /
`Button::image_and_text`; nothing in a view names an asset file directly. Add an icon by dropping the
SVG in `docs/brand/icons/` and adding a `CATALOG` entry (a test guards the mapping). No Adobe icons
or assets are used.

## Themes

`theme::Tokens` provides `DARK` (default, blue accent) and `GRAY` (neutral studio gray, amber
accent). `state::ThemeChoice` selects one; it is persisted and switchable from the `Window` menu.
The chosen palette is applied on startup and on change; views never hard-code colours.

## Performance

The viewport re-composites and re-renders only when a signature of scene/camera/settings/size changes,
so idle frames are cheap.

## Verifying UI changes

Render offscreen (no window, no focus stealing) or launch with `--control` and screenshot; look at
the PNG. Demo assets must be public-domain/CC0.
