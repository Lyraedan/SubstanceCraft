//! SubstanceCraft GPU compositor.
//!
//! Evaluates a texture set's layer stack on the GPU with wgpu. The CPU compositor
//! (`substancecraft-compose`) is the oracle: the GPU output is validated against it. For now the
//! GPU path supports flat stacks of fill layers with blend modes, opacity and masks; stacks that
//! contain groups, effects or painted images return [`GpuError::Unsupported`] so the caller can
//! fall back to the CPU compositor.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use bytemuck::{Pod, Zeroable};
use substancecraft_doc::{Channel, FillSource, LayerKind, LayerStack, Mask};
use substancecraft_raster::RasterImage;
use thiserror::Error;

/// Errors from GPU compositing. All are recoverable.
#[derive(Debug, Error)]
pub enum GpuError {
    /// No GPU adapter was available.
    #[error("no usable GPU adapter was found")]
    NoAdapter,
    /// The GPU device could not be created.
    #[error("could not create the GPU device: {0}")]
    Device(String),
    /// Reading the result back failed.
    #[error("compositor readback failed: {0}")]
    Readback(String),
    /// The stack uses a feature the GPU path does not implement yet; use the CPU compositor.
    #[error("the GPU compositor does not support this stack: {0}")]
    Unsupported(String),
}

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Params {
    src: [f32; 4],
    // x = coverage, y = blend mode, z = 1 if the channel is scalar
    control: [f32; 4],
}

/// Create a headless device and queue for GPU compositing and tests.
pub fn headless_device() -> Result<(wgpu::Device, wgpu::Queue), GpuError> {
    let instance = wgpu::Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|_| GpuError::NoAdapter)?;
    pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|err| GpuError::Device(err.to_string()))
}

/// The blend-mode wire value used by the shader.
fn blend_code(mode: substancecraft_doc::BlendMode) -> f32 {
    use substancecraft_doc::BlendMode::*;
    match mode {
        Normal => 0.0,
        Multiply => 1.0,
        Screen => 2.0,
        Overlay => 3.0,
        Add => 4.0,
        Subtract => 5.0,
        Darken => 6.0,
        Lighten => 7.0,
        ColorDodge => 8.0,
        ColorBurn => 9.0,
        LinearBurn => 10.0,
        HardLight => 11.0,
        SoftLight => 12.0,
        VividLight => 13.0,
        LinearLight => 14.0,
        PinLight => 15.0,
        HardMix => 16.0,
        Difference => 17.0,
        Exclusion => 18.0,
        Divide => 19.0,
        Hue => 20.0,
        Saturation => 21.0,
        Color => 22.0,
        Luminosity => 23.0,
        DarkerColor => 24.0,
        LighterColor => 25.0,
    }
}

fn channel_is_scalar(channel: Channel) -> bool {
    matches!(channel, Channel::Metallic | Channel::Roughness | Channel::Height | Channel::AmbientOcclusion)
}

/// Plan the fill-layer passes for a stack, or report the first unsupported feature.
fn plan(stack: &LayerStack, channel: Channel) -> Result<Vec<Params>, GpuError> {
    let mut passes: Vec<Params> = Vec::new();
    for layer in &stack.layers {
        if !layer.visible {
            continue;
        }
        if layer.mask.as_ref().is_some_and(Mask::is_dynamic) {
            return Err(GpuError::Unsupported("mask effect stacks".to_string()));
        }
        if layer.parts.is_some() {
            return Err(GpuError::Unsupported("layer parts".to_string()));
        }
        let coverage = layer.coverage_for(channel);
        if coverage <= 0.0 {
            continue;
        }
        match &layer.kind {
            LayerKind::Fill { color, value, source, strength, .. } => {
                if !matches!(source, FillSource::Uniform) {
                    return Err(GpuError::Unsupported("image/procedural/mesh-map fill sources".to_string()));
                }
                if (*strength - 1.0).abs() > 1e-6 {
                    return Err(GpuError::Unsupported("fill strength".to_string()));
                }
                let scalar = channel_is_scalar(channel);
                let src = if scalar { [*value, *value, *value, 1.0] } else { [color[0], color[1], color[2], 1.0] };
                passes.push(Params { src, control: [coverage, blend_code(layer.blend), if scalar { 1.0 } else { 0.0 }, 0.0] });
            }
            LayerKind::Paint { image: None } => {}
            LayerKind::Paint { .. } => return Err(GpuError::Unsupported("painted layers".to_string())),
            LayerKind::Group { .. } => return Err(GpuError::Unsupported("groups".to_string())),
            LayerKind::Effect { .. } => return Err(GpuError::Unsupported("effects".to_string())),
        }
    }
    Ok(passes)
}

