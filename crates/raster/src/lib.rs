//! SubstanceCraft raster foundation.
//!
//! Depth-agnostic pixel buffers and the per-channel image type shared by the CPU and GPU
//! compositors and the paint engine. Never assume a `u8`-only pixel path: the working format is
//! linear `f32` RGBA.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

pub use bytemuck;

/// A linear, 32-bit float, RGBA image in row-major order.
///
/// This is the compositor's working surface and the paint engine's backing store.
#[derive(Clone, Debug, PartialEq)]
pub struct RasterImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA pixels.
    pub pixels: Vec<[f32; 4]>,
}

impl RasterImage {
    /// An image of `width × height` filled with transparent black.
    pub fn new(width: u32, height: u32) -> Self {
        let count = (width as usize).saturating_mul(height as usize);
        Self { width, height, pixels: vec![[0.0, 0.0, 0.0, 0.0]; count] }
    }

    /// An image of `width × height` filled with a constant colour.
    pub fn filled(width: u32, height: u32, color: [f32; 4]) -> Self {
        let count = (width as usize).saturating_mul(height as usize);
        Self { width, height, pixels: vec![color; count] }
    }

    /// Whether the image has no pixels.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0 || self.pixels.is_empty()
    }

    /// The pixel at `(x, y)`, or transparent black if out of range.
    pub fn pixel(&self, x: u32, y: u32) -> [f32; 4] {
        if x >= self.width || y >= self.height {
            return [0.0, 0.0, 0.0, 0.0];
        }
        self.pixels.get((y as usize) * (self.width as usize) + (x as usize)).copied().unwrap_or([0.0, 0.0, 0.0, 0.0])
    }

    /// Set the pixel at `(x, y)`; out-of-range writes are ignored.
    pub fn set_pixel(&mut self, x: u32, y: u32, value: [f32; 4]) {
        if x >= self.width || y >= self.height {
            return;
        }
        let index = (y as usize) * (self.width as usize) + (x as usize);
        if let Some(slot) = self.pixels.get_mut(index) {
            *slot = value;
        }
    }

    /// Nearest-neighbour sample by normalised UV in `0..=1` (wrapping).
    pub fn sample_nearest(&self, u: f32, v: f32) -> [f32; 4] {
        if self.is_empty() {
            return [0.0, 0.0, 0.0, 0.0];
        }
        let x = (u.fract().abs() * self.width as f32) as u32;
        let y = (v.fract().abs() * self.height as f32) as u32;
        self.pixel(x.min(self.width - 1), y.min(self.height - 1))
    }

    /// Convert to tightly packed RGBA8 with sRGB gamma encoding and clamping (for previews/export).
    pub fn to_rgba8(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(self.pixels.len() * 4);
        for pixel in &self.pixels {
            bytes.push(encode_srgb(pixel[0]));
            bytes.push(encode_srgb(pixel[1]));
            bytes.push(encode_srgb(pixel[2]));
            bytes.push((pixel[3].clamp(0.0, 1.0) * 255.0 + 0.5) as u8);
        }
        bytes
    }
}

/// Clamp and sRGB-encode one linear channel to a byte.
fn encode_srgb(value: f32) -> u8 {
    let value = value.clamp(0.0, 1.0);
    let encoded = if value <= 0.003_130_8 { value * 12.92 } else { 1.055 * value.powf(1.0 / 2.4) - 0.055 };
    (encoded * 255.0 + 0.5) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_transparent_black() {
        let image = RasterImage::new(4, 2);
        assert_eq!(image.pixels.len(), 8);
        assert_eq!(image.pixel(1, 1), [0.0, 0.0, 0.0, 0.0]);
        assert_eq!(image.pixel(9, 9), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn set_and_get_round_trip() {
        let mut image = RasterImage::new(2, 2);
        image.set_pixel(1, 0, [0.2, 0.4, 0.6, 1.0]);
        assert_eq!(image.pixel(1, 0), [0.2, 0.4, 0.6, 1.0]);
        image.set_pixel(5, 5, [1.0; 4]);
        assert_eq!(image.pixel(1, 0), [0.2, 0.4, 0.6, 1.0]);
    }

    #[test]
    fn nearest_sampling_wraps() {
        let image = RasterImage::filled(2, 2, [0.0, 0.0, 0.0, 1.0]);
        assert_eq!(image.sample_nearest(0.0, 0.0)[3], 1.0);
        assert_eq!(image.sample_nearest(1.5, -0.5)[3], 1.0);
    }
}
