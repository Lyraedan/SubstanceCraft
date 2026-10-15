//! Render settings: which data the viewport shows and how it is graded.

pub use substancecraft_color::ToneMap;

/// Which surface data the viewport displays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum DisplayMode {
    /// Fully shaded metal/rough result.
    #[default]
    BaseColor,
    /// Object-space shading normals.
    Normal,
    /// The roughness channel.
    Roughness,
    /// The metalness channel.
    Metalness,
    /// A UV checker to inspect the unwrap.
    UvChecker,
    /// Triangle edges only.
    Wireframe,
    /// The ambient-occlusion channel.
    AmbientOcclusion,
    /// The height channel (as greyscale).
    Height,
    /// A neutral matcap shaded from the normal.
    Matcap,
}

impl DisplayMode {
    /// A wire value passed to the shader.
    pub fn shader_code(self) -> f32 {
        match self {
            DisplayMode::BaseColor => 0.0,
            DisplayMode::Normal => 1.0,
            DisplayMode::Roughness => 2.0,
            DisplayMode::Metalness => 3.0,
            DisplayMode::UvChecker => 4.0,
            DisplayMode::Wireframe => 5.0,
            DisplayMode::AmbientOcclusion => 6.0,
            DisplayMode::Height => 7.0,
            DisplayMode::Matcap => 8.0,
        }
    }

    /// A stable id for commands and menus.
    pub fn id(self) -> &'static str {
        match self {
            DisplayMode::BaseColor => "base_color",
            DisplayMode::Normal => "normal",
            DisplayMode::Roughness => "roughness",
            DisplayMode::Metalness => "metalness",
            DisplayMode::UvChecker => "uv_checker",
            DisplayMode::Wireframe => "wireframe",
            DisplayMode::AmbientOcclusion => "ambient_occlusion",
            DisplayMode::Height => "height",
            DisplayMode::Matcap => "matcap",
        }
    }

    /// Parse a display mode from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "base_color" => DisplayMode::BaseColor,
            "normal" => DisplayMode::Normal,
            "roughness" => DisplayMode::Roughness,
            "metalness" => DisplayMode::Metalness,
            "uv_checker" => DisplayMode::UvChecker,
            "wireframe" => DisplayMode::Wireframe,
            "ambient_occlusion" => DisplayMode::AmbientOcclusion,
            "height" => DisplayMode::Height,
            "matcap" => DisplayMode::Matcap,
            _ => return None,
        })
    }

    /// Whether this mode samples the uploaded display texture.
    pub fn uses_texture(self) -> bool {
        matches!(self, DisplayMode::BaseColor | DisplayMode::AmbientOcclusion | DisplayMode::Height)
    }
}

/// How the viewport grades and shades the scene.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RenderSettings {
    /// Which channel to display.
    pub display: DisplayMode,
    /// Exposure in stops.
    pub exposure_stops: f32,
    /// The tone-mapping operator.
    pub tone_map: ToneMap,
    /// The linear background colour.
    pub background: [f32; 3],
    /// The key-light direction (world space, pointing from the surface toward the light).
    pub light_direction: [f32; 3],
    /// The key-light linear colour (its magnitude is intensity).
    pub light_color: [f32; 3],
    /// The sky hemisphere ambient colour.
    pub ambient_sky: [f32; 3],
    /// The ground hemisphere ambient colour.
    pub ambient_ground: [f32; 3],
    /// Whether to draw the ground grid.
    pub show_grid: bool,
    /// Whether to draw the procedural sky background (otherwise a flat clear colour).
    pub show_background: bool,
    /// Whether to draw only the active texture set's geometry.
    pub isolate_active_set: bool,
    /// Post-effect vignette strength in `0..=1`.
    pub vignette: f32,
    /// Post-effect bloom strength in `0..=1`.
    pub bloom: f32,
}

impl Default for RenderSettings {
    fn default() -> Self {
        Self {
            display: DisplayMode::BaseColor,
            exposure_stops: 0.0,
            tone_map: ToneMap::Aces,
            background: [0.02, 0.022, 0.026],
            light_direction: [0.5, 0.8, 0.6],
            light_color: [3.0, 2.9, 2.7],
            ambient_sky: [0.35, 0.45, 0.6],
            ambient_ground: [0.08, 0.07, 0.06],
            show_grid: true,
            show_background: true,
            isolate_active_set: false,
            vignette: 0.0,
            bloom: 0.0,
        }
    }
}

impl RenderSettings {
    /// The tone-map id used by the shader.
    pub fn tone_map_code(&self) -> f32 {
        match self.tone_map {
            ToneMap::None => 0.0,
            ToneMap::Reinhard => 1.0,
            ToneMap::Aces => 2.0,
            ToneMap::Agx => 3.0,
            ToneMap::Neutral => 4.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mode_ids_round_trip() {
        for mode in [
            DisplayMode::BaseColor,
            DisplayMode::Normal,
            DisplayMode::Roughness,
            DisplayMode::Metalness,
            DisplayMode::UvChecker,
            DisplayMode::Wireframe,
            DisplayMode::AmbientOcclusion,
            DisplayMode::Height,
            DisplayMode::Matcap,
        ] {
            assert_eq!(DisplayMode::from_id(mode.id()), Some(mode));
        }
    }

    #[test]
    fn unknown_display_mode_is_none() {
        assert_eq!(DisplayMode::from_id("nope"), None);
    }
}
