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

The app shell composes `egui::Panel::{top,bottom,left,right}` and a `CentralPanel` for the viewport,
using the new egui 0.36 `Panel` API. Panels are resizable. The central area is the 3D viewport.

## Icons

Icons are original SVGs in `docs/brand/`, loaded through `craft_ui::icons::SvgIconSet` with egui's
SVG loader. No Adobe icons or assets are used.

## Verifying UI changes

Render offscreen (no window, no focus stealing) or launch with `--control` and screenshot; look at
the PNG. Demo assets must be public-domain/CC0.
