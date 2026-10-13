//! SubstanceCraft brush engine.
//!
//! Turns pointer samples into stroke coverage: dabs placed along a path at brush spacing, with
//! hardness falloff, pressure response and symmetry, stamped into a layer's [`RasterImage`]. This
//! crate is pure (no GPU, no UI); projection onto the mesh happens in the engine via `geom` raycasts.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use substancecraft_raster::RasterImage;

/// Mirror behaviour for a stroke.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Symmetry {
    /// No mirroring.
    #[default]
    None,
    /// Mirror across the vertical centre (`u = 0.5`).
    Horizontal,
    /// Mirror across the horizontal centre (`v = 0.5`).
    Vertical,
    /// Mirror both ways.
    Both,
}

impl Symmetry {
    /// Every mode, in UI order.
    pub const ALL: [Symmetry; 4] = [Symmetry::None, Symmetry::Horizontal, Symmetry::Vertical, Symmetry::Both];

    /// A stable id for commands and menus.
    pub fn id(self) -> &'static str {
        match self {
            Symmetry::None => "none",
            Symmetry::Horizontal => "horizontal",
            Symmetry::Vertical => "vertical",
            Symmetry::Both => "both",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Symmetry::None => "None",
            Symmetry::Horizontal => "Horizontal",
            Symmetry::Vertical => "Vertical",
            Symmetry::Both => "Both",
        }
    }

    /// Parse a symmetry mode from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        Symmetry::ALL.into_iter().find(|mode| mode.id() == id)
    }

    /// The stamp centres for one input centre, in texture pixels.
    pub fn centers(self, center: [f32; 2], width: f32, height: f32) -> Vec<[f32; 2]> {
        let mut centers = vec![center];
        if matches!(self, Symmetry::Horizontal | Symmetry::Both) {
            centers.push([width - center[0], center[1]]);
        }
        if matches!(self, Symmetry::Vertical | Symmetry::Both) {
            centers.push([center[0], height - center[1]]);
        }
        if self == Symmetry::Both {
            centers.push([width - center[0], height - center[1]]);
        }
        centers
    }
}

/// Brush parameters.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BrushSettings {
    /// Dab radius in texture pixels.
    pub radius_px: f32,
    /// Falloff hardness in `0..=1` (1 = hard edge).
    pub hardness: f32,
    /// Distance between dabs as a fraction of the radius.
    pub spacing: f32,
    /// Per-stroke opacity in `0..=1`.
    pub opacity: f32,
    /// Per-dab flow in `0..=1`.
    pub flow: f32,
    /// The paint colour (linear RGBA).
    pub color: [f32; 4],
    /// Whether pen pressure scales the radius.
    pub size_pressure: bool,
    /// Whether pen pressure scales opacity.
    pub opacity_pressure: bool,
    /// Mirroring.
    pub symmetry: Symmetry,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            radius_px: 48.0,
            hardness: 0.5,
            spacing: 0.15,
            opacity: 1.0,
            flow: 1.0,
            color: [0.9, 0.3, 0.2, 1.0],
            size_pressure: true,
            opacity_pressure: false,
            symmetry: Symmetry::None,
        }
    }
}

impl BrushSettings {
    /// The dab radius for a pressure value.
    pub fn radius_at(&self, pressure: f32) -> f32 {
        let scale = if self.size_pressure { pressure.clamp(0.0, 1.0) } else { 1.0 };
        (self.radius_px * scale).max(0.5)
    }

    /// The per-dab coverage for a pressure value.
    pub fn alpha_at(&self, pressure: f32) -> f32 {
        let scale = if self.opacity_pressure { pressure.clamp(0.0, 1.0) } else { 1.0 };
        (self.opacity.clamp(0.0, 1.0) * self.flow.clamp(0.0, 1.0) * scale).clamp(0.0, 1.0)
    }
}

/// The state of an in-progress stroke. Coordinates are in texture pixels.
#[derive(Clone, Debug, Default)]
pub struct StrokeState {
    last: Option<[f32; 2]>,
    distance_since_dab: f32,
}

impl StrokeState {
    /// A fresh stroke.
    pub fn new() -> Self {
        Self::default()
    }

    /// Begin a stroke at `center`, stamping the first dab.
    pub fn begin(&mut self, target: &mut RasterImage, settings: &BrushSettings, center: [f32; 2], pressure: f32) {
        self.last = Some(center);
        self.distance_since_dab = 0.0;
        self.stamp(target, settings, center, pressure);
    }

    /// Continue a stroke to `center`, emitting dabs at brush spacing along the way.
    pub fn extend(&mut self, target: &mut RasterImage, settings: &BrushSettings, center: [f32; 2], pressure: f32) {
        let Some(from) = self.last else {
            self.begin(target, settings, center, pressure);
            return;
        };
        let delta = [center[0] - from[0], center[1] - from[1]];
        let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        if distance <= 1e-6 {
            self.last = Some(center);
            return;
        }
        let step = (settings.spacing.clamp(0.01, 1.0) * settings.radius_at(pressure)).max(0.5);
        let direction = [delta[0] / distance, delta[1] / distance];
        let mut position = from;
        let mut remaining = distance;

        while self.distance_since_dab + remaining >= step {
            let advance = step - self.distance_since_dab;
            position = [position[0] + direction[0] * advance, position[1] + direction[1] * advance];
            remaining -= advance;
            self.stamp(target, settings, position, pressure);
            self.distance_since_dab = 0.0;
        }
        self.distance_since_dab += remaining;
        self.last = Some(center);
    }

