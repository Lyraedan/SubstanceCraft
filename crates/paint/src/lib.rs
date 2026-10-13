//! SubstanceCraft brush engine.
//!
//! Turns pointer samples into stroke coverage: dabs placed along a path at brush spacing, with
//! hardness falloff, pressure response and symmetry. Provides brush, eraser, smudge and clone
//! stamps, plus UV-island polyfill. This crate is pure (no GPU, no UI); projection onto the mesh
//! happens in the engine via `geom` raycasts.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub mod fill;

use substancecraft_raster::RasterImage;

pub use fill::polyfill;

/// The available paint tools.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToolKind {
    /// Paints colour.
    #[default]
    Brush,
    /// Erases the active layer's alpha.
    Eraser,
    /// Smears existing colour along the stroke.
    Smudge,
    /// Copies colour from a source offset.
    Clone,
    /// Fills the connected UV island under the cursor.
    Polyfill,
}

impl ToolKind {
    /// Every tool, in UI order.
    pub const ALL: [ToolKind; 5] = [ToolKind::Brush, ToolKind::Eraser, ToolKind::Smudge, ToolKind::Clone, ToolKind::Polyfill];

    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            ToolKind::Brush => "brush",
            ToolKind::Eraser => "eraser",
            ToolKind::Smudge => "smudge",
            ToolKind::Clone => "clone",
            ToolKind::Polyfill => "polyfill",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            ToolKind::Brush => "Brush",
            ToolKind::Eraser => "Eraser",
            ToolKind::Smudge => "Smudge",
            ToolKind::Clone => "Clone",
            ToolKind::Polyfill => "Polyfill",
        }
    }

    /// Parse a tool from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        ToolKind::ALL.into_iter().find(|tool| tool.id() == id)
    }
}

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

    /// A stable id.
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
    /// Dab radius in texture pixels (used when no screen-space scale is supplied).
    pub radius_px: f32,
    /// Dab radius in screen pixels (used when a screen-space scale is supplied).
    pub screen_radius_px: f32,
    /// Falloff hardness in `0..=1` (1 = hard edge).
    pub hardness: f32,
    /// Distance between dabs as a fraction of the radius.
    pub spacing: f32,
    /// Per-dab opacity in `0..=1`.
    pub opacity: f32,
    /// Per-dab flow in `0..=1`.
    pub flow: f32,
    /// Scatter as a fraction of the radius (particle-like).
    pub jitter: f32,
    /// The paint colour (linear RGBA).
    pub color: [f32; 4],
    /// Whether pen pressure scales the radius.
    pub size_pressure: bool,
    /// Whether pen pressure scales opacity.
    pub opacity_pressure: bool,
    /// Mirroring.
    pub symmetry: Symmetry,
    /// Clone source offset in texture pixels.
    pub clone_offset: [f32; 2],
    /// Smudge strength in `0..=1`.
    pub smudge: f32,
}

impl Default for BrushSettings {
    fn default() -> Self {
        Self {
            radius_px: 48.0,
            screen_radius_px: 40.0,
            hardness: 0.5,
            spacing: 0.15,
            opacity: 1.0,
            flow: 1.0,
            jitter: 0.0,
            color: [0.9, 0.3, 0.2, 1.0],
            size_pressure: true,
            opacity_pressure: false,
            symmetry: Symmetry::None,
            clone_offset: [96.0, 0.0],
            smudge: 0.5,
        }
    }
}

impl BrushSettings {
    /// The dab radius for a pressure value, in texture pixels. When `scale` (texels per screen
    /// pixel) is positive, the screen-space radius is used; otherwise the fixed texel radius.
    pub fn radius_at(&self, pressure: f32, scale: f32) -> f32 {
        let pressure_scale = if self.size_pressure { pressure.clamp(0.0, 1.0) } else { 1.0 };
        let base = if scale > 0.0 { (self.screen_radius_px * scale).max(0.5) } else { self.radius_px };
        (base * pressure_scale).max(0.5)
    }

    /// The per-dab coverage for a pressure value.
    pub fn alpha_at(&self, pressure: f32) -> f32 {
        let scale = if self.opacity_pressure { pressure.clamp(0.0, 1.0) } else { 1.0 };
        (self.opacity.clamp(0.0, 1.0) * self.flow.clamp(0.0, 1.0) * scale).clamp(0.0, 1.0)
    }
}