/// Composite one channel of a flat fill-layer stack on the GPU.
pub fn composite_channel(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    stack: &LayerStack,
    channel: Channel,
    base: [f32; 4],
    width: u32,
    height: u32,
) -> Result<RasterImage, GpuError> {
    let width = width.max(1);
    let height = height.max(1);

    let passes = plan(stack, channel)?;
    let mut image = RasterImage::filled(width, height, base);
    if passes.is_empty() {
        return Ok(image);
    }

    let shader = device
        .create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("sc.compose.shader"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("sc.compose.layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<Params>() as u64),
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("sc.compose.pipeline_layout"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("sc.compose.pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState { module: &shader, entry_point: Some("vs_main"), compilation_options: Default::default(), buffers: &[] },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: None, write_mask: wgpu::ColorWrites::ALL })],
        }),
        multiview_mask: None,
        cache: None,
    });

    let uniform = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sc.compose.params"),
        size: std::mem::size_of::<Params>() as u64,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let make_texture = |label: &str| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_SRC
                | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        })
    };
    let texture_a = make_texture("sc.compose.a");
    let texture_b = make_texture("sc.compose.b");
    let view_a = texture_a.create_view(&wgpu::TextureViewDescriptor::default());
    let view_b = texture_b.create_view(&wgpu::TextureViewDescriptor::default());

    // Seed A with the base colour.
    {
        let bytes: Vec<u8> = image.pixels.iter().flat_map(|pixel| pixel.iter().flat_map(|c| c.to_le_bytes())).collect();
        queue.write_texture(
            wgpu::TexelCopyTextureInfo { texture: &texture_a, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
            &bytes,
            wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 16), rows_per_image: Some(height) },
            wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        );
    }

    let bind_a = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sc.compose.bind_a"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_a) },
            wgpu::BindGroupEntry { binding: 1, resource: uniform.as_entire_binding() },
        ],
    });
    let bind_b = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sc.compose.bind_b"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view_b) },
            wgpu::BindGroupEntry { binding: 1, resource: uniform.as_entire_binding() },
        ],
    });

    let mut reads_a = true;
    for params in &passes {
        queue.write_buffer(&uniform, 0, bytemuck::bytes_of(params));
        let (dest_view, bind) = if reads_a { (&view_b, &bind_a) } else { (&view_a, &bind_b) };
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sc.compose.encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sc.compose.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: dest_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT), store: wgpu::StoreOp::Store },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, bind, &[]);
            pass.draw(0..3, 0..1);
        }
        queue.submit(Some(encoder.finish()));
        reads_a = !reads_a;
    }

    // The accumulator ends in A if `reads_a` is still true, else B.
    let result_texture = if reads_a { &texture_a } else { &texture_b };
    read_texture(device, queue, result_texture, &mut image)?;
    Ok(image)
}