    fn stamp(&self, target: &mut RasterImage, settings: &BrushSettings, center: [f32; 2], pressure: f32) {
        let radius = settings.radius_at(pressure);
        let alpha = settings.alpha_at(pressure);
        if alpha <= 0.0 {
            return;
        }
        for mirrored in settings.symmetry.centers(center, target.width as f32, target.height as f32) {
            paint_stamp(target, mirrored, radius, settings.hardness, settings.color, alpha);
        }
    }
}

/// Stamp one anti-aliased dab into the target (source-over).
pub fn paint_stamp(target: &mut RasterImage, center: [f32; 2], radius: f32, hardness: f32, color: [f32; 4], alpha: f32) {
    if target.is_empty() {
        return;
    }
    let radius = radius.max(0.5);
    let hardness = hardness.clamp(0.0, 1.0);
    let min_x = (center[0] - radius).floor().max(0.0) as u32;
    let max_x = ((center[0] + radius).ceil() as i64).clamp(0, target.width as i64 - 1) as u32;
    let min_y = (center[1] - radius).floor().max(0.0) as u32;
    let max_y = ((center[1] + radius).ceil() as i64).clamp(0, target.height as i64 - 1) as u32;

    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let dx = x as f32 + 0.5 - center[0];
            let dy = y as f32 + 0.5 - center[1];
            let distance = (dx * dx + dy * dy).sqrt() / radius;
            if distance >= 1.0 {
                continue;
            }
            let falloff = if distance <= hardness {
                1.0
            } else {
                let t = (1.0 - distance) / (1.0 - hardness).max(1e-4);
                t * t * (3.0 - 2.0 * t)
            };
            let coverage = (falloff * alpha).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let dst = target.pixel(x, y);
            let out = [
                dst[0] + (color[0] - dst[0]) * coverage,
                dst[1] + (color[1] - dst[1]) * coverage,
                dst[2] + (color[2] - dst[2]) * coverage,
                dst[3] + (1.0 - dst[3]) * coverage,
            ];
            target.set_pixel(x, y, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> BrushSettings {
        BrushSettings {
            radius_px: 8.0,
            hardness: 0.5,
            spacing: 0.25,
            opacity: 1.0,
            flow: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            size_pressure: false,
            opacity_pressure: false,
            symmetry: Symmetry::None,
        }
    }

    #[test]
    fn a_single_dab_paints_the_centre() {
        let mut target = RasterImage::new(64, 64);
        let mut stroke = StrokeState::new();
        stroke.begin(&mut target, &settings(), [32.0, 32.0], 1.0);
        let pixel = target.pixel(32, 32);
        assert!(pixel[0] > 0.9, "centre not painted: {pixel:?}");
        assert_eq!(target.pixel(0, 0), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_stroke_paints_along_the_path() {
        let mut target = RasterImage::new(128, 64);
        let mut stroke = StrokeState::new();
        let s = settings();
        stroke.begin(&mut target, &s, [10.0, 32.0], 1.0);
        stroke.extend(&mut target, &s, [110.0, 32.0], 1.0);
        for x in [20u32, 50, 80, 105] {
            assert!(target.pixel(x, 32)[0] > 0.5, "gap at x={x}");
        }
    }

    #[test]
    fn size_pressure_shrinks_the_dab() {
        let s = BrushSettings { size_pressure: true, ..settings() };
        let mut big = RasterImage::new(64, 64);
        let mut small = RasterImage::new(64, 64);
        StrokeState::new().begin(&mut big, &s, [32.0, 32.0], 1.0);
        StrokeState::new().begin(&mut small, &s, [32.0, 32.0], 0.25);
        let big_edge = big.pixel(32 + 7, 32)[0];
        let small_edge = small.pixel(32 + 7, 32)[0];
        assert!(big_edge > small_edge, "pressure should shrink the dab ({big_edge} vs {small_edge})");
    }

    #[test]
    fn symmetry_mirrors_dabs() {
        let s = BrushSettings { symmetry: Symmetry::Horizontal, ..settings() };
        let mut target = RasterImage::new(64, 64);
        StrokeState::new().begin(&mut target, &s, [10.0, 32.0], 1.0);
        assert!(target.pixel(10, 32)[0] > 0.5);
        assert!(target.pixel(54, 32)[0] > 0.5, "mirror dab missing");
    }

    #[test]
    fn symmetry_centers_are_correct() {
        let centers = Symmetry::Both.centers([0.0, 0.0], 10.0, 10.0);
        assert_eq!(centers.len(), 4);
        assert!(centers.contains(&[10.0, 10.0]));
    }
}