/// The in-progress stroke's spacing state. Coordinates are in texture pixels.
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
    pub fn begin(&mut self, center: [f32; 2], mut stamp: impl FnMut([f32; 2])) {
        self.last = Some(center);
        self.distance_since_dab = 0.0;
        stamp(center);
    }

    /// Continue a stroke to `center`, emitting dabs spaced `step` pixels apart along the way.
    pub fn extend(&mut self, center: [f32; 2], step: f32, mut stamp: impl FnMut([f32; 2])) {
        let Some(from) = self.last else {
            self.begin(center, stamp);
            return;
        };
        let delta = [center[0] - from[0], center[1] - from[1]];
        let distance = (delta[0] * delta[0] + delta[1] * delta[1]).sqrt();
        if distance <= 1e-6 {
            self.last = Some(center);
            return;
        }
        let step = step.max(0.5);
        let direction = [delta[0] / distance, delta[1] / distance];
        let mut position = from;
        let mut remaining = distance;
        while self.distance_since_dab + remaining >= step {
            let advance = step - self.distance_since_dab;
            position = [position[0] + direction[0] * advance, position[1] + direction[1] * advance];
            remaining -= advance;
            stamp(position);
            self.distance_since_dab = 0.0;
        }
        self.distance_since_dab += remaining;
        self.last = Some(center);
    }

    /// Append the dab positions for a stroke point: the first point emits one dab, later points
    /// emit dabs spaced `step` pixels apart. This is the closure-free API used by the engine.
    pub fn dabs(&mut self, center: [f32; 2], step: f32, out: &mut Vec<[f32; 2]>) {
        if self.last.is_none() {
            self.last = Some(center);
            self.distance_since_dab = 0.0;
            out.push(center);
            return;
        }
        self.extend(center, step, |position| out.push(position));
    }

    /// The wrapped representative of `center` (unwrapped texture pixels) nearest to the previous
    /// point, so a stroke that crosses a UV seam stays continuous instead of jumping across the
    /// whole texture.
    fn unwrap(&self, center: [f32; 2], size: [f32; 2]) -> [f32; 2] {
        let Some(last) = self.last else { return center };
        if size[0] <= 0.0 || size[1] <= 0.0 {
            return center;
        }
        let u = ((last[0] - center[0]) / size[0]).round();
        let v = ((last[1] - center[1]) / size[1]).round();
        [center[0] + u * size[0], center[1] + v * size[1]]
    }

    /// Append the dab positions for a stroke point, following the texture wrap: a UV seam jump is
    /// folded back to the nearest representative so the stroke stays seamless. `center` is in
    /// unbounded texture pixels and `size` is the target's pixel size. Positions may lie outside
    /// `0..size`; the caller wraps them when stamping.
    pub fn dabs_wrapped(&mut self, center: [f32; 2], step: f32, size: [f32; 2], out: &mut Vec<[f32; 2]>) {
        let center = self.unwrap(center, size);
        self.dabs(center, step, out);
    }
}

/// The falloff at a normalised distance (`0` centre, `1` edge) for a hardness.
fn falloff(distance: f32, hardness: f32) -> f32 {
    if distance >= 1.0 {
        return 0.0;
    }
    if distance <= hardness {
        return 1.0;
    }
    let t = (1.0 - distance) / (1.0 - hardness).max(1e-4);
    t * t * (3.0 - 2.0 * t)
}

