//! SubstanceCraft CPU compositor.
//!
//! Evaluates a texture set's layer stack into per-channel images on the CPU. It is intentionally
//! simple and obviously correct: the GPU compositor's output is validated against it, and it backs
//! headless export and baking of flattened results.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use substancecraft_doc::{
    BlendMode, Channel, Effect, FillSource, LayerKind, LayerStack, Mask, MaskEffect, Material, MeshMapKind, Projection, ProjectionKind, TextureSet,
    planar_uv,
};
use substancecraft_geom::Mesh;
use substancecraft_raster::RasterImage;

/// Per-texel object-space surface attributes, used to evaluate position-based projections
/// (planar/tri-planar) and mesh-map fills.
#[derive(Clone)]
pub struct Surface {
    width: u32,
    height: u32,
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    parts: Vec<u32>,
}

impl Surface {
    /// Build surface attributes by rasterising the mesh's UV triangles at `width × height`.
    pub fn build(mesh: &Mesh, width: u32, height: u32) -> Self {
        let count = (width as usize).saturating_mul(height as usize);
        let mut surface = Self { width, height, positions: vec![[0.0; 3]; count], normals: vec![[0.0; 3]; count], parts: vec![u32::MAX; count] };
        if width == 0 || height == 0 {
            return surface;
        }
        let vertex_parts = substancecraft_geom::mesh_parts(mesh);
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let [i0, i1, i2] = *triangle;
            let (Some(p0), Some(p1), Some(p2)) = (vertex_position(mesh, i0), vertex_position(mesh, i1), vertex_position(mesh, i2)) else {
                continue;
            };
            let (Some(t0), Some(t1), Some(t2)) = (vertex_uv(mesh, i0), vertex_uv(mesh, i1), vertex_uv(mesh, i2)) else { continue };
            let part = vertex_parts.get(i0 as usize).copied().unwrap_or(u32::MAX);
            surface.rasterize(
                p0,
                p1,
                p2,
                t0,
                t1,
                t2,
                vertex_normal(mesh, i0, p0, p1, p2),
                vertex_normal(mesh, i1, p1, p2, p0),
                vertex_normal(mesh, i2, p2, p0, p1),
                part,
            );
        }
        surface
    }

    #[allow(clippy::too_many_arguments)]
    fn rasterize(
        &mut self,
        p0: [f32; 3],
        p1: [f32; 3],
        p2: [f32; 3],
        t0: [f32; 2],
        t1: [f32; 2],
        t2: [f32; 2],
        n0: [f32; 3],
        n1: [f32; 3],
        n2: [f32; 3],
        part: u32,
    ) {
        let (w, h) = (self.width as f32, self.height as f32);
        let (a, b, c) = ([t0[0] * w, t0[1] * h], [t1[0] * w, t1[1] * h], [t2[0] * w, t2[1] * h]);
        let area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
        if area.abs() < 1e-9 {
            return;
        }
        let min_x = a[0].min(b[0]).min(c[0]).floor().max(0.0) as i64;
        let min_y = a[1].min(b[1]).min(c[1]).floor().max(0.0) as i64;
        let max_x = (a[0].max(b[0]).max(c[0]).ceil() as i64).min(self.width as i64 - 1);
        let max_y = (a[1].max(b[1]).max(c[1]).ceil() as i64).min(self.height as i64 - 1);
        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let p = [x as f32 + 0.5, y as f32 + 0.5];
                let w0 = ((b[0] - p[0]) * (c[1] - p[1]) - (c[0] - p[0]) * (b[1] - p[1])) / area;
                let w1 = ((c[0] - p[0]) * (a[1] - p[1]) - (a[0] - p[0]) * (c[1] - p[1])) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                    continue;
                }
                let index = (y as u32) as usize * self.width as usize + x as usize;
                if let (Some(slot), Some(normal)) = (self.positions.get_mut(index), self.normals.get_mut(index)) {
                    *slot = lerp3(p0, p1, p2, w0, w1, w2);
                    *normal = normalize3(lerp3(n0, n1, n2, w0, w1, w2));
                }
                if let Some(slot) = self.parts.get_mut(index) {
                    *slot = part;
                }
            }
        }
    }

    fn get(&self, x: u32, y: u32) -> Option<([f32; 3], [f32; 3])> {
        if x >= self.width || y >= self.height {
            return None;
        }
        let index = y as usize * self.width as usize + x as usize;
        let position = *self.positions.get(index)?;
        let normal = *self.normals.get(index)?;
        (normal != [0.0; 3]).then_some((position, normal))
    }

    fn part_at(&self, x: u32, y: u32) -> u32 {
        if x >= self.width || y >= self.height {
            return u32::MAX;
        }
        self.parts.get(y as usize * self.width as usize + x as usize).copied().unwrap_or(u32::MAX)
    }
}

/// Everything the CPU compositor needs beyond the layer stack itself.
#[derive(Default)]
pub struct EvalContext<'a> {
    /// Per-texel surface attributes for position-based projections.
    pub surface: Option<&'a Surface>,
    /// Baked mesh maps by kind, for mesh-map fills.
    pub mesh_maps: &'a [(MeshMapKind, RasterImage)],
}

fn vertex_position(mesh: &Mesh, index: u32) -> Option<[f32; 3]> {
    mesh.positions.get(index as usize).map(|position| [position.x, position.y, position.z])
}

fn vertex_uv(mesh: &Mesh, index: u32) -> Option<[f32; 2]> {
    mesh.uvs.get(index as usize).map(|uv| [uv.x, uv.y])
}

