//! SubstanceCraft procedural material content.
//!
//! Evaluates the generators and filters described in `substancecraft-doc` into images, all
//! implemented from public maths (gradient/simplex/Worley noise, standard image filters), never
//! from proprietary content.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::f32::consts::TAU;

use substancecraft_doc::{Filter, Generator};
use substancecraft_raster::RasterImage;

/// Generate a grayscale texture (value replicated across RGB, alpha 1).
pub fn generate(generator: &Generator, width: u32, height: u32) -> RasterImage {
    let width = width.max(1);
    let height = height.max(1);
    let mut image = RasterImage::new(width, height);
    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        for x in 0..width {
            let u = (x as f32 + 0.5) / width as f32;
            let value = match *generator {
                Generator::Perlin { scale, octaves, seed } => fbm(u * scale, v * scale, octaves, seed, false),
                Generator::Simplex { scale, octaves, seed } => fbm(u * scale, v * scale, octaves, seed, true),
                Generator::Voronoi { cells, seed } => voronoi(u * cells, v * cells, seed),
                Generator::White { seed } => hash_unit(x as i32, y as i32, seed),
            };
            let value = value.clamp(0.0, 1.0);
            image.set_pixel(x, y, [value, value, value, 1.0]);
        }
    }
    image
}

/// Apply a filter to an image, returning a new image.
pub fn apply(filter: &Filter, image: &RasterImage) -> RasterImage {
    match *filter {
        Filter::Invert => map_rgb(image, |c| 1.0 - c),
        Filter::Levels { in_black, in_white, gamma, out_black, out_white } => {
            let range = (in_white - in_black).abs().max(1e-6);
            let gamma = gamma.max(1e-3);
            map_rgb(image, |c| {
                let t = ((c - in_black) / range).clamp(0.0, 1.0);
                out_black + t.powf(1.0 / gamma) * (out_white - out_black)
            })
        }
        Filter::Contrast { amount } => map_rgb(image, |c| (c - 0.5) * amount + 0.5),
        Filter::Brightness { amount } => map_rgb(image, |c| c + amount),
        Filter::Blur { radius } => box_blur(image, radius),
    }
}

/// Map every RGB channel through `f`, clamping the result and preserving alpha.
fn map_rgb(image: &RasterImage, f: impl Fn(f32) -> f32) -> RasterImage {
    let mut out = image.clone();
    for pixel in &mut out.pixels {
        pixel[0] = f(pixel[0]).clamp(0.0, 1.0);
        pixel[1] = f(pixel[1]).clamp(0.0, 1.0);
        pixel[2] = f(pixel[2]).clamp(0.0, 1.0);
    }
    out
}

/// A separable box blur of the colour channels.
fn box_blur(image: &RasterImage, radius: u32) -> RasterImage {
    if radius == 0 || image.is_empty() {
        return image.clone();
    }
    let (width, height) = (image.width, image.height);
    let radius = radius as i32;
    let mut horizontal = RasterImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let mut accumulator = [0.0f32; 3];
            let mut count = 0.0f32;
            for k in -radius..=radius {
                let sample_x = (x as i32 + k).clamp(0, width as i32 - 1) as u32;
                let pixel = image.pixel(sample_x, y);
                accumulator[0] += pixel[0];
                accumulator[1] += pixel[1];
                accumulator[2] += pixel[2];
                count += 1.0;
            }
            let alpha = image.pixel(x, y)[3];
            horizontal.set_pixel(x, y, [accumulator[0] / count, accumulator[1] / count, accumulator[2] / count, alpha]);
        }
    }

    let mut result = RasterImage::new(width, height);
    for y in 0..height {
        for x in 0..width {
            let mut accumulator = [0.0f32; 3];
            let mut count = 0.0f32;
            for k in -radius..=radius {
                let sample_y = (y as i32 + k).clamp(0, height as i32 - 1) as u32;
                let pixel = horizontal.pixel(x, sample_y);
                accumulator[0] += pixel[0];
                accumulator[1] += pixel[1];
                accumulator[2] += pixel[2];
                count += 1.0;
            }
            let alpha = image.pixel(x, y)[3];
            result.set_pixel(x, y, [accumulator[0] / count, accumulator[1] / count, accumulator[2] / count, alpha]);
        }
    }
    result
}