/// Iterate the pixels under a dab, calling `visit(x, y, coverage)`. Coordinates wrap around the
/// image edges so a dab straddling a UV seam paints both sides seamlessly. The radius is capped at
/// half the smaller dimension so the wrapped footprint never revisits a pixel twice.
fn for_each_dab_pixel(target: &RasterImage, center: [f32; 2], radius: f32, hardness: f32, alpha: f32, mut visit: impl FnMut(u32, u32, f32)) {
    if target.is_empty() || alpha <= 0.0 {
        return;
    }
    let width = target.width as f32;
    let height = target.height as f32;
    let center = [center[0].rem_euclid(width), center[1].rem_euclid(height)];
    let radius = radius.max(0.5).min(width * 0.5).min(height * 0.5).max(0.5);
    let min_x = (center[0] - radius).floor() as i64;
    let max_x = (center[0] + radius).ceil() as i64;
    let min_y = (center[1] - radius).floor() as i64;
    let max_y = (center[1] + radius).ceil() as i64;
    let (w, h) = (target.width as i64, target.height as i64);
    for iy in min_y..=max_y {
        for ix in min_x..=max_x {
            let dx = ix as f32 + 0.5 - center[0];
            let dy = iy as f32 + 0.5 - center[1];
            let coverage = falloff((dx * dx + dy * dy).sqrt() / radius, hardness) * alpha;
            if coverage > 0.0 {
                visit(ix.rem_euclid(w) as u32, iy.rem_euclid(h) as u32, coverage.clamp(0.0, 1.0));
            }
        }
    }
}

/// Stamp one anti-aliased brush dab (source-over).
pub fn paint_stamp(target: &mut RasterImage, center: [f32; 2], radius: f32, hardness: f32, color: [f32; 4], alpha: f32) {
    let mut writes = Vec::new();
    for_each_dab_pixel(target, center, radius, hardness, alpha, |x, y, coverage| writes.push((x, y, coverage)));
    for (x, y, coverage) in writes {
        let dst = target.pixel(x, y);
        target.set_pixel(
            x,
            y,
            [
                dst[0] + (color[0] - dst[0]) * coverage,
                dst[1] + (color[1] - dst[1]) * coverage,
                dst[2] + (color[2] - dst[2]) * coverage,
                dst[3] + (1.0 - dst[3]) * coverage,
            ],
        );
    }
}

/// Stamp an eraser dab, reducing alpha.
pub fn erase_stamp(target: &mut RasterImage, center: [f32; 2], radius: f32, hardness: f32, strength: f32) {
    let mut writes = Vec::new();
    for_each_dab_pixel(target, center, radius, hardness, strength, |x, y, coverage| writes.push((x, y, coverage)));
    for (x, y, coverage) in writes {
        let mut pixel = target.pixel(x, y);
        pixel[3] *= 1.0 - coverage;
        target.set_pixel(x, y, pixel);
    }
}

/// Stamp a smudge dab: pull colour from `source` at `pick` toward the target at `center`.
pub fn smudge_stamp(target: &mut RasterImage, source: &RasterImage, center: [f32; 2], pick: [f32; 2], radius: f32, hardness: f32, strength: f32) {
    let sampled = source.sample_pixel(pick);
    paint_stamp(target, center, radius, hardness, sampled, strength);
}