fn vertex_normal(mesh: &Mesh, index: u32, position: [f32; 3], other1: [f32; 3], other2: [f32; 3]) -> [f32; 3] {
    match mesh.normals.get(index as usize) {
        Some(normal) if normal.length_squared() > 0.0 => [normal.x, normal.y, normal.z],
        _ => face_normal(position, other1, other2),
    }
}

fn face_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let cross = [ab[1] * ac[2] - ab[2] * ac[1], ab[2] * ac[0] - ab[0] * ac[2], ab[0] * ac[1] - ab[1] * ac[0]];
    normalize3(cross)
}

fn lerp3(p0: [f32; 3], p1: [f32; 3], p2: [f32; 3], w0: f32, w1: f32, w2: f32) -> [f32; 3] {
    [p0[0] * w0 + p1[0] * w1 + p2[0] * w2, p0[1] * w0 + p1[1] * w1 + p2[1] * w2, p0[2] * w0 + p1[2] * w1 + p2[2] * w2]
}

fn normalize3(v: [f32; 3]) -> [f32; 3] {
    let length = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if length <= 1e-9 { [0.0; 3] } else { [v[0] / length, v[1] / length, v[2] / length] }
}

/// The default (base) colour of a channel before any layer is applied, derived from the source
/// material for the data channels.
pub fn default_channel_base(channel: Channel, material: &Material) -> [f32; 4] {
    let rgb = match channel {
        Channel::BaseColor => [material.base_color[0], material.base_color[1], material.base_color[2]],
        Channel::Metallic => [material.metallic; 3],
        Channel::Roughness => [material.roughness; 3],
        Channel::Normal => [0.5, 0.5, 1.0],
        Channel::Height => [0.5; 3],
        Channel::AmbientOcclusion => [1.0; 3],
        Channel::Emissive => [0.0; 3],
    };
    [rgb[0], rgb[1], rgb[2], 1.0]
}

/// Whether a channel stores a scalar (replicated) or a colour.
fn channel_is_scalar(channel: Channel) -> bool {
    matches!(channel, Channel::Metallic | Channel::Roughness | Channel::Height | Channel::AmbientOcclusion)
}

/// The constant source colour a fill layer contributes to a channel.
fn fill_source(channel: Channel, color: [f32; 4], value: f32) -> [f32; 3] {
    if channel_is_scalar(channel) { [value, value, value] } else { [color[0], color[1], color[2]] }
}

/// Blend two RGB colours with a blend mode; components are not clamped (the caller clamps on output).
///
/// The separable modes follow the W3C Compositing and Blending specification; the non-separable
/// modes (hue, saturation, colour, luminosity, darker/lighter colour) work on the full RGB triple.
pub fn blend_rgb(mode: BlendMode, base: [f32; 3], src: [f32; 3]) -> [f32; 3] {
    match mode {
        BlendMode::Hue => set_lum(set_sat(src, saturation(base)), luminance(base)),
        BlendMode::Saturation => set_lum(set_sat(base, saturation(src)), luminance(base)),
        BlendMode::Color => set_lum(src, luminance(base)),
        BlendMode::Luminosity => set_lum(base, luminance(src)),
        BlendMode::DarkerColor => {
            if luminance(base) <= luminance(src) {
                base
            } else {
                src
            }
        }
        BlendMode::LighterColor => {
            if luminance(base) >= luminance(src) {
                base
            } else {
                src
            }
        }
        separable => {
            let mut out = [0.0f32; 3];
            for index in 0..3 {
                out[index] = blend_channel(separable, base[index], src[index]);
            }
            out
        }
    }
}

/// Blend one pair of components for the separable modes.
fn blend_channel(mode: BlendMode, b: f32, s: f32) -> f32 {
    match mode {
        BlendMode::Normal => s,
        BlendMode::Multiply => b * s,
        BlendMode::Screen => b + s - b * s,
        BlendMode::Overlay => hard_light(s, b),
        BlendMode::Add => b + s,
        BlendMode::Subtract => b - s,
        BlendMode::Darken => b.min(s),
        BlendMode::Lighten => b.max(s),
        BlendMode::ColorDodge => color_dodge(b, s),
        BlendMode::ColorBurn => color_burn(b, s),
        BlendMode::LinearBurn => b + s - 1.0,
        BlendMode::HardLight => hard_light(b, s),
        BlendMode::SoftLight => soft_light(b, s),
        BlendMode::VividLight => {
            if s <= 0.5 {
                color_burn(b, 2.0 * s)
            } else {
                color_dodge(b, 2.0 * s - 1.0)
            }
        }
        BlendMode::LinearLight => b + 2.0 * s - 1.0,
        BlendMode::PinLight => {
            if s <= 0.5 {
                b.min(2.0 * s)
            } else {
                b.max(2.0 * s - 1.0)
            }
        }
        BlendMode::HardMix => {
            if b + s >= 1.0 {
                1.0
            } else {
                0.0
            }
        }
        BlendMode::Difference => (b - s).abs(),
        BlendMode::Exclusion => b + s - 2.0 * b * s,
        BlendMode::Divide => {
            if s <= 0.0 {
                1.0
            } else {
                b / s
            }
        }
        // Non-separable modes are handled in `blend_rgb` and never reach this function.
        BlendMode::Hue | BlendMode::Saturation | BlendMode::Color | BlendMode::Luminosity | BlendMode::DarkerColor | BlendMode::LighterColor => s,
    }
}