/// Sum of noise octaves, normalised to `0..1`.
fn fbm(x: f32, y: f32, octaves: u32, seed: u32, simplex: bool) -> f32 {
    let octaves = octaves.clamp(1, 8);
    let mut amplitude = 1.0f32;
    let mut frequency = 1.0f32;
    let mut total = 0.0f32;
    let mut norm = 0.0f32;
    for octave in 0..octaves {
        let sample_seed = seed.wrapping_add(octave.wrapping_mul(1013));
        let noise = if simplex { simplex2(x * frequency, y * frequency, sample_seed) } else { perlin2(x * frequency, y * frequency, sample_seed) };
        total += noise * amplitude;
        norm += amplitude;
        amplitude *= 0.5;
        frequency *= 2.0;
    }
    if norm > 0.0 { (total / norm) * 0.5 + 0.5 } else { 0.5 }
}

/// Classic Perlin gradient noise, roughly in `-1..1`.
fn perlin2(x: f32, y: f32, seed: u32) -> f32 {
    let x0 = x.floor() as i32;
    let y0 = y.floor() as i32;
    let xf = x - x0 as f32;
    let yf = y - y0 as f32;
    let u = fade(xf);
    let v = fade(yf);

    let n00 = dot_gradient(x0, y0, seed, xf, yf);
    let n10 = dot_gradient(x0 + 1, y0, seed, xf - 1.0, yf);
    let n01 = dot_gradient(x0, y0 + 1, seed, xf, yf - 1.0);
    let n11 = dot_gradient(x0 + 1, y0 + 1, seed, xf - 1.0, yf - 1.0);

    lerp(lerp(n00, n10, u), lerp(n01, n11, u), v)
}

/// 2D simplex noise, roughly in `-1..1`.
fn simplex2(x: f32, y: f32, seed: u32) -> f32 {
    const F2: f32 = 0.366_025_4;
    const G2: f32 = 0.211_324_87;
    let s = (x + y) * F2;
    let i = (x + s).floor();
    let j = (y + s).floor();
    let t = (i + j) * G2;
    let x0 = x - (i - t);
    let y0 = y - (j - t);
    let (i1, j1) = if x0 > y0 { (1.0, 0.0) } else { (0.0, 1.0) };
    let x1 = x0 - i1 + G2;
    let y1 = y0 - j1 + G2;
    let x2 = x0 - 1.0 + 2.0 * G2;
    let y2 = y0 - 1.0 + 2.0 * G2;

    let ii = i as i32;
    let jj = j as i32;
    let mut n = 0.0f32;
    for (dx, dy, gx, gy) in [(x0, y0, ii, jj), (x1, y1, ii + i1 as i32, jj + j1 as i32), (x2, y2, ii + 1, jj + 1)] {
        let mut t = 0.5 - dx * dx - dy * dy;
        if t > 0.0 {
            let g = gradient(gx, gy, seed);
            t *= t;
            n += t * t * (g[0] * dx + g[1] * dy);
        }
    }
    n * 70.0
}

/// Worley F1 distance normalised to `0..1`.
fn voronoi(x: f32, y: f32, seed: u32) -> f32 {
    let cx = x.floor() as i32;
    let cy = y.floor() as i32;
    let mut nearest = f32::INFINITY;
    for oy in -1..=1 {
        for ox in -1..=1 {
            let cell_x = cx + ox;
            let cell_y = cy + oy;
            let jitter_x = hash_unit(cell_x, cell_y, seed);
            let jitter_y = hash_unit(cell_x, cell_y, seed ^ 0x9e37_79b9);
            let point_x = cell_x as f32 + jitter_x;
            let point_y = cell_y as f32 + jitter_y;
            let dx = point_x - x;
            let dy = point_y - y;
            nearest = nearest.min((dx * dx + dy * dy).sqrt());
        }
    }
    (nearest * 1.5).clamp(0.0, 1.0)
}

