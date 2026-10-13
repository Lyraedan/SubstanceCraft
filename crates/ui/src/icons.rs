//! The original SubstanceCraft icon catalog.
//!
//! Icons are monochrome SVGs authored for SubstanceCraft in `docs/brand/icons/` (24×24,
//! `currentColor` strokes, 2px). Nothing is copied from Adobe. `craft_ui`'s `SvgIconSet`
//! substitutes white so egui can load and tint them; install the image loaders once via
//! [`install_loaders`].

use std::sync::OnceLock;

use craft_ui::icons::{SvgIconSet, SvgStyle};
use egui::{Image, vec2};

/// The embedded catalog as `(name, svg bytes)`. Several names alias one drawing.
static CATALOG: &[(&str, &[u8])] = &[
    ("brush", include_bytes!("../../../docs/brand/icons/brush.svg")),
    ("paint", include_bytes!("../../../docs/brand/icons/brush.svg")),
    ("eraser", include_bytes!("../../../docs/brand/icons/eraser.svg")),
    ("smudge", include_bytes!("../../../docs/brand/icons/smudge.svg")),
    ("clone", include_bytes!("../../../docs/brand/icons/clone.svg")),
    ("polyfill", include_bytes!("../../../docs/brand/icons/polyfill.svg")),
    ("orbit", include_bytes!("../../../docs/brand/icons/orbit.svg")),
    ("add", include_bytes!("../../../docs/brand/icons/add.svg")),
    ("delete", include_bytes!("../../../docs/brand/icons/delete.svg")),
    ("up", include_bytes!("../../../docs/brand/icons/up.svg")),
    ("down", include_bytes!("../../../docs/brand/icons/down.svg")),
    ("group", include_bytes!("../../../docs/brand/icons/group.svg")),
    ("undo", include_bytes!("../../../docs/brand/icons/undo.svg")),
    ("redo", include_bytes!("../../../docs/brand/icons/redo.svg")),
    ("visibility", include_bytes!("../../../docs/brand/icons/visibility.svg")),
    ("hidden", include_bytes!("../../../docs/brand/icons/hidden.svg")),
    ("search", include_bytes!("../../../docs/brand/icons/search.svg")),
    ("layers", include_bytes!("../../../docs/brand/icons/layers.svg")),
    ("material", include_bytes!("../../../docs/brand/icons/material.svg")),
    ("generator", include_bytes!("../../../docs/brand/icons/generator.svg")),
    ("effect", include_bytes!("../../../docs/brand/icons/generator.svg")),
    ("filter", include_bytes!("../../../docs/brand/icons/filter.svg")),
    ("assets", include_bytes!("../../../docs/brand/icons/assets.svg")),
];

static SET: OnceLock<SvgIconSet> = OnceLock::new();

/// The shared icon catalog, prepared once.
fn set() -> &'static SvgIconSet {
    SET.get_or_init(|| SvgIconSet::new(CATALOG, SvgStyle { current_color: "white", stroke_width: None }, "bytes://substancecraft/icons/", "brush"))
}

/// Whether a name exists in the catalog.
pub fn contains(name: &str) -> bool {
    set().contains(name)
}

/// An icon image at `size` points (square). Tint it with [`Image::tint`] if needed.
pub fn image(name: &str, size: f32) -> Image<'static> {
    Image::new(set().source(name)).fit_to_exact_size(vec2(size, size))
}

/// Install egui's image loaders (including the SVG decoder) on the context.
pub fn install_loaders(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_referenced_icon_exists() {
        for name in [
            "brush",
            "paint",
            "eraser",
            "smudge",
            "clone",
            "polyfill",
            "orbit",
            "add",
            "delete",
            "up",
            "down",
            "group",
            "undo",
            "redo",
            "visibility",
            "hidden",
            "search",
            "layers",
            "material",
            "generator",
            "effect",
            "filter",
            "assets",
        ] {
            assert!(contains(name), "{name} should be in the catalog");
        }
    }

    #[test]
    fn unknown_icons_are_reported_missing() {
        assert!(!contains("no-such-icon"));
    }
}