/// `base / (1 - src)`, clamped to `0..=1`.
fn color_dodge(b: f32, s: f32) -> f32 {
    if b <= 0.0 {
        0.0
    } else if s >= 1.0 {
        1.0
    } else {
        (b / (1.0 - s)).min(1.0)
    }
}

/// `1 - (1 - base) / src`, clamped to `0..=1`.
fn color_burn(b: f32, s: f32) -> f32 {
    if b >= 1.0 {
        1.0
    } else if s <= 0.0 {
        0.0
    } else {
        1.0 - ((1.0 - b) / s).min(1.0)
    }
}

/// Overlay driven by the source.
fn hard_light(b: f32, s: f32) -> f32 {
    if s <= 0.5 { 2.0 * b * s } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - s) }
}

/// The W3C soft-light curve.
fn soft_light(b: f32, s: f32) -> f32 {
    if s <= 0.5 { b - (1.0 - 2.0 * s) * b * (1.0 - b) } else { b + (2.0 * s - 1.0) * (soft_light_d(b) - b) }
}

fn soft_light_d(b: f32) -> f32 {
    if b <= 0.25 { ((16.0 * b - 12.0) * b + 4.0) * b } else { b.sqrt() }
}

/// Perceptual luminance of an RGB triple.
fn luminance(c: [f32; 3]) -> f32 {
    0.3 * c[0] + 0.59 * c[1] + 0.11 * c[2]
}

/// Colourfulness (max - min).
fn saturation(c: [f32; 3]) -> f32 {
    c[0].max(c[1]).max(c[2]) - c[0].min(c[1]).min(c[2])
}

/// Re-luminance a colour to `l`, clipping back into gamut.
fn set_lum(c: [f32; 3], l: f32) -> [f32; 3] {
    let d = l - luminance(c);
    clip_color([c[0] + d, c[1] + d, c[2] + d])
}

fn clip_color(c: [f32; 3]) -> [f32; 3] {
    let l = luminance(c);
    let n = c[0].min(c[1]).min(c[2]);
    let x = c[0].max(c[1]).max(c[2]);
    let mut out = c;
    if n < 0.0 && (l - n) != 0.0 {
        let scale = l / (l - n);
        out = [l + (out[0] - l) * scale, l + (out[1] - l) * scale, l + (out[2] - l) * scale];
    }
    if x > 1.0 && (x - l) != 0.0 {
        let scale = (1.0 - l) / (x - l);
        out = [l + (out[0] - l) * scale, l + (out[1] - l) * scale, l + (out[2] - l) * scale];
    }
    out
}

/// Re-saturate a colour to `s` (W3C `SetSat`).
fn set_sat(c: [f32; 3], s: f32) -> [f32; 3] {
    let mut order = [0usize, 1, 2];
    order.sort_by(|&a, &b| c[a].total_cmp(&c[b]));
    let (min_i, mid_i, max_i) = (order[0], order[1], order[2]);
    let (cmin, cmid, cmax) = (c[min_i], c[mid_i], c[max_i]);
    if cmax > cmin {
        let mut out = [0.0f32; 3];
        out[min_i] = 0.0;
        out[mid_i] = (cmid - cmin) * s / (cmax - cmin);
        out[max_i] = s;
        out
    } else {
        [0.0, 0.0, 0.0]
    }
}

/// The per-texel mask value at a pixel index (1.0 when there is no mask).
fn mask_value(mask: Option<&RasterImage>, index: usize) -> f32 {
    mask.and_then(|image| image.pixels.get(index)).map(|pixel| pixel[0].clamp(0.0, 1.0)).unwrap_or(1.0)
}

/// Composite a constant source colour over an image.
fn composite_constant(image: &mut RasterImage, src: [f32; 3], coverage: f32, blend: BlendMode, mask: Option<&RasterImage>) {
    for (index, pixel) in image.pixels.iter_mut().enumerate() {
        let coverage = (coverage * mask_value(mask, index)).clamp(0.0, 1.0);
        if coverage <= 0.0 {
            continue;
        }
        let base = [pixel[0], pixel[1], pixel[2]];
        let blended = blend_rgb(blend, base, src);
        for component in 0..3 {
            pixel[component] = (base[component] + (blended[component] - base[component]) * coverage).clamp(0.0, 1.0);
        }
        pixel[3] = 1.0;
    }
}

/// Composite a painted image over an image, sampling the paint per pixel.
fn composite_paint(image: &mut RasterImage, paint: &RasterImage, coverage: f32, blend: BlendMode, channel: Channel, mask: Option<&RasterImage>) {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return;
    }
    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        for x in 0..width {
            let u = (x as f32 + 0.5) / width as f32;
            let sample = paint.sample_nearest(u, v);
            let index = (y as usize) * (width as usize) + (x as usize);
            // Transparent paint contributes nothing; its alpha and the mask scale the coverage.
            let coverage = (coverage * sample[3] * mask_value(mask, index)).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let src = if channel_is_scalar(channel) { [sample[0]; 3] } else { [sample[0], sample[1], sample[2]] };
            if let Some(pixel) = image.pixels.get_mut(index) {
                let base = [pixel[0], pixel[1], pixel[2]];
                let blended = blend_rgb(blend, base, src);
                for component in 0..3 {
                    pixel[component] = (base[component] + (blended[component] - base[component]) * coverage).clamp(0.0, 1.0);
                }
                pixel[3] = 1.0;
            }
        }
    }
}

