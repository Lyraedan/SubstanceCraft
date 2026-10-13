# Brand assets

Original SubstanceCraft icons, logos and theme art live here. Nothing in this directory is copied
from Adobe or any other proprietary source; each asset is authored for SubstanceCraft (or is a
permissively licensed third-party asset recorded in `../ATTRIBUTION.md`).

## Icons

`icons/` holds the workspace icon catalog: original monochrome SVGs, 24×24 viewBox, `fill="none"`,
`stroke="currentColor"`, `stroke-width="2"`, round caps/joins. `crates/ui/src/icons.rs` embeds them
with `include_bytes!` and prepares them through `craft_ui::icons::SvgIconSet` (which substitutes the
`currentColor`), then egui's SVG loader (`egui_extras`) rasterizes and tints them.

To add an icon: add the SVG here, an entry to `CATALOG` in `crates/ui/src/icons.rs`, and use
`icons::image("name", size)` (or `Button::image`) at the call site. The `every_referenced_icon_exists`
test guards the mapping.

Note: `NotoEmoji`-style symbols were replaced by this catalog in Phase 5; the placeholder glyph
registry is gone.
