//! Placeholder glyphs for the workspace chrome.
//!
//! These stand in for the future original SVG icon catalog (see `docs/brand/`). A view asks for
//! an icon by a stable name and receives a single glyph; when the SVG catalog lands, only this
//! module changes and the call sites keep asking for the same names.

/// The glyph shown when an icon name is unknown.
pub const FALLBACK: &str = "•";

/// Resolve a stable icon name to a placeholder glyph.
///
/// Unknown names return [`FALLBACK`] rather than failing, so a missing icon never breaks layout.
pub fn glyph(name: &str) -> &'static str {
    match name {
        // Tools.
        "brush" => "🖊",
        "eraser" => "✖",
        "smudge" => "💧",
        "clone" => "🗗",
        "polyfill" => "▣",
        // Viewport tools.
        "paint" => "🖊",
        "orbit" => "🔄",
        // Layers.
        "fill" => "🎨",
        "paint_layer" => "🖊",
        "group" => "📁",
        "effect" => "✨",
        // Panels.
        "texture_sets" => "🔖",
        "layers" => "📚",
        "properties" => "⚙",
        "assets" => "🛠",
        "settings" => "⚙",
        // Assets shelf filters.
        "material" => "◎",
        "texture" => "🖼",
        "generator" => "✨",
        "filter" => "🔀",
        "tool" => "🛠",
        "font" => "🔤",
        // Row operations.
        "visibility" => "👁",
        "hidden" => "🚫",
        "search" => "🔍",
        "add" => "➕",
        "delete" => "🗑",
        "up" => "⬆",
        "down" => "⬇",
        "undo" => "↩",
        "redo" => "↪",
        _ => FALLBACK,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_names_resolve_to_a_real_glyph() {
        for name in ["brush", "eraser", "smudge", "clone", "polyfill", "orbit", "layers", "assets"] {
            let glyph = glyph(name);
            assert_ne!(glyph, FALLBACK, "{name} should have a dedicated glyph");
            assert!(!glyph.is_empty());
        }
    }

    #[test]
    fn unknown_names_fall_back_without_failing() {
        assert_eq!(glyph("no-such-icon"), FALLBACK);
    }
}