/// Mix a filtered image back over the original by coverage (used by filter effects).
fn composite_filtered(image: &mut RasterImage, filtered: &RasterImage, coverage: f32, mask: Option<&RasterImage>) {
    for (index, (pixel, other)) in image.pixels.iter_mut().zip(&filtered.pixels).enumerate() {
        let coverage = (coverage * mask_value(mask, index)).clamp(0.0, 1.0);
        for component in 0..3 {
            pixel[component] = (pixel[component] + (other[component] - pixel[component]) * coverage).clamp(0.0, 1.0);
        }
    }
}

/// Composite a whole image (used for groups) over an image.
fn composite_image(image: &mut RasterImage, group: &RasterImage, coverage: f32, blend: BlendMode, mask: Option<&RasterImage>) {
    for (index, pixel) in image.pixels.iter_mut().enumerate() {
        let Some(over) = group.pixels.get(index) else { continue };
        let coverage = (coverage * mask_value(mask, index)).clamp(0.0, 1.0);
        let base = [pixel[0], pixel[1], pixel[2]];
        let top = [over[0], over[1], over[2]];
        let blended = blend_rgb(blend, base, top);
        for component in 0..3 {
            pixel[component] = (base[component] + (blended[component] - base[component]) * coverage).clamp(0.0, 1.0);
        }
        pixel[3] = 1.0;
    }
}

/// Evaluate a layer's dynamic mask into a per-texel coverage image.
fn evaluate_mask(mask: &Mask, width: u32, height: u32, context: &EvalContext<'_>) -> Option<RasterImage> {
    let Mask::Stack { effects } = mask else { return None };
    let mut image = RasterImage::filled(width, height, [0.0, 0.0, 0.0, 1.0]);
    for effect in effects {
        match effect {
            MaskEffect::Fill { value } => {
                let value = value.clamp(0.0, 1.0);
                for pixel in &mut image.pixels {
                    pixel[0] = value;
                    pixel[1] = value;
                    pixel[2] = value;
                }
            }
            MaskEffect::Generator(generator) => {
                let generated = substancecraft_material::generate(generator, width, height);
                for (pixel, source) in image.pixels.iter_mut().zip(&generated.pixels) {
                    pixel[0] = source[0];
                    pixel[1] = source[0];
                    pixel[2] = source[0];
                }
            }
            MaskEffect::Filter(filter) => {
                image = substancecraft_material::apply(filter, &image);
            }
            MaskEffect::Image { image: source, projection, invert } => {
                for y in 0..height {
                    for x in 0..width {
                        let uv = [(x as f32 + 0.5) / width as f32, (y as f32 + 0.5) / height as f32];
                        let sample = sample_source(source, *projection, uv, x, y, context);
                        // A logo stamps white where it is opaque; otherwise use brightness.
                        let mut value = if sample[3] < 1.0 { sample[3] } else { sample[0] };
                        if *invert {
                            value = 1.0 - value;
                        }
                        let index = y as usize * width as usize + x as usize;
                        if let Some(pixel) = image.pixels.get_mut(index) {
                            pixel[0] = value;
                            pixel[1] = value;
                            pixel[2] = value;
                        }
                    }
                }
            }
            MaskEffect::MeshMap { kind, invert } => {
                if let Some((_, map)) = context.mesh_maps.iter().find(|(map_kind, _)| map_kind == kind) {
                    for (index, pixel) in image.pixels.iter_mut().enumerate() {
                        let mut value = map.pixels.get(index).map(|source| source[0]).unwrap_or(0.0).clamp(0.0, 1.0);
                        if *invert {
                            value = 1.0 - value;
                        }
                        pixel[0] = value;
                        pixel[1] = value;
                        pixel[2] = value;
                    }
                }
            }
        }
    }
    Some(image)
}

/// A per-texel coverage image that is 1 where the texel's part is allowed and 0 elsewhere.
fn part_coverage_image(allowed: &[u32], surface: Option<&Surface>, width: u32, height: u32) -> RasterImage {
    let mut image = RasterImage::filled(width, height, [1.0, 1.0, 1.0, 1.0]);
    let Some(surface) = surface else { return image };
    for y in 0..height {
        for x in 0..width {
            let value = if allowed.contains(&surface.part_at(x, y)) { 1.0 } else { 0.0 };
            let index = y as usize * width as usize + x as usize;
            if let Some(pixel) = image.pixels.get_mut(index) {
                pixel[0] = value;
                pixel[1] = value;
                pixel[2] = value;
            }
        }
    }
    image
}

/// Multiply two optional coverage images into one (when both are present, the product).
fn combine_coverage(a: Option<RasterImage>, b: Option<RasterImage>) -> Option<RasterImage> {
    match (a, b) {
        (None, None) => None,
        (Some(image), None) | (None, Some(image)) => Some(image),
        (Some(mut first), Some(second)) => {
            for (pixel, other) in first.pixels.iter_mut().zip(&second.pixels) {
                pixel[0] *= other[0];
                pixel[1] *= other[1];
                pixel[2] *= other[2];
            }
            Some(first)
        }
    }
}