fn fade(t: f32) -> f32 {
    t * t * t * (t * (t * 6.0 - 15.0) + 10.0)
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

fn dot_gradient(ix: i32, iy: i32, seed: u32, dx: f32, dy: f32) -> f32 {
    let g = gradient(ix, iy, seed);
    g[0] * dx + g[1] * dy
}

fn gradient(ix: i32, iy: i32, seed: u32) -> [f32; 2] {
    let angle = hash_unit(ix, iy, seed) * TAU;
    let (sin, cos) = angle.sin_cos();
    [cos, sin]
}

/// A stable hash in `0..1` for a lattice point.
fn hash_unit(x: i32, y: i32, seed: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d) ^ (y as u32).wrapping_mul(0x1656_67b1) ^ seed.wrapping_mul(0x9e37_79b9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2c1b_3c6d);
    h ^= h >> 12;
    h = h.wrapping_mul(0x297a_2d39);
    h ^= h >> 15;
    (h as f32) / (u32::MAX as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::{Filter, Generator};

    #[test]
    fn generators_produce_values_in_range() {
        for generator in [Generator::default_perlin(), Generator::default_simplex(), Generator::default_voronoi(), Generator::default_white()] {
            let image = generate(&generator, 32, 32);
            assert_eq!(image.width, 32);
            for pixel in &image.pixels {
                assert!((0.0..=1.0).contains(&pixel[0]), "out of range: {}", pixel[0]);
                assert_eq!(pixel[3], 1.0);
            }
        }
    }

    #[test]
    fn noise_is_deterministic_and_varies() {
        let a = generate(&Generator::default_perlin(), 32, 32);
        let b = generate(&Generator::default_perlin(), 32, 32);
        assert_eq!(a.pixels, b.pixels, "generation must be deterministic");
        let distinct = a.pixels.windows(2).any(|pair| (pair[0][0] - pair[1][0]).abs() > 1e-4);
        assert!(distinct, "noise should vary across the image");
    }

    #[test]
    fn perlin_and_simplex_differ() {
        let perlin = generate(&Generator::default_perlin(), 16, 16);
        let simplex = generate(&Generator::default_simplex(), 16, 16);
        assert_ne!(perlin.pixels, simplex.pixels);
    }

    #[test]
    fn invert_reverses_colour() {
        let mut image = RasterImage::filled(2, 2, [0.25, 0.5, 0.75, 1.0]);
        // Keep alpha; invert rgb.
        image.set_pixel(0, 0, [0.25, 0.5, 0.75, 1.0]);
        let out = apply(&Filter::Invert, &image);
        assert!((out.pixel(0, 0)[0] - 0.75).abs() < 1e-6);
        assert!((out.pixel(0, 0)[2] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn levels_remap_midpoint() {
        let image = RasterImage::filled(1, 1, [0.5, 0.5, 0.5, 1.0]);
        let out = apply(&Filter::default_levels(), &image);
        assert!((out.pixel(0, 0)[0] - 0.5).abs() < 1e-6);
        let boosted = apply(&Filter::Levels { in_black: 0.0, in_white: 1.0, gamma: 2.0, out_black: 0.0, out_white: 1.0 }, &image);
        assert!(boosted.pixel(0, 0)[0] > 0.5);
    }

    #[test]
    fn blur_smooths_a_spike() {
        let mut image = RasterImage::new(9, 9);
        image.set_pixel(4, 4, [1.0, 1.0, 1.0, 1.0]);
        let out = apply(&Filter::Blur { radius: 2 }, &image);
        assert!(out.pixel(4, 4)[0] < 1.0, "spike should dim");
        assert!(out.pixel(5, 4)[0] > 0.0, "blur should spread");
    }

    #[test]
    fn brightness_and_contrast_clamp() {
        let image = RasterImage::filled(1, 1, [0.4, 0.4, 0.4, 1.0]);
        let bright = apply(&Filter::Brightness { amount: 0.5 }, &image);
        assert!((bright.pixel(0, 0)[0] - 0.9).abs() < 1e-6);
        let dark = apply(&Filter::Brightness { amount: -1.0 }, &image);
        assert_eq!(dark.pixel(0, 0)[0], 0.0);
    }
}
