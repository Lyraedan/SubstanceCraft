//! SubstanceCraft image and texture codecs.
//!
//! Standalone decoders/encoders with no workspace dependencies on the document or
//! engine, so they can be reused and tested in isolation.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::Path;

use thiserror::Error;

/// Errors from decoding. Every one is recoverable and reported, never panicked.
#[derive(Debug, Error)]
pub enum CodecError {
    /// The operating system refused the read.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The decoder rejected the file.
    #[error("could not decode image: {0}")]
    Decode(String),
    /// The format has no decoder here.
    #[error("unsupported environment format: {0}")]
    Unsupported(String),
    /// The image carried no pixels.
    #[error("image is empty")]
    Empty,
}

/// A linear, 32-bit float, RGBA image in row-major order.
#[derive(Clone, Debug)]
pub struct ImageF32 {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Row-major RGBA pixels (linear light).
    pub pixels: Vec<[f32; 4]>,
}

impl ImageF32 {
    /// A pixel, or transparent black if out of range.
    pub fn pixel(&self, x: u32, y: u32) -> [f32; 4] {
        if x >= self.width || y >= self.height {
            return [0.0, 0.0, 0.0, 0.0];
        }
        self.pixels.get((y as usize) * (self.width as usize) + (x as usize)).copied().unwrap_or([0.0, 0.0, 0.0, 0.0])
    }

    /// Whether the image has no pixels.
    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0 || self.pixels.is_empty()
    }
}

/// Load a linear HDR environment map from `.hdr` (Radiance) or `.exr`.
pub fn load_environment(path: impl AsRef<Path>) -> Result<ImageF32, CodecError> {
    let path = path.as_ref();
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or_default().to_ascii_lowercase();
    match extension.as_str() {
        "hdr" => load_radiance_hdr(path),
        "exr" => load_exr(path),
        other => Err(CodecError::Unsupported(other.to_string())),
    }
}

/// Decode a Radiance `.hdr` file to linear RGBA.
pub fn load_radiance_hdr(path: impl AsRef<Path>) -> Result<ImageF32, CodecError> {
    let image = image::open(path.as_ref()).map_err(|err| CodecError::Decode(err.to_string()))?;
    let rgb = image.to_rgb32f();
    let (width, height) = rgb.dimensions();
    if width == 0 || height == 0 {
        return Err(CodecError::Empty);
    }
    let pixels = rgb.pixels().map(|pixel| [pixel[0], pixel[1], pixel[2], 1.0]).collect();
    Ok(ImageF32 { width, height, pixels })
}

/// Decode an `.exr` file to linear RGBA.
pub fn load_exr(path: impl AsRef<Path>) -> Result<ImageF32, CodecError> {
    use exr::prelude::*;

    let image = read_first_rgba_layer_from_file(
        path.as_ref(),
        |resolution, _channels: &RgbaChannels| -> Vec<Vec<[f32; 4]>> { vec![vec![[0.0, 0.0, 0.0, 1.0]; resolution.width()]; resolution.height()] },
        |pixels: &mut Vec<Vec<[f32; 4]>>, position: Vec2<usize>, (r, g, b, a): (f32, f32, f32, f32)| {
            if let Some(row) = pixels.get_mut(position.y())
                && let Some(pixel) = row.get_mut(position.x())
            {
                *pixel = [r, g, b, a];
            }
        },
    )
    .map_err(|err| CodecError::Decode(err.to_string()))?;

    let size = image.layer_data.size;
    let rows = image.layer_data.channel_data.pixels;
    let mut pixels = Vec::with_capacity(size.width() * size.height());
    for row in rows {
        pixels.extend_from_slice(&row);
    }
    if pixels.is_empty() {
        return Err(CodecError::Empty);
    }
    Ok(ImageF32 { width: size.width() as u32, height: size.height() as u32, pixels })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_file_is_an_error_not_a_panic() {
        assert!(load_environment("/definitely/not/here.hdr").is_err());
    }

    #[test]
    fn unsupported_extension_is_reported() {
        assert!(matches!(load_environment("/tmp/thing.tif"), Err(CodecError::Unsupported(_))));
    }

    #[test]
    fn pixel_lookup_is_bounds_checked() {
        let image = ImageF32 { width: 1, height: 1, pixels: vec![[0.25, 0.5, 0.75, 1.0]] };
        assert_eq!(image.pixel(0, 0), [0.25, 0.5, 0.75, 1.0]);
        assert_eq!(image.pixel(5, 5), [0.0, 0.0, 0.0, 0.0]);
    }

    #[test]
    fn exr_round_trips() {
        let path = std::env::temp_dir().join(format!("substancecraft_codec_{}.exr", std::process::id()));
        // Skip cleanly if the environment cannot write.
        if exr::prelude::write_rgb_file(&path, 4, 2, |x, y| (x as f32 * 0.1, y as f32 * 0.2, 0.5)).is_err() {
            return;
        }
        let image = load_exr(&path).expect("load exr");
        assert_eq!((image.width, image.height), (4, 2));
        let pixel = image.pixel(2, 1);
        assert!((pixel[0] - 0.2).abs() < 1e-3, "r={}", pixel[0]);
        assert!((pixel[1] - 0.2).abs() < 1e-3, "g={}", pixel[1]);
        assert!((pixel[2] - 0.5).abs() < 1e-3, "b={}", pixel[2]);
        let _ = std::fs::remove_file(&path);
    }
}