/// Composite the layers of a stack onto `image` in place (bottom to top).
fn evaluate_into(stack: &LayerStack, channel: Channel, image: &mut RasterImage, context: &EvalContext<'_>) {
    for layer in &stack.layers {
        if !layer.visible {
            continue;
        }
        let coverage = layer.coverage_for(channel);
        if coverage <= 0.0 {
            continue;
        }
        let dynamic_mask = layer.mask.as_ref().and_then(|mask| evaluate_mask(mask, image.width, image.height, context));
        let part_mask = layer.parts.as_ref().map(|allowed| part_coverage_image(allowed, context.surface, image.width, image.height));
        let coverage_image = combine_coverage(dynamic_mask, part_mask);
        let mask = coverage_image.as_ref();
        match &layer.kind {
            LayerKind::Fill { color, value, source, projection, strength } => {
                evaluate_fill(image, channel, *color, *value, source, *projection, *strength, coverage, layer.blend, mask, context);
            }
            LayerKind::Paint { image: Some(paint) } => {
                composite_paint(image, paint, coverage, layer.blend, channel, mask);
            }
            LayerKind::Paint { image: None } => {}
            LayerKind::Effect { effect } => match effect {
                Effect::Generator(generator) => {
                    let generated = substancecraft_material::generate(generator, image.width, image.height);
                    composite_paint(image, &generated, coverage, layer.blend, channel, mask);
                }
                Effect::Filter(filter) => {
                    let filtered = substancecraft_material::apply(filter, image);
                    composite_filtered(image, &filtered, coverage, mask);
                }
            },
            LayerKind::Group { children } => {
                let mut group = image.clone();
                evaluate_into(children, channel, &mut group, context);
                composite_image(image, &group, coverage, layer.blend, mask);
            }
        }
    }
}

/// Evaluate one fill layer into `image`.
#[allow(clippy::too_many_arguments)]
fn evaluate_fill(
    image: &mut RasterImage,
    channel: Channel,
    color: [f32; 4],
    value: f32,
    source: &FillSource,
    projection: Projection,
    strength: f32,
    coverage: f32,
    blend: BlendMode,
    mask: Option<&RasterImage>,
    context: &EvalContext<'_>,
) {
    match source {
        FillSource::Uniform => {
            let src = apply_strength(channel, fill_source(channel, color, value), strength);
            composite_constant(image, src, coverage, blend, mask);
        }
        FillSource::Image { image: source_image } => {
            composite_projected(image, source_image, coverage, blend, channel, projection, strength, mask, context);
        }
        FillSource::Procedural { generator } => {
            let generated = substancecraft_material::generate(generator, image.width, image.height);
            composite_projected(image, &generated, coverage, blend, channel, projection, strength, mask, context);
        }
        FillSource::MeshMap { kind } => {
            if let Some((_, map)) = context.mesh_maps.iter().find(|(map_kind, _)| map_kind == kind) {
                composite_projected(image, map, coverage, blend, channel, Projection::default(), strength, mask, context);
            }
        }
    }
}

/// Composite a projected source image over `image`.
#[allow(clippy::too_many_arguments)]
fn composite_projected(
    image: &mut RasterImage,
    source: &RasterImage,
    coverage: f32,
    blend: BlendMode,
    channel: Channel,
    projection: Projection,
    strength: f32,
    mask: Option<&RasterImage>,
    context: &EvalContext<'_>,
) {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return;
    }
    for y in 0..height {
        for x in 0..width {
            let uv = [(x as f32 + 0.5) / width as f32, (y as f32 + 0.5) / height as f32];
            let sample = sample_source(source, projection, uv, x, y, context);
            let index = (y as usize) * (width as usize) + (x as usize);
            let coverage = (coverage * sample[3] * mask_value(mask, index)).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let raw = if channel_is_scalar(channel) { [sample[0]; 3] } else { [sample[0], sample[1], sample[2]] };
            let src = apply_strength(channel, raw, strength);
            if let Some(pixel) = image.pixels.get_mut(index) {
                let base = [pixel[0], pixel[1], pixel[2]];
                let blended = blend_rgb(blend, base, src);
                for component in 0..3 {
                    pixel[component] = (base[component] + (blended[component] - base[component]) * coverage).clamp(0.0, 1.0);
                }
                pixel[3] = 1.0;
            }
        }
    }
}

/// Sample a source image through a projection.
fn sample_source(source: &RasterImage, projection: Projection, uv: [f32; 2], x: u32, y: u32, context: &EvalContext<'_>) -> [f32; 4] {
    match projection.kind {
        ProjectionKind::Uv => {
            let st = projection.transform_uv(uv);
            source.sample_nearest(st[0], st[1])
        }
        ProjectionKind::Planar => match context.surface.and_then(|surface| surface.get(x, y)) {
            Some((position, _)) => {
                let st = projection.transform_uv(planar_uv(projection.axis, position));
                source.sample_nearest(st[0], st[1])
            }
            None => {
                let st = projection.transform_uv(uv);
                source.sample_nearest(st[0], st[1])
            }
        },
        ProjectionKind::TriPlanar => match context.surface.and_then(|surface| surface.get(x, y)) {
            Some((position, normal)) => sample_triplanar(source, projection, position, normal),
            None => {
                let st = projection.transform_uv(uv);
                source.sample_nearest(st[0], st[1])
            }
        },
    }
}

/// Blend the three axis projections of a tri-planar sample by the surface normal.
fn sample_triplanar(source: &RasterImage, projection: Projection, position: [f32; 3], normal: [f32; 3]) -> [f32; 4] {
    let mut weights = [normal[0].abs(), normal[1].abs(), normal[2].abs()];
    let sum = weights[0] + weights[1] + weights[2];
    if sum <= 1e-6 {
        let st = projection.transform_uv(planar_uv(1, position));
        return source.sample_nearest(st[0], st[1]);
    }
    for weight in &mut weights {
        *weight /= sum;
    }
    let mut out = [0.0f32; 4];
    for (axis, weight) in weights.iter().enumerate() {
        let st = projection.transform_uv(planar_uv(axis as u8, position));
        let sample = source.sample_nearest(st[0], st[1]);
        for (slot, value) in out.iter_mut().zip(sample) {
            *slot += value * weight;
        }
    }
    out
}