/// Copy a texture back into `image`.
fn read_texture(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture, image: &mut RasterImage) -> Result<(), GpuError> {
    let (width, height) = (image.width, image.height);
    let unpadded = width as usize * 16;
    let padded = align_to(unpadded as u32, 256) as usize;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("sc.compose.readback"),
        size: (padded * height as usize) as u64,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sc.compose.readback_encoder") });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo { texture, mip_level: 0, origin: wgpu::Origin3d::ZERO, aspect: wgpu::TextureAspect::All },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(padded as u32), rows_per_image: Some(height) },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    queue.submit(Some(encoder.finish()));

    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    match receiver.recv() {
        Ok(Ok(())) => {}
        Ok(Err(err)) => return Err(GpuError::Readback(err.to_string())),
        Err(err) => return Err(GpuError::Readback(err.to_string())),
    }

    let view = slice.get_mapped_range().map_err(|err| GpuError::Readback(err.to_string()))?;
    for row in 0..height as usize {
        let start = row * padded;
        for column in 0..width as usize {
            let offset = start + column * 16;
            let mut pixel = [0.0f32; 4];
            for (component, slot) in pixel.iter_mut().enumerate() {
                let bytes = [
                    view[offset + component * 4],
                    view[offset + component * 4 + 1],
                    view[offset + component * 4 + 2],
                    view[offset + component * 4 + 3],
                ];
                *slot = f32::from_le_bytes(bytes);
            }
            image.set_pixel(column as u32, row as u32, pixel);
        }
    }
    drop(view);
    buffer.unmap();
    Ok(())
}

fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

