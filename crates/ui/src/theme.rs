//! Design tokens: the single source of truth for colours and radii.
//!
//! Views read from [`Tokens`]; they never hard-code a colour. Palettes are deliberately
//! original to SubstanceCraft (no Adobe colours or assets).

use craft_ui::theme::WidgetPalette;
use egui::{Color32, CornerRadius, Stroke, Visuals};

/// A complete application palette and geometry scale.
#[derive(Clone, Copy, Debug)]
pub struct Tokens {
    /// The desktop behind the panels.
    pub window_bg: Color32,
    /// Panel and toolbar backgrounds.
    pub panel_bg: Color32,
    /// The menu bar background (slightly darker than the panels).
    pub menu_bg: Color32,
    /// The left icon tool rail background.
    pub rail_bg: Color32,
    /// Section header / tab-strip backgrounds inside panels.
    pub header_bg: Color32,
    /// Raised surfaces: rows, inputs, buttons.
    pub surface: Color32,
    /// Surface under the pointer.
    pub surface_hover: Color32,
    /// Surface while pressed / selected.
    pub surface_active: Color32,
    /// Accent used for selection and focus.
    pub accent: Color32,
    /// Primary text.
    pub text: Color32,
    /// Secondary / dimmed text.
    pub text_dim: Color32,
    /// Hairline borders.
    pub border: Color32,
    /// The viewport letterbox behind the mesh.
    pub viewport_bg: Color32,
    /// Corner radius for widgets, in pixels.
    pub radius: u8,
}

impl Tokens {
    /// The default dark studio theme.
    pub const DARK: Self = Self {
        window_bg: Color32::from_rgb(20, 21, 24),
        panel_bg: Color32::from_rgb(30, 32, 36),
        menu_bg: Color32::from_rgb(24, 25, 28),
        rail_bg: Color32::from_rgb(26, 27, 31),
        header_bg: Color32::from_rgb(36, 38, 43),
        surface: Color32::from_rgb(42, 45, 50),
        surface_hover: Color32::from_rgb(54, 58, 64),
        surface_active: Color32::from_rgb(66, 71, 78),
        accent: Color32::from_rgb(86, 156, 214),
        text: Color32::from_rgb(226, 228, 232),
        text_dim: Color32::from_rgb(150, 154, 160),
        border: Color32::from_rgb(58, 61, 66),
        viewport_bg: Color32::from_rgb(14, 15, 17),
        radius: 5,
    };

    /// Install these tokens into an egui context.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = Visuals::dark();
        visuals.panel_fill = self.panel_bg;
        visuals.window_fill = self.panel_bg;
        visuals.faint_bg_color = self.surface;
        visuals.extreme_bg_color = self.window_bg;
        visuals.override_text_color = Some(self.text);
        visuals.hyperlink_color = self.accent;
        visuals.selection.bg_fill = self.accent;
        visuals.selection.stroke = Stroke::new(1.0, self.text);
        visuals.window_stroke = Stroke::new(1.0, self.border);
        visuals.window_corner_radius = CornerRadius::same(self.radius);

        let mut widgets = visuals.widgets.clone();
        WidgetPalette {
            noninteractive: self.panel_bg,
            inactive: self.surface,
            hovered: self.surface_hover,
            active: self.surface_active,
            open: self.surface_active,
        }
        .apply_to(&mut widgets, CornerRadius::same(self.radius), Stroke::new(1.0, self.text));
        visuals.widgets = widgets;

        ctx.set_theme(egui::ThemePreference::Dark);
        ctx.set_visuals(visuals);

        ctx.all_styles_mut(|style| {
            style.spacing.item_spacing = egui::vec2(6.0, 6.0);
            style.spacing.button_padding = egui::vec2(8.0, 4.0);
        });
    }
}

impl Default for Tokens {
    fn default() -> Self {
        Self::DARK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applying_tokens_uses_only_the_palette() {
        let ctx = egui::Context::default();
        Tokens::DARK.apply(&ctx);
        let visuals = ctx.style_of(egui::Theme::Dark).visuals.clone();
        assert_eq!(visuals.panel_fill, Tokens::DARK.panel_bg);
        assert_eq!(visuals.widgets.inactive.bg_fill, Tokens::DARK.surface);
        assert_eq!(visuals.selection.bg_fill, Tokens::DARK.accent);
    }
}