/// Whether any fill or mask in the stack needs per-texel surface attributes (planar/tri-planar).
pub fn stack_needs_surface(stack: &LayerStack) -> bool {
    stack.layers.iter().any(|layer| {
        let fill = matches!(&layer.kind, LayerKind::Fill { source, projection, .. } if !matches!(source, FillSource::Uniform) && projection.kind != ProjectionKind::Uv);
        let mask = matches!(&layer.mask, Some(Mask::Stack { effects }) if effects.iter().any(|effect| matches!(effect, MaskEffect::Image { projection, .. } if projection.kind != ProjectionKind::Uv)));
        let children = matches!(&layer.kind, LayerKind::Group { children } if stack_needs_surface(children));
        fill || mask || children || layer.parts.is_some()
    })
}

/// Apply a fill's strength multiplier for a channel.
fn apply_strength(channel: Channel, src: [f32; 3], strength: f32) -> [f32; 3] {
    let strength = strength.max(0.0);
    match channel {
        Channel::Normal => {
            [(0.5 + (src[0] - 0.5) * strength).clamp(0.0, 1.0), (0.5 + (src[1] - 0.5) * strength).clamp(0.0, 1.0), src[2].clamp(0.0, 1.0)]
        }
        Channel::Height => [0.5 + (src[0] - 0.5) * strength, 0.5 + (src[1] - 0.5) * strength, 0.5 + (src[2] - 0.5) * strength],
        _ => [src[0] * strength, src[1] * strength, src[2] * strength],
    }
}

/// Evaluate one channel of a stack into an image of `width × height` with an empty context.
pub fn evaluate_channel(stack: &LayerStack, channel: Channel, base: [f32; 4], width: u32, height: u32) -> RasterImage {
    evaluate_channel_ctx(stack, channel, base, width, height, &EvalContext::default())
}

/// Evaluate one channel of a stack into an image, using `context` for projections and mesh maps.
pub fn evaluate_channel_ctx(stack: &LayerStack, channel: Channel, base: [f32; 4], width: u32, height: u32, context: &EvalContext<'_>) -> RasterImage {
    let mut image = RasterImage::filled(width, height, base);
    evaluate_into(stack, channel, &mut image, context);
    image
}

/// Evaluate every channel of a texture set into images, with an empty context.
pub fn evaluate_set(set: &TextureSet, material: &Material, width: u32, height: u32) -> Vec<(Channel, RasterImage)> {
    evaluate_set_ctx(set, material, width, height, &EvalContext::default())
}

/// Evaluate every channel of a texture set into images, using `context`.
pub fn evaluate_set_ctx(set: &TextureSet, material: &Material, width: u32, height: u32, context: &EvalContext<'_>) -> Vec<(Channel, RasterImage)> {
    set.channels
        .iter()
        .map(|&channel| (channel, evaluate_channel_ctx(&set.layers, channel, default_channel_base(channel, material), width, height, context)))
        .collect()
}

/// Evaluate just the base-colour channel of a set (a convenience for previews).
pub fn evaluate_base_color(set: &TextureSet, material: &Material, width: u32, height: u32) -> RasterImage {
    evaluate_base_color_ctx(set, material, width, height, &EvalContext::default())
}