const SHADER: &str = include_str!("compose.wgsl");

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::{BlendMode, Channel, Layer, LayerStack, Mask};

    fn blend(mode: u32, base: [f32; 3], src: [f32; 3]) -> [f32; 3] {
        let dodge = |b: f32, s: f32| {
            if b <= 0.0 {
                0.0
            } else if s >= 1.0 {
                1.0
            } else {
                (b / (1.0 - s)).min(1.0)
            }
        };
        let burn = |b: f32, s: f32| {
            if b >= 1.0 {
                1.0
            } else if s <= 0.0 {
                0.0
            } else {
                1.0 - ((1.0 - b) / s).min(1.0)
            }
        };
        let hard = |b: f32, s: f32| if s <= 0.5 { 2.0 * b * s } else { 1.0 - 2.0 * (1.0 - b) * (1.0 - s) };
        let mut out = [0.0f32; 3];
        for i in 0..3 {
            let (b, s) = (base[i], src[i]);
            out[i] = match mode {
                0 => s,
                1 => b * s,
                2 => b + s - b * s,
                3 => hard(s, b),
                4 => b + s,
                5 => b - s,
                6 => b.min(s),
                7 => b.max(s),
                8 => dodge(b, s),
                9 => burn(b, s),
                10 => b + s - 1.0,
                11 => hard(b, s),
                12 => {
                    let d = if b <= 0.25 { ((16.0 * b - 12.0) * b + 4.0) * b } else { b.sqrt() };
                    if s <= 0.5 { b - (1.0 - 2.0 * s) * b * (1.0 - b) } else { b + (2.0 * s - 1.0) * (d - b) }
                }
                13 => {
                    if s <= 0.5 {
                        burn(b, 2.0 * s)
                    } else {
                        dodge(b, 2.0 * s - 1.0)
                    }
                }
                14 => b + 2.0 * s - 1.0,
                15 => {
                    if s <= 0.5 {
                        b.min(2.0 * s)
                    } else {
                        b.max(2.0 * s - 1.0)
                    }
                }
                16 => {
                    if b + s >= 1.0 {
                        1.0
                    } else {
                        0.0
                    }
                }
                17 => (b - s).abs(),
                18 => b + s - 2.0 * b * s,
                19 => {
                    if s <= 0.0 {
                        1.0
                    } else {
                        b / s
                    }
                }
                _ => s,
            };
        }
        out
    }

    /// A small independent CPU reference (distinct from the compose crate) so GPU correctness is
    /// not tested against the same code being validated.
    fn cpu_reference(stack: &LayerStack, base: [f32; 4], width: u32, height: u32) -> RasterImage {
        let mut image = RasterImage::filled(width, height, base);
        for layer in &stack.layers {
            if !layer.visible || layer.coverage() <= 0.0 {
                continue;
            }
            let color = match &layer.kind {
                LayerKind::Fill { color, .. } => *color,
                _ => continue,
            };
            let src = [color[0], color[1], color[2]];
            let code = blend_code(layer.blend) as u32;
            for pixel in &mut image.pixels {
                let b = [pixel[0], pixel[1], pixel[2]];
                let blended = blend(code, b, src);
                for c in 0..3 {
                    pixel[c] = (b[c] + (blended[c] - b[c]) * layer.coverage_for(Channel::BaseColor)).clamp(0.0, 1.0);
                }
                pixel[3] = 1.0;
            }
        }
        image
    }

    #[test]
    fn gpu_matches_cpu_for_a_fill_stack() {
        let Ok((device, queue)) = headless_device() else {
            return;
        };

        let mut stack = LayerStack::new();
        stack.push(Layer::fill("base", [0.1, 0.2, 0.3, 1.0], 0.0));
        let mut mid = Layer::fill("mid", [0.9, 0.4, 0.1, 1.0], 0.0);
        mid.blend = BlendMode::Multiply;
        mid.opacity = 0.7;
        stack.push(mid);
        let mut top = Layer::fill("top", [0.2, 0.8, 0.5, 1.0], 0.0);
        top.blend = BlendMode::Screen;
        top.mask = Some(Mask::Constant(0.6));
        stack.push(top);

        let base = [0.05, 0.05, 0.05, 1.0];
        let (width, height) = (16, 16);
        let gpu = composite_channel(&device, &queue, &stack, Channel::BaseColor, base, width, height).expect("gpu");
        let cpu = cpu_reference(&stack, base, width, height);

        for (gpu_pixel, cpu_pixel) in gpu.pixels.iter().zip(&cpu.pixels) {
            for c in 0..4 {
                assert!((gpu_pixel[c] - cpu_pixel[c]).abs() < 2e-3, "gpu {gpu_pixel:?} vs cpu {cpu_pixel:?}");
            }
        }
    }

    #[test]
    fn gpu_matches_cpu_for_expanded_blend_modes() {
        let Ok((device, queue)) = headless_device() else {
            return;
        };

        for mode in [BlendMode::ColorDodge, BlendMode::ColorBurn, BlendMode::Difference, BlendMode::SoftLight, BlendMode::Divide] {
            let mut stack = LayerStack::new();
            let mut layer = Layer::fill("fill", [0.6, 0.35, 0.8, 1.0], 0.0);
            layer.blend = mode;
            stack.push(layer);

            let base = [0.2, 0.7, 0.4, 1.0];
            let (width, height) = (8, 8);
            let gpu = composite_channel(&device, &queue, &stack, Channel::BaseColor, base, width, height).expect("gpu");
            let cpu = cpu_reference(&stack, base, width, height);
            for (gpu_pixel, cpu_pixel) in gpu.pixels.iter().zip(&cpu.pixels) {
                for c in 0..4 {
                    assert!((gpu_pixel[c] - cpu_pixel[c]).abs() < 2e-3, "{mode:?}: gpu {gpu_pixel:?} vs cpu {cpu_pixel:?}");
                }
            }
        }
    }

    #[test]
    fn groups_are_reported_unsupported_before_gpu_work() {
        let mut stack = LayerStack::new();
        stack.push(Layer::group("grp"));
        assert!(matches!(plan(&stack, Channel::BaseColor), Err(GpuError::Unsupported(_))));
    }

    #[test]
    fn plan_counts_only_contributing_layers() {
        let mut stack = LayerStack::new();
        stack.push(Layer::fill("a", [0.0; 4], 0.0));
        let mut hidden = Layer::fill("b", [1.0; 4], 1.0);
        hidden.visible = false;
        stack.push(hidden);
        assert!(matches!(plan(&stack, Channel::BaseColor), Ok(passes) if passes.len() == 1));
    }
}
