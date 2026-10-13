//! SubstanceCraft colour management.
//!
//! Transfer functions (sRGB / linear), tone-mapping operators and the working colour-space
//! conversions used by the compositor and viewport renderer. Implemented from public,
//! published formulae; never from proprietary profiles.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use glam::Vec3;

pub use glam;

/// Convert one sRGB-encoded channel to linear light.
pub fn srgb_channel_to_linear(channel: f32) -> f32 {
    if channel <= 0.04045 { channel / 12.92 } else { ((channel + 0.055) / 1.055).powf(2.4) }
}

/// Convert one linear-light channel to sRGB encoding.
pub fn linear_channel_to_srgb(channel: f32) -> f32 {
    if channel <= 0.003_130_8 { channel * 12.92 } else { 1.055 * channel.max(0.0).powf(1.0 / 2.4) - 0.055 }
}

/// Convert an sRGB-encoded colour to linear light, componentwise.
pub fn srgb_to_linear(srgb: Vec3) -> Vec3 {
    Vec3::new(srgb_channel_to_linear(srgb.x), srgb_channel_to_linear(srgb.y), srgb_channel_to_linear(srgb.z))
}

/// Convert a linear-light colour to sRGB encoding, componentwise.
pub fn linear_to_srgb(linear: Vec3) -> Vec3 {
    Vec3::new(linear_channel_to_srgb(linear.x), linear_channel_to_srgb(linear.y), linear_channel_to_srgb(linear.z))
}

/// The tone-mapping operator applied to the rendered (linear, HDR) image.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ToneMap {
    /// Hard clamp to `0..=1`; useful as a reference.
    None,
    /// Reinhard's simple `x / (1 + x)` curve.
    Reinhard,
    /// The ACES filmic approximation (Narkowicz).
    #[default]
    Aces,
    /// A compact AgX-style curve with saturation control.
    Agx,
    /// The Khronos PBR-neutral curve.
    Neutral,
}

/// Apply exposure (in stops) and the chosen tone map to a linear HDR colour, returning a
/// display-referred linear colour in `0..=1` (encode with [`linear_to_srgb`] for output).
pub fn tone_map(mode: ToneMap, color: Vec3, exposure_stops: f32) -> Vec3 {
    let exposed = color * 2.0f32.powf(exposure_stops.clamp(-20.0, 20.0));
    let mapped = match mode {
        ToneMap::None => exposed,
        ToneMap::Reinhard => exposed / (Vec3::ONE + exposed),
        ToneMap::Aces => aces(exposed),
        ToneMap::Agx => agx(exposed),
        ToneMap::Neutral => neutral(exposed),
    };
    mapped.clamp(Vec3::ZERO, Vec3::ONE)
}

/// ACES filmic approximation by Krzysztof Narkowicz.
fn aces(x: Vec3) -> Vec3 {
    const A: f32 = 2.51;
    const B: f32 = 0.03;
    const C: f32 = 2.43;
    const D: f32 = 0.59;
    const E: f32 = 0.14;
    ((x * (A * x + B)) / (x * (C * x + D) + E)).clamp(Vec3::ZERO, Vec3::ONE)
}

/// A compact AgX-style curve: log2 encoding, contrast S-curve, then a spread/saturation pass.
fn agx(x: Vec3) -> Vec3 {
    let mapped = x.max(Vec3::ZERO);
    let log = Vec3::new((mapped.x + 1e-5).log2(), (mapped.y + 1e-5).log2(), (mapped.z + 1e-5).log2());
    let normalized = ((log + 8.0) / 16.0).clamp(Vec3::ZERO, Vec3::ONE);
    let contrasted = normalized * normalized * (3.0 - 2.0 * normalized);
    let luma = contrasted.dot(Vec3::new(0.2126, 0.7152, 0.0722));
    let saturated = Vec3::splat(luma) + (contrasted - Vec3::splat(luma)) * 1.1;
    saturated.powf(2.2)
}

/// Khronos PBR-neutral curve (a gentle shoulder preserving mid-tones and hue).
fn neutral(x: Vec3) -> Vec3 {
    let start = 0.8;
    let d = 0.15;
    let peak = 1.0 + d;
    let x = x.max(Vec3::ZERO);
    let elbow = start + d;
    let scaled = (x - Vec3::splat(elbow)) / (Vec3::splat(peak) - Vec3::splat(elbow));
    let shoulder = Vec3::ONE - (Vec3::ONE - scaled).max(Vec3::ZERO).powf(2.0);
    let compressed = Vec3::splat(elbow) + shoulder * (Vec3::splat(peak) - Vec3::splat(elbow));
    Vec3::select(x.cmplt(Vec3::splat(elbow)), x, compressed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn srgb_round_trips() {
        for channel in [0.0f32, 0.02, 0.5, 0.9, 1.0] {
            let linear = srgb_channel_to_linear(channel);
            let back = linear_channel_to_srgb(linear);
            assert!((back - channel).abs() < 1e-4, "{channel} -> {linear} -> {back}");
        }
    }

    #[test]
    fn tone_map_is_bounded() {
        for mode in [ToneMap::None, ToneMap::Reinhard, ToneMap::Aces, ToneMap::Agx, ToneMap::Neutral] {
            let mapped = tone_map(mode, Vec3::new(8.0, 120.0, 0.001), 0.0);
            assert!(mapped.min_element() >= 0.0, "{mode:?} produced a negative channel");
            assert!(mapped.max_element() <= 1.0, "{mode:?} produced a channel above one");
        }
    }

    #[test]
    fn exposure_raises_brightness() {
        let base = tone_map(ToneMap::Reinhard, Vec3::splat(0.2), 0.0);
        let brighter = tone_map(ToneMap::Reinhard, Vec3::splat(0.2), 1.0);
        assert!(brighter.x > base.x);
    }

    #[test]
    fn handpicked_linear_is_zero() {
        let mapped = tone_map(ToneMap::Aces, Vec3::ZERO, 0.0);
        assert!(mapped.length() < 1e-6);
    }
}