/// Evaluate just the base-colour channel of a set, using `context`.
pub fn evaluate_base_color_ctx(set: &TextureSet, material: &Material, width: u32, height: u32, context: &EvalContext<'_>) -> RasterImage {
    evaluate_channel_ctx(&set.layers, Channel::BaseColor, default_channel_base(Channel::BaseColor, material), width, height, context)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::{Layer, Mask};
    use substancecraft_raster::RasterImage;

    fn base() -> [f32; 4] {
        [0.0, 0.0, 0.0, 1.0]
    }

    #[test]
    fn empty_stack_is_the_base() {
        let image = evaluate_channel(&LayerStack::new(), Channel::BaseColor, base(), 2, 2);
        assert_eq!(image.pixel(0, 0), base());
    }

    #[test]
    fn a_fill_replaces_the_base_at_full_coverage() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("fill", [1.0, 0.0, 0.0, 1.0], 0.0));
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert!((image.pixel(1, 1)[0] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn opacity_blends_toward_the_base() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("fill", [1.0, 1.0, 1.0, 1.0], 0.0);
        layer.opacity = 0.5;
        stack.push(layer);
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert!((image.pixel(0, 0)[0] - 0.5).abs() < 1e-6);
    }

    #[test]
    fn mask_scales_coverage() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("fill", [1.0, 1.0, 1.0, 1.0], 0.0);
        layer.mask = Some(Mask::Constant(0.25));
        stack.push(layer);
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert!((image.pixel(0, 0)[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn invisible_layers_do_nothing() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("fill", [1.0, 1.0, 1.0, 1.0], 0.0);
        layer.visible = false;
        stack.push(layer);
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert_eq!(image.pixel(0, 0), base());
    }

    #[test]
    fn multiply_darkens() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("fill", [0.5, 0.5, 0.5, 1.0], 0.0);
        layer.blend = BlendMode::Multiply;
        stack.push(layer);
        let image = evaluate_channel(&stack, Channel::BaseColor, [0.8, 0.8, 0.8, 1.0], 1, 1);
        assert!((image.pixel(0, 0)[0] - 0.4).abs() < 1e-6);
    }

    #[test]
    fn expanded_separable_modes() {
        assert!((blend_rgb(BlendMode::ColorDodge, [0.25; 3], [0.5; 3])[0] - 0.5).abs() < 1e-6);
        assert!((blend_rgb(BlendMode::ColorBurn, [0.75; 3], [0.5; 3])[0] - 0.5).abs() < 1e-6);
        assert!((blend_rgb(BlendMode::Difference, [0.8; 3], [0.3; 3])[0] - 0.5).abs() < 1e-6);
        assert!((blend_rgb(BlendMode::Exclusion, [0.5; 3], [0.5; 3])[0] - 0.5).abs() < 1e-6);
        // A mid-grey source is neutral for soft light.
        assert!((blend_rgb(BlendMode::SoftLight, [0.4; 3], [0.5; 3])[0] - 0.4).abs() < 1e-6);
        // Hard mix posterises to 0 or 1.
        assert_eq!(blend_rgb(BlendMode::HardMix, [0.2; 3], [0.2; 3])[0], 0.0);
        assert_eq!(blend_rgb(BlendMode::HardMix, [0.8; 3], [0.8; 3])[0], 1.0);
    }

    #[test]
    fn non_separable_modes_keep_the_expected_luminosity() {
        let base = [0.2, 0.4, 0.6];
        let src = [0.9, 0.1, 0.3];
        assert!((luminance(blend_rgb(BlendMode::Color, base, src)) - luminance(base)).abs() < 1e-4);
        assert!((luminance(blend_rgb(BlendMode::Luminosity, base, src)) - luminance(src)).abs() < 1e-4);
        // Lighter colour picks the brighter of the two.
        assert_eq!(blend_rgb(BlendMode::LighterColor, base, src), base);
    }

    /// Build a 1-row surface whose position x follows the texel gradient and normal is +Z.
    fn flat_surface(width: u32) -> Surface {
        let mut positions = vec![[0.0f32; 3]; width as usize];
        for (index, position) in positions.iter_mut().enumerate() {
            position[0] = (index as f32 + 0.25) / width as f32;
        }
        Surface { width, height: 1, positions, normals: vec![[0.0, 0.0, 1.0]; width as usize], parts: vec![u32::MAX; width as usize] }
    }

    fn left_right_image() -> RasterImage {
        let mut image = RasterImage::filled(2, 2, [0.0, 0.0, 0.0, 1.0]);
        image.set_pixel(0, 0, [1.0, 1.0, 1.0, 1.0]);
        image.set_pixel(0, 1, [1.0, 1.0, 1.0, 1.0]);
        image
    }

    #[test]
    fn image_fill_samples_the_source_by_uv() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill_with_source("img", [0.0; 4], 0.0, FillSource::Image { image: left_right_image() }, Projection::default()));
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert!((image.pixel(0, 0)[0] - 1.0).abs() < 1e-6, "left texel should sample the bright column");
        assert!((image.pixel(1, 0)[0] - 0.0).abs() < 1e-6, "right texel should sample the dark column");
    }

    #[test]
    fn fill_strength_scales_scalar_channels() {
        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("rough", [0.0; 4], 0.5);
        if let LayerKind::Fill { strength, .. } = &mut layer.kind {
            *strength = 0.5;
        }
        stack.push(layer);
        let image = evaluate_channel(&stack, Channel::Roughness, [0.0; 4], 1, 1);
        assert!((image.pixel(0, 0)[0] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn mesh_map_fill_samples_the_baked_map() {
        let maps = vec![(MeshMapKind::AmbientOcclusion, RasterImage::filled(2, 2, [0.2, 0.2, 0.2, 1.0]))];
        let context = EvalContext { surface: None, mesh_maps: &maps };
        let mut stack = LayerStack::new();
        stack.push(Layer::fill_with_source("ao", [1.0; 4], 0.0, FillSource::MeshMap { kind: MeshMapKind::AmbientOcclusion }, Projection::default()));
        let image = evaluate_channel_ctx(&stack, Channel::AmbientOcclusion, [1.0; 4], 2, 2, &context);
        assert!((image.pixel(0, 0)[0] - 0.2).abs() < 1e-6);
    }

    #[test]
    fn planar_and_triplanar_projections_read_the_surface() {
        let surface = flat_surface(4);
        let maps: Vec<(MeshMapKind, RasterImage)> = Vec::new();
        let context = EvalContext { surface: Some(&surface), mesh_maps: &maps };
        for kind in [ProjectionKind::Planar, ProjectionKind::TriPlanar] {
            let projection = Projection { kind, ..Projection::default() };
            let mut stack = LayerStack::new();
            stack.push(Layer::fill_with_source("p", [0.0; 4], 0.0, FillSource::Image { image: left_right_image() }, projection));
            let image = evaluate_channel_ctx(&stack, Channel::BaseColor, base(), 4, 1, &context);
            // Position starts near u = 0.0625 → the bright left column; it drifts right across the row.
            assert!(image.pixel(0, 0)[0] > 0.5, "{kind:?} should sample the bright side at the left");
        }
    }

    #[test]
    fn mask_effect_stack_scales_per_texel_coverage() {
        // An AO map that is black on the left column and white on the right.
        let mut map = RasterImage::filled(2, 2, [1.0, 1.0, 1.0, 1.0]);
        map.set_pixel(0, 0, [0.0, 0.0, 0.0, 1.0]);
        map.set_pixel(0, 1, [0.0, 0.0, 0.0, 1.0]);
        let maps = vec![(MeshMapKind::AmbientOcclusion, map)];
        let context = EvalContext { surface: None, mesh_maps: &maps };

        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("white", [1.0; 4], 1.0);
        layer.mask = Some(Mask::Stack { effects: vec![MaskEffect::MeshMap { kind: MeshMapKind::AmbientOcclusion, invert: false }] });
        stack.push(layer);

        let image = evaluate_channel_ctx(&stack, Channel::BaseColor, [0.0, 0.0, 0.0, 1.0], 2, 2, &context);
        assert!((image.pixel(0, 0)[0] - 0.0).abs() < 1e-6, "masked-out texel should keep the black base");
        assert!((image.pixel(1, 0)[0] - 1.0).abs() < 1e-6, "unmasked texel should be white");
    }

    #[test]
    fn layer_parts_restrict_coverage_per_texel() {
        use substancecraft_geom::glam::{Vec2, Vec3, Vec4};

        // Two triangles far apart in object space (two parts) with disjoint UV islands.
        let mesh = Mesh {
            positions: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(1.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
                Vec3::new(10.0, 0.0, 0.0),
                Vec3::new(11.0, 0.0, 0.0),
                Vec3::new(10.0, 1.0, 0.0),
            ],
            normals: vec![Vec3::Z; 6],
            uvs: vec![Vec2::new(0.0, 0.0), Vec2::new(0.5, 0.0), Vec2::new(0.0, 0.5), Vec2::new(0.5, 0.5), Vec2::new(1.0, 0.5), Vec2::new(0.5, 1.0)],
            tangents: vec![Vec4::new(1.0, 0.0, 0.0, 1.0); 6],
            indices: vec![0, 1, 2, 3, 4, 5],
            submeshes: Vec::new(),
        };
        let surface = Surface::build(&mesh, 8, 8);
        let maps: Vec<(MeshMapKind, RasterImage)> = Vec::new();
        let context = EvalContext { surface: Some(&surface), mesh_maps: &maps };

        let mut stack = LayerStack::new();
        let mut layer = Layer::fill("white", [1.0; 4], 1.0);
        layer.parts = Some(vec![0]);
        stack.push(layer);

        let image = evaluate_channel_ctx(&stack, Channel::BaseColor, [0.0, 0.0, 0.0, 1.0], 8, 8, &context);
        assert!(image.pixel(1, 1)[0] > 0.5, "a texel on part 0 should be filled");
        assert!(image.pixel(4, 4)[0] < 0.5, "a texel on part 1 should stay black");
    }

    #[test]
    fn groups_composite_their_children() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("bottom", [0.2, 0.2, 0.2, 1.0], 0.0));
        let mut group = Layer::group("grp");
        if let LayerKind::Group { children } = &mut group.kind {
            children.push(Layer::fill("inner", [1.0, 1.0, 1.0, 1.0], 0.0));
        }
        group.opacity = 0.5;
        stack.push(group);
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 1, 1);
        // bottom fill -> 0.2 ; group -> mix(0.2, 1.0, 0.5) = 0.6
        assert!((image.pixel(0, 0)[0] - 0.6).abs() < 1e-6);
    }

    #[test]
    fn scalar_channels_use_the_fill_value() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("rough", [0.0, 0.0, 0.0, 1.0], 0.3));
        let image = evaluate_channel(&stack, Channel::Roughness, [0.0; 4], 1, 1);
        assert!((image.pixel(0, 0)[0] - 0.3).abs() < 1e-6);
    }

    #[test]
    fn transparent_paint_leaves_the_base_untouched() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("base", [0.2, 0.4, 0.8, 1.0], 0.0));
        let mut paint = Layer::paint("paint");
        if let LayerKind::Paint { image } = &mut paint.kind {
            *image = Some(RasterImage::new(2, 2)); // fully transparent
        }
        stack.push(paint);
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert!((image.pixel(0, 0)[0] - 0.2).abs() < 1e-6, "transparent paint blacked out the base");
    }

    #[test]
    fn generator_effect_fills_the_channel() {
        let mut stack = LayerStack::new();
        stack.push(Layer::effect("noise", substancecraft_doc::Effect::Generator(substancecraft_doc::Generator::default_white())));
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 8, 8);
        // White noise fills the channel with varying grayscale, not the black base.
        let varied = image.pixels.iter().any(|pixel| pixel[0] > 0.01);
        assert!(varied, "generator effect did not fill the channel");
    }

    #[test]
    fn filter_effect_transforms_the_layers_below() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("base", [0.2, 0.4, 0.8, 1.0], 0.0));
        stack.push(Layer::effect("invert", substancecraft_doc::Effect::Filter(substancecraft_doc::Filter::Invert)));
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 2, 2);
        assert!((image.pixel(0, 0)[0] - 0.8).abs() < 1e-6, "invert filter did not apply");
    }

    #[test]
    fn filter_effect_respects_opacity() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("base", [0.2, 0.2, 0.2, 1.0], 0.0));
        let mut effect = Layer::effect("invert", substancecraft_doc::Effect::Filter(substancecraft_doc::Filter::Invert));
        effect.opacity = 0.5;
        stack.push(effect);
        let image = evaluate_channel(&stack, Channel::BaseColor, base(), 1, 1);
        // mix(0.2, 0.8, 0.5) = 0.5
        assert!((image.pixel(0, 0)[0] - 0.5).abs() < 1e-6);
    }
}