/// Stamp a clone dab: copy `source` at `center + offset` into the target at `center`.
pub fn clone_stamp(target: &mut RasterImage, source: &RasterImage, center: [f32; 2], radius: f32, hardness: f32, offset: [f32; 2], alpha: f32) {
    let sampled = source.sample_pixel([center[0] + offset[0], center[1] + offset[1]]);
    paint_stamp(target, center, radius, hardness, sampled, alpha);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> BrushSettings {
        BrushSettings {
            radius_px: 8.0,
            screen_radius_px: 8.0,
            hardness: 0.5,
            spacing: 0.25,
            opacity: 1.0,
            flow: 1.0,
            color: [1.0, 0.0, 0.0, 1.0],
            size_pressure: false,
            ..BrushSettings::default()
        }
    }

    #[test]
    fn a_single_dab_paints_the_centre() {
        let mut target = RasterImage::new(64, 64);
        let mut stroke = StrokeState::new();
        stroke.begin([32.0, 32.0], |center| paint_stamp(&mut target, center, 8.0, 0.5, [1.0, 0.0, 0.0, 1.0], 1.0));
        assert!(target.pixel(32, 32)[0] > 0.9);
        assert_eq!(target.pixel(0, 0), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn a_stroke_paints_along_the_path() {
        let mut target = RasterImage::new(128, 64);
        let mut stroke = StrokeState::new();
        stroke.begin([10.0, 32.0], |c| paint_stamp(&mut target, c, 8.0, 0.5, [1.0, 0.0, 0.0, 1.0], 1.0));
        stroke.extend([110.0, 32.0], 2.0, |c| paint_stamp(&mut target, c, 8.0, 0.5, [1.0, 0.0, 0.0, 1.0], 1.0));
        for x in [20u32, 50, 80, 105] {
            assert!(target.pixel(x, 32)[0] > 0.5, "gap at x={x}");
        }
    }

    #[test]
    fn eraser_reduces_alpha() {
        let mut target = RasterImage::filled(64, 64, [1.0, 0.0, 0.0, 1.0]);
        erase_stamp(&mut target, [32.0, 32.0], 8.0, 0.5, 1.0);
        assert!(target.pixel(32, 32)[3] < 0.1);
        assert!(target.pixel(0, 0)[3] > 0.9);
    }

    #[test]
    fn clone_copies_from_the_offset() {
        let mut source = RasterImage::new(64, 64);
        source.set_pixel(10, 10, [0.0, 1.0, 0.0, 1.0]);
        let mut target = RasterImage::new(64, 64);
        clone_stamp(&mut target, &source, [30.0, 30.0], 8.0, 0.5, [-20.0, -20.0], 1.0);
        assert!(target.pixel(30, 30)[1] > 0.5, "clone did not copy the source colour");
    }

    #[test]
    fn smudge_pulls_colour() {
        let mut source = RasterImage::new(64, 64);
        source.set_pixel(20, 20, [0.0, 0.0, 1.0, 1.0]);
        let mut target = RasterImage::new(64, 64);
        smudge_stamp(&mut target, &source, [20.0, 20.0], [20.0, 20.0], 8.0, 0.5, 1.0);
        assert!(target.pixel(20, 20)[2] > 0.5);
    }

    #[test]
    fn size_pressure_shrinks_the_dab() {
        let s = BrushSettings { size_pressure: true, ..settings() };
        let mut big = RasterImage::new(64, 64);
        let mut small = RasterImage::new(64, 64);
        paint_stamp(&mut big, [32.0, 32.0], s.radius_at(1.0, 0.0), s.hardness, s.color, 1.0);
        paint_stamp(&mut small, [32.0, 32.0], s.radius_at(0.25, 0.0), s.hardness, s.color, 1.0);
        assert!(big.pixel(39, 32)[0] > small.pixel(39, 32)[0]);
    }

    #[test]
    fn symmetry_mirrors_and_centers_are_correct() {
        let centers = Symmetry::Both.centers([0.0, 0.0], 10.0, 10.0);
        assert_eq!(centers.len(), 4);
        assert!(centers.contains(&[10.0, 10.0]));
    }

    #[test]
    fn tool_ids_round_trip() {
        for tool in ToolKind::ALL {
            assert_eq!(ToolKind::from_id(tool.id()), Some(tool));
        }
        assert_eq!(ToolKind::from_id("nope"), None);
    }

    #[test]
    fn a_stroke_crossing_the_seam_stays_continuous() {
        let mut stroke = StrokeState::new();
        let mut out = Vec::new();
        // A seam jump: u goes 0.875 -> 0.125 (a raw "-48px" jump on a 64px texture).
        stroke.dabs_wrapped([56.0, 32.0], 2.0, [64.0, 64.0], &mut out);
        stroke.dabs_wrapped([8.0, 32.0], 2.0, [64.0, 64.0], &mut out);
        // Folded to 72, so the dabs continue past the right edge and never cross the middle.
        assert!(out.iter().any(|position| position[0] > 63.0), "stroke did not continue across the seam: {out:?}");
        assert!(out.iter().all(|position| position[0] < 32.0 || position[0] >= 56.0), "stroke crossed the middle: {out:?}");
    }

    #[test]
    fn a_dab_at_the_edge_wraps_to_the_other_side() {
        let mut target = RasterImage::new(64, 64);
        paint_stamp(&mut target, [0.5, 32.0], 6.0, 1.0, [1.0, 0.0, 0.0, 1.0], 1.0);
        assert!(target.pixel(0, 32)[0] > 0.0, "dab missed the near edge");
        assert!(target.pixel(63, 32)[0] > 0.0, "dab did not wrap across the edge to the far side");
    }
}
