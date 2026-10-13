//! SubstanceCraft CPU compositor.
//!
//! Evaluates a texture set's layer stack into per-channel images on the CPU. It is intentionally
//! simple and obviously correct: the GPU compositor's output is validated against it, and it backs
//! headless export and baking of flattened results.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use substancecraft_doc::{BlendMode, Channel, LayerKind, LayerStack, Material, TextureSet};
use substancecraft_raster::RasterImage;

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
pub fn blend_rgb(mode: BlendMode, base: [f32; 3], src: [f32; 3]) -> [f32; 3] {
    let mut out = [0.0f32; 3];
    for index in 0..3 {
        let b = base[index];
        let s = src[index];
        out[index] = match mode {
            BlendMode::Normal => s,
            BlendMode::Multiply => b * s,
            BlendMode::Screen => 1.0 - (1.0 - b) * (1.0 - s),
            BlendMode::Overlay => {
                if b < 0.5 {
                    2.0 * b * s
                } else {
                    1.0 - 2.0 * (1.0 - b) * (1.0 - s)
                }
            }
            BlendMode::Add => b + s,
            BlendMode::Subtract => b - s,
            BlendMode::Darken => b.min(s),
            BlendMode::Lighten => b.max(s),
        };
    }
    out
}

/// Composite a constant source colour over an image.
fn composite_constant(image: &mut RasterImage, src: [f32; 3], coverage: f32, blend: BlendMode) {
    for pixel in &mut image.pixels {
        let base = [pixel[0], pixel[1], pixel[2]];
        let blended = blend_rgb(blend, base, src);
        for index in 0..3 {
            pixel[index] = (base[index] + (blended[index] - base[index]) * coverage).clamp(0.0, 1.0);
        }
        pixel[3] = 1.0;
    }
}

/// Composite a painted image over an image, sampling the paint per pixel.
fn composite_paint(image: &mut RasterImage, paint: &RasterImage, coverage: f32, blend: BlendMode, channel: Channel) {
    let (width, height) = (image.width, image.height);
    if width == 0 || height == 0 {
        return;
    }
    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        for x in 0..width {
            let u = (x as f32 + 0.5) / width as f32;
            let sample = paint.sample_nearest(u, v);
            // Transparent paint contributes nothing; the sample alpha scales the coverage.
            let coverage = (coverage * sample[3]).clamp(0.0, 1.0);
            if coverage <= 0.0 {
                continue;
            }
            let src = if channel_is_scalar(channel) { [sample[0]; 3] } else { [sample[0], sample[1], sample[2]] };
            let index = (y as usize) * (width as usize) + (x as usize);
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

/// Composite a whole image (used for groups) over an image.
fn composite_image(image: &mut RasterImage, group: &RasterImage, coverage: f32, blend: BlendMode) {
    for (index, pixel) in image.pixels.iter_mut().enumerate() {
        let Some(over) = group.pixels.get(index) else { continue };
        let base = [pixel[0], pixel[1], pixel[2]];
        let top = [over[0], over[1], over[2]];
        let blended = blend_rgb(blend, base, top);
        for channel in 0..3 {
            pixel[channel] = (base[channel] + (blended[channel] - base[channel]) * coverage).clamp(0.0, 1.0);
        }
        pixel[3] = 1.0;
    }
}

/// Composite the layers of a stack onto `image` in place (bottom to top).
fn evaluate_into(stack: &LayerStack, channel: Channel, image: &mut RasterImage) {
    for layer in &stack.layers {
        if !layer.visible {
            continue;
        }
        let coverage = layer.coverage();
        if coverage <= 0.0 {
            continue;
        }
        match &layer.kind {
            LayerKind::Fill { color, value } => {
                composite_constant(image, fill_source(channel, *color, *value), coverage, layer.blend);
            }
            LayerKind::Paint { image: Some(paint) } => {
                composite_paint(image, paint, coverage, layer.blend, channel);
            }
            LayerKind::Paint { image: None } | LayerKind::Effect => {}
            LayerKind::Group { children } => {
                let mut group = image.clone();
                evaluate_into(children, channel, &mut group);
                composite_image(image, &group, coverage, layer.blend);
            }
        }
    }
}

/// Evaluate one channel of a stack into an image of `width × height`.
pub fn evaluate_channel(stack: &LayerStack, channel: Channel, base: [f32; 4], width: u32, height: u32) -> RasterImage {
    let mut image = RasterImage::filled(width, height, base);
    evaluate_into(stack, channel, &mut image);
    image
}

/// Evaluate every channel of a texture set into images.
pub fn evaluate_set(set: &TextureSet, material: &Material, width: u32, height: u32) -> Vec<(Channel, RasterImage)> {
    set.channels
        .iter()
        .map(|&channel| (channel, evaluate_channel(&set.layers, channel, default_channel_base(channel, material), width, height)))
        .collect()
}

/// Evaluate just the base-colour channel of a set (a convenience for previews).
pub fn evaluate_base_color(set: &TextureSet, material: &Material, width: u32, height: u32) -> RasterImage {
    evaluate_channel(&set.layers, Channel::BaseColor, default_channel_base(Channel::BaseColor, material), width, height)
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
}
