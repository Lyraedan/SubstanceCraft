//! Fonts embedded from the optional `craft-fonts` build input.
//!
//! The Crafting Apps share their non-Latin fonts through
//! [`storytold/craft-fonts`](https://github.com/storytold/craft-fonts). It is a **fully optional**
//! build input: when `CRAFT_FONTS_DIR` names a craft-fonts checkout, the crate's `build.rs` embeds
//! every font in `fonts/manifest.txt` and [`install`] registers them as egui fallbacks; with the variable
//! unset `CRAFT_FONTS` is empty and every function here is a no-op, so nothing depends on
//! craft-fonts being present. See craft-fonts' `docs/integration.md`.

/// One font from the optional craft-fonts build input (empty unless built with `CRAFT_FONTS_DIR`).
pub struct CraftFont {
    /// Family name, as the font's own `name` table spells it (`BIZ UDPGothic`, …).
    pub family: &'static str,
    /// Style within the family (`Regular`, `Bold`, …).
    pub style: &'static str,
    /// ISO 15924 script codes the font is for, e.g. `"Jpan"`.
    pub scripts: &'static [&'static str],
    /// The font file's bytes.
    pub bytes: &'static [u8],
}

include!(concat!(env!("OUT_DIR"), "/craft_fonts.rs"));

/// Whether this build embedded any craft-fonts.
pub fn has_fonts() -> bool {
    !CRAFT_FONTS.is_empty()
}

/// The embedded fonts for an ISO 15924 script, in manifest order.
pub fn for_script(script: &str) -> impl Iterator<Item = &'static CraftFont> {
    CRAFT_FONTS.iter().filter(move |font| font.scripts.contains(&script))
}

/// Register the embedded Japanese fonts as egui fallbacks, after the app's own fonts. A no-op when
/// the app was built without craft-fonts.
pub fn install(ctx: &egui::Context) {
    let fonts: Vec<&CraftFont> = for_script("Jpan").collect();
    if fonts.is_empty() {
        return;
    }
    let mut definitions = egui::FontDefinitions::default();
    for font in fonts {
        let key = format!("craft-fonts:{}-{}", font.family, font.style);
        definitions.font_data.insert(key.clone(), std::sync::Arc::new(egui::FontData::from_static(font.bytes)));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            definitions.families.entry(family).or_default().push(key.clone());
        }
    }
    ctx.set_fonts(definitions);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every embedded file looks like a font. Without `CRAFT_FONTS_DIR` the table is empty and this
    /// asserts nothing (as craft-fonts' integration guide requires tests to skip cleanly).
    #[test]
    fn embedded_fonts_are_valid_when_present() {
        for font in CRAFT_FONTS {
            assert!(!font.bytes.is_empty(), "{}-{} is empty", font.family, font.style);
            let known = font.bytes.starts_with(&[0x00, 0x01, 0x00, 0x00])
                || font.bytes.starts_with(b"true")
                || font.bytes.starts_with(b"OTTO")
                || font.bytes.starts_with(b"ttcf");
            assert!(known, "{}-{} is not a recognised font", font.family, font.style);
        }
    }

    /// Installing and rendering a frame must never panic, with or without the fonts embedded.
    #[test]
    fn a_frame_renders_with_the_embedded_fonts() {
        let ctx = egui::Context::default();
        install(&ctx);
        let output = ctx.run_ui(egui::RawInput::default(), |ui| {
            ui.label("SubstanceCraft 日本語 テクスチャ");
        });
        output.drop_without_applying_deltas();
    }
}
