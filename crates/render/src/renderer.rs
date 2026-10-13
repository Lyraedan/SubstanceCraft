//! The wgpu viewport renderer: one offscreen colour+depth target drawn with PBR, background,
//! grid and wireframe pipelines.

use std::collections::HashSet;

use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use substancecraft_codecs::ImageF32;
use substancecraft_doc::Scene;
use thiserror::Error;

use crate::camera::OrbitCamera;
use crate::settings::{DisplayMode, RenderSettings};

/// The offscreen colour format. `Rgba8Unorm` (gamma storage) is what egui expects for a native
/// texture, and the shaders write sRGB-encoded values.
pub const COLOR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
/// The depth format.
pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

const MAX_MATERIALS: usize = 256;
/// The ground grid extends this many units in each direction.
const GRID_HALF_EXTENT: i32 = 20;

/// Errors from the renderer. All are recoverable and reported, never panicked.
#[derive(Debug, Error)]
pub enum RenderError {
    /// No offscreen target exists yet (call `resize` first).
    #[error("no render target; resize the viewport first")]
    NoTarget,
    /// No GPU adapter was available for a headless render.
    #[error("no usable GPU adapter was found")]
    NoAdapter,
    /// The GPU device could not be created.
    #[error("could not create the GPU device: {0}")]
    Device(String),
    /// Framebuffer readback failed.
    #[error("framebuffer readback failed: {0}")]
    Readback(String),
}

/// A packed mesh vertex: position, normal, UV and tangent (with bitangent sign).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    /// Object-space position.
    pub position: [f32; 3],
    /// Object-space normal.
    pub normal: [f32; 3],
    /// UV.
    pub uv: [f32; 2],
    /// Tangent with bitangent sign in `w`.
    pub tangent: [f32; 4],
}

/// A packed line vertex: position and gamma colour.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
struct LineVertex {
    position: [f32; 3],
    color: [f32; 3],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct CameraUniform {
    view_proj: [[f32; 4]; 4],
    inv_view_proj: [[f32; 4]; 4],
    camera_pos: [f32; 4],
    light_dir: [f32; 4],
    light_color: [f32; 4],
    ambient_sky: [f32; 4],
    ambient_ground: [f32; 4],
    params: [f32; 4],
    env_params: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct MaterialUniform {
    base_color: [f32; 4],
    params: [f32; 4],
}

/// One submesh draw: a material index, its first index and its index count.
#[derive(Clone, Copy, Debug)]
struct DrawRange {
    material: u32,
    first_index: u32,
    index_count: u32,
}

/// The viewport renderer. Draws the current scene into an offscreen colour+depth target.
pub struct ViewportRenderer {
    pbr_pipeline: wgpu::RenderPipeline,
    line_pipeline: wgpu::RenderPipeline,
    background_pipeline: wgpu::RenderPipeline,

    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    material_buffer: wgpu::Buffer,
    material_bind_group: wgpu::BindGroup,
    material_stride: u32,
    env_layout: wgpu::BindGroupLayout,
    env_sampler: wgpu::Sampler,
    env_bind_group: wgpu::BindGroup,
    env_texture: Option<wgpu::Texture>,
    max_lod: f32,
    has_environment: bool,

    vertex_buffer: Option<wgpu::Buffer>,
    index_buffer: Option<wgpu::Buffer>,
    draws: Vec<DrawRange>,
    edge_buffer: Option<wgpu::Buffer>,
    edge_vertex_count: u32,
    grid_buffer: wgpu::Buffer,
    grid_vertex_count: u32,

    color: Option<wgpu::Texture>,
    color_view: Option<wgpu::TextureView>,
    depth_view: Option<wgpu::TextureView>,
    size: (u32, u32),
}

impl ViewportRenderer {
    /// Build the pipelines and uniform resources for a device.
    pub fn new(device: &wgpu::Device) -> Self {
        let pbr_shader = device
            .create_shader_module(wgpu::ShaderModuleDescriptor { label: Some("sc.viewport.pbr"), source: wgpu::ShaderSource::Wgsl(SHADER.into()) });
        let line_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sc.viewport.line"),
            source: wgpu::ShaderSource::Wgsl(LINE_SHADER.into()),
        });
        let background_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sc.viewport.background"),
            source: wgpu::ShaderSource::Wgsl(BACKGROUND_SHADER.into()),
        });

        let camera_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sc.viewport.camera_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<CameraUniform>() as u64),
                },
                count: None,
            }],
        });

        let material_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sc.viewport.material_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: true,
                    min_binding_size: wgpu::BufferSize::new(std::mem::size_of::<MaterialUniform>() as u64),
                },
                count: None,
            }],
        });

        let camera_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sc.viewport.camera"),
            size: std::mem::size_of::<CameraUniform>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sc.viewport.camera_bind"),
            layout: &camera_layout,
            entries: &[wgpu::BindGroupEntry { binding: 0, resource: camera_buffer.as_entire_binding() }],
        });

        let alignment = device.limits().min_uniform_buffer_offset_alignment.max(1);
        let material_size = std::mem::size_of::<MaterialUniform>() as u32;
        let material_stride = align_to(material_size, alignment);
        let material_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sc.viewport.materials"),
            size: (material_stride as u64) * (MAX_MATERIALS as u64),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let material_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sc.viewport.material_bind"),
            layout: &material_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &material_buffer,
                    offset: 0,
                    size: wgpu::BufferSize::new(material_size as u64),
                }),
            }],
        });

        let env_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sc.viewport.env_layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let env_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sc.viewport.env_sampler"),
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
            ..Default::default()
        });
        let default_env = create_solid_environment(device, &env_sampler, &env_layout);

        let camera_only_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sc.viewport.camera_only_layout"),
            bind_group_layouts: &[Some(&camera_layout)],
            immediate_size: 0,
        });
        let background_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sc.viewport.background_layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&env_layout)],
            immediate_size: 0,
        });
        let pbr_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sc.viewport.pbr_layout"),
            bind_group_layouts: &[Some(&camera_layout), Some(&material_layout), Some(&env_layout)],
            immediate_size: 0,
        });

        let depth = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::Less),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        let depth_lequal = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(true),
            depth_compare: Some(wgpu::CompareFunction::LessEqual),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });
        let depth_none = Some(wgpu::DepthStencilState {
            format: DEPTH_FORMAT,
            depth_write_enabled: Some(false),
            depth_compare: Some(wgpu::CompareFunction::Always),
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        });

        let pbr_pipeline = create_pipeline(
            device,
            "sc.viewport.pbr_pipeline",
            &pbr_layout,
            &pbr_shader,
            "vs_main",
            "fs_main",
            &[Some(Vertex::layout())],
            wgpu::PrimitiveTopology::TriangleList,
            depth,
        );
        let line_pipeline = create_pipeline(
            device,
            "sc.viewport.line_pipeline",
            &camera_only_layout,
            &line_shader,
            "vs_line",
            "fs_line",
            &[Some(LineVertex::layout())],
            wgpu::PrimitiveTopology::LineList,
            depth_lequal,
        );
        let background_pipeline = create_pipeline(
            device,
            "sc.viewport.background_pipeline",
            &background_layout,
            &background_shader,
            "vs_background",
            "fs_background",
            &[],
            wgpu::PrimitiveTopology::TriangleList,
            depth_none,
        );

        let grid_vertices = build_grid(GRID_HALF_EXTENT, 1.0);
        let grid_bytes = bytemuck::cast_slice(&grid_vertices);
        let grid_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sc.viewport.grid"),
            size: grid_bytes.len() as u64,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: true,
        });
        {
            if let Ok(mut view) = grid_buffer.slice(..).get_mapped_range_mut() {
                view.copy_from_slice(grid_bytes);
            }
        }
        grid_buffer.unmap();

        Self {
            pbr_pipeline,
            line_pipeline,
            background_pipeline,
            camera_buffer,
            camera_bind_group,
            material_buffer,
            material_bind_group,
            material_stride,
            env_layout,
            env_sampler,
            env_bind_group: default_env,
            env_texture: None,
            max_lod: 0.0,
            has_environment: false,
            vertex_buffer: None,
            index_buffer: None,
            draws: Vec::new(),
            edge_buffer: None,
            edge_vertex_count: 0,
            grid_buffer,
            grid_vertex_count: grid_vertices.len() as u32,
            color: None,
            color_view: None,
            depth_view: None,
            size: (0, 0),
        }
    }

    /// Upload a scene's geometry and materials, replacing any previous scene.
    pub fn set_scene(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, scene: &Scene) {
        self.draws.clear();
        self.vertex_buffer = None;
        self.index_buffer = None;
        self.edge_buffer = None;
        self.edge_vertex_count = 0;

        let mesh = &scene.mesh;
        if mesh.is_empty() || mesh.indices.is_empty() {
            return;
        }

        let vertices: Vec<Vertex> = (0..mesh.vertex_count())
            .map(|index| Vertex {
                position: mesh.positions.get(index).copied().unwrap_or(Vec3::ZERO).to_array(),
                normal: mesh.normals.get(index).copied().unwrap_or(Vec3::Z).to_array(),
                uv: mesh.uvs.get(index).copied().unwrap_or_default().to_array(),
                tangent: mesh.tangents.get(index).copied().unwrap_or(glam::Vec4::new(1.0, 0.0, 0.0, 1.0)).to_array(),
            })
            .collect();

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sc.viewport.vertices"),
            size: (std::mem::size_of::<Vertex>() * vertices.len()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&vertex_buffer, 0, bytemuck::cast_slice(&vertices));

        let index_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sc.viewport.indices"),
            size: (std::mem::size_of::<u32>() * mesh.indices.len()) as u64,
            usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        queue.write_buffer(&index_buffer, 0, bytemuck::cast_slice(&mesh.indices));

        let submeshes = if mesh.submeshes.is_empty() {
            vec![substancecraft_geom::Submesh { material: 0, first_index: 0, index_count: mesh.indices.len() as u32 }]
        } else {
            mesh.submeshes.clone()
        };
        for submesh in submeshes {
            self.draws.push(DrawRange { material: submesh.material, first_index: submesh.first_index, index_count: submesh.index_count });
        }

        let mut materials = vec![[0.0f32; 8]; MAX_MATERIALS];
        for (index, material) in scene.materials.iter().take(MAX_MATERIALS).enumerate() {
            materials[index] = [
                material.base_color[0],
                material.base_color[1],
                material.base_color[2],
                material.base_color[3],
                material.metallic,
                material.roughness,
                0.0,
                0.0,
            ];
        }
        if scene.materials.is_empty() {
            materials[0] = [0.8, 0.8, 0.8, 1.0, 0.0, 0.5, 0.0, 0.0];
        }
        let mut bytes = vec![0u8; self.material_stride as usize * MAX_MATERIALS];
        for (index, values) in materials.iter().enumerate() {
            let uniform =
                MaterialUniform { base_color: [values[0], values[1], values[2], values[3]], params: [values[4], values[5], values[6], values[7]] };
            let offset = index * self.material_stride as usize;
            bytes[offset..offset + std::mem::size_of::<MaterialUniform>()].copy_from_slice(bytemuck::bytes_of(&uniform));
        }
        queue.write_buffer(&self.material_buffer, 0, &bytes);

        let edge_vertices = build_edges(mesh);
        self.edge_vertex_count = edge_vertices.len() as u32;
        if !edge_vertices.is_empty() {
            let edge_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sc.viewport.edges"),
                size: (std::mem::size_of::<LineVertex>() * edge_vertices.len()) as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            queue.write_buffer(&edge_buffer, 0, bytemuck::cast_slice(&edge_vertices));
            self.edge_buffer = Some(edge_buffer);
        }

        self.vertex_buffer = Some(vertex_buffer);
        self.index_buffer = Some(index_buffer);
    }

    /// Throw away the uploaded scene.
    pub fn clear_scene(&mut self) {
        self.vertex_buffer = None;
        self.index_buffer = None;
        self.edge_buffer = None;
        self.edge_vertex_count = 0;
        self.draws.clear();
    }

    /// Upload an equirectangular HDR environment for image-based lighting and the background.
    pub fn set_environment(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, image: &ImageF32) {
        if image.is_empty() {
            self.clear_environment(device);
            return;
        }

        let levels = build_mip_chain(image);
        let mip_level_count = levels.len().max(1) as u32;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sc.viewport.environment"),
            size: wgpu::Extent3d { width: image.width.max(1), height: image.height.max(1), depth_or_array_layers: 1 },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba16Float,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        for (level, pixels) in levels.iter().enumerate() {
            let width = (image.width >> level).max(1);
            let height = (image.height >> level).max(1);
            let mut bytes = Vec::with_capacity(pixels.len() * 8);
            for pixel in pixels {
                for channel in pixel {
                    bytes.extend_from_slice(&half::f16::from_f32(*channel).to_le_bytes());
                }
            }
            queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                &bytes,
                wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(width * 8), rows_per_image: Some(height) },
                wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            );
        }

        let view =
            texture.create_view(&wgpu::TextureViewDescriptor { base_mip_level: 0, mip_level_count: Some(mip_level_count), ..Default::default() });
        self.env_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("sc.viewport.env_bind"),
            layout: &self.env_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
                wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(&self.env_sampler) },
            ],
        });
        self.env_texture = Some(texture);
        self.max_lod = mip_level_count.saturating_sub(1) as f32;
        self.has_environment = true;
    }

    /// Drop the environment and fall back to the hemisphere ambient and gradient background.
    pub fn clear_environment(&mut self, device: &wgpu::Device) {
        self.env_bind_group = create_solid_environment(device, &self.env_sampler, &self.env_layout);
        self.env_texture = None;
        self.max_lod = 0.0;
        self.has_environment = false;
    }

    /// (Re)create the offscreen colour and depth targets. A zero size is treated as 1×1.
    pub fn resize(&mut self, device: &wgpu::Device, width: u32, height: u32) {
        let width = width.max(1);
        let height = height.max(1);
        if self.size == (width, height) && self.color.is_some() {
            return;
        }
        let extent = wgpu::Extent3d { width, height, depth_or_array_layers: 1 };

        let color = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sc.viewport.color"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: COLOR_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let depth = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("sc.viewport.depth"),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: DEPTH_FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });

        self.color_view = Some(color.create_view(&wgpu::TextureViewDescriptor::default()));
        self.depth_view = Some(depth.create_view(&wgpu::TextureViewDescriptor::default()));
        self.color = Some(color);
        self.size = (width, height);
    }

    /// The current offscreen size in pixels.
    pub fn size(&self) -> (u32, u32) {
        self.size
    }

    /// The offscreen colour texture, for registering with egui.
    pub fn texture(&self) -> Option<&wgpu::Texture> {
        self.color.as_ref()
    }

    /// The offscreen colour view.
    pub fn color_view(&self) -> Option<&wgpu::TextureView> {
        self.color_view.as_ref()
    }

    /// Draw the current scene. A no-op if there is no target.
    pub fn render(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, camera: &OrbitCamera, settings: &RenderSettings) {
        let (Some(color_view), Some(depth_view)) = (self.color_view.as_ref(), self.depth_view.as_ref()) else {
            return;
        };

        let (width, height) = self.size;
        let aspect = width as f32 / height as f32;
        let view_proj = camera.view_projection(aspect);
        let uniform = CameraUniform {
            view_proj: view_proj.to_cols_array_2d(),
            inv_view_proj: view_proj.inverse().to_cols_array_2d(),
            camera_pos: camera.position().extend(1.0).to_array(),
            light_dir: [settings.light_direction[0], settings.light_direction[1], settings.light_direction[2], 0.0],
            light_color: [settings.light_color[0], settings.light_color[1], settings.light_color[2], 1.0],
            ambient_sky: [settings.ambient_sky[0], settings.ambient_sky[1], settings.ambient_sky[2], 1.0],
            ambient_ground: [settings.ambient_ground[0], settings.ambient_ground[1], settings.ambient_ground[2], 1.0],
            params: [settings.exposure_stops, settings.display.shader_code(), settings.tone_map_code(), 0.0],
            env_params: [if self.has_environment { 1.0 } else { 0.0 }, self.max_lod, 0.0, 0.0],
        };
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::bytes_of(&uniform));

        let has_mesh = self.vertex_buffer.is_some() && self.index_buffer.is_some() && !self.draws.is_empty();
        let background = settings.background;
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sc.viewport.encoder") });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("sc.viewport.pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: color_view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color { r: background[0] as f64, g: background[1] as f64, b: background[2] as f64, a: 1.0 }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: depth_view,
                    depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(1.0), store: wgpu::StoreOp::Store }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });

            if settings.show_background {
                pass.set_pipeline(&self.background_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(1, &self.env_bind_group, &[]);
                pass.draw(0..3, 0..1);
            }

            if has_mesh && settings.display != DisplayMode::Wireframe {
                pass.set_pipeline(&self.pbr_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_bind_group(2, &self.env_bind_group, &[]);
                let (Some(vertex_buffer), Some(index_buffer)) = (self.vertex_buffer.as_ref(), self.index_buffer.as_ref()) else {
                    return;
                };
                pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint32);
                for draw in &self.draws {
                    let material = draw.material.min(MAX_MATERIALS as u32 - 1);
                    let offset = material * self.material_stride;
                    pass.set_bind_group(1, &self.material_bind_group, &[offset]);
                    pass.draw_indexed(draw.first_index..draw.first_index + draw.index_count, 0, 0..1);
                }
            }

            if settings.show_grid && self.grid_vertex_count != 0 {
                pass.set_pipeline(&self.line_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, self.grid_buffer.slice(..));
                pass.draw(0..self.grid_vertex_count, 0..1);
            }

            if settings.display == DisplayMode::Wireframe
                && let Some(edge_buffer) = self.edge_buffer.as_ref()
            {
                pass.set_pipeline(&self.line_pipeline);
                pass.set_bind_group(0, &self.camera_bind_group, &[]);
                pass.set_vertex_buffer(0, edge_buffer.slice(..));
                pass.draw(0..self.edge_vertex_count, 0..1);
            }
        }
        queue.submit(Some(encoder.finish()));
    }

    /// Read the colour target back as tightly packed RGBA8 rows (top-left origin).
    pub fn read_pixels(&self, device: &wgpu::Device, queue: &wgpu::Queue) -> Result<Vec<u8>, RenderError> {
        let texture = self.color.as_ref().ok_or(RenderError::NoTarget)?;
        let (width, height) = self.size;
        if width == 0 || height == 0 {
            return Err(RenderError::NoTarget);
        }

        let unpadded = width as usize * 4;
        let padded = align_to(unpadded as u32, 256) as usize;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("sc.viewport.readback"),
            size: (padded * height as usize) as u64,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some("sc.viewport.readback_encoder") });
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
            Ok(Err(err)) => return Err(RenderError::Readback(err.to_string())),
            Err(err) => return Err(RenderError::Readback(err.to_string())),
        }

        let view = slice.get_mapped_range().map_err(|err| RenderError::Readback(err.to_string()))?;
        let mut pixels = Vec::with_capacity(unpadded * height as usize);
        for row in 0..height as usize {
            let start = row * padded;
            pixels.extend_from_slice(&view[start..start + unpadded]);
        }
        drop(view);
        buffer.unmap();
        Ok(pixels)
    }
}

impl Vertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 4] = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x2, offset: 24, shader_location: 2 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x4, offset: 32, shader_location: 3 },
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

impl LineVertex {
    fn layout() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBUTES: [wgpu::VertexAttribute; 2] = [
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
            wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 12, shader_location: 1 },
        ];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<LineVertex>() as u64,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBUTES,
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn create_pipeline(
    device: &wgpu::Device,
    label: &str,
    layout: &wgpu::PipelineLayout,
    shader: &wgpu::ShaderModule,
    vs_entry: &str,
    fs_entry: &str,
    buffers: &[Option<wgpu::VertexBufferLayout<'_>>],
    topology: wgpu::PrimitiveTopology,
    depth_stencil: Option<wgpu::DepthStencilState>,
) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState { module: shader, entry_point: Some(vs_entry), compilation_options: Default::default(), buffers },
        primitive: wgpu::PrimitiveState {
            topology,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: if topology == wgpu::PrimitiveTopology::TriangleList { Some(wgpu::Face::Back) } else { None },
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some(fs_entry),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: COLOR_FORMAT,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// Build the ground grid as a line list, with tinted major axes.
fn build_grid(half_extent: i32, spacing: f32) -> Vec<LineVertex> {
    let mut vertices = Vec::new();
    let extent = half_extent as f32 * spacing;
    for i in -half_extent..=half_extent {
        let offset = i as f32 * spacing;
        let major = i % 5 == 0;
        let (x_color, z_color) = if i == 0 {
            ([0.75, 0.30, 0.30], [0.30, 0.45, 0.78])
        } else if major {
            ([0.42, 0.43, 0.46], [0.42, 0.43, 0.46])
        } else {
            ([0.26, 0.27, 0.30], [0.26, 0.27, 0.30])
        };
        vertices.push(LineVertex { position: [-extent, 0.0, offset], color: x_color });
        vertices.push(LineVertex { position: [extent, 0.0, offset], color: x_color });
        vertices.push(LineVertex { position: [offset, 0.0, -extent], color: z_color });
        vertices.push(LineVertex { position: [offset, 0.0, extent], color: z_color });
    }
    vertices
}

/// Build a unique-edge line list from a mesh's triangles.
fn build_edges(mesh: &substancecraft_geom::Mesh) -> Vec<LineVertex> {
    let color = [0.85, 0.86, 0.9];
    let mut seen = HashSet::new();
    let mut vertices = Vec::new();
    for triangle in mesh.indices.as_chunks::<3>().0 {
        let [a, b, c] = *triangle;
        for (from, to) in [(a, b), (b, c), (c, a)] {
            let key = if from < to { (from, to) } else { (to, from) };
            if seen.insert(key) {
                for index in [from, to] {
                    let position = mesh.positions.get(index as usize).copied().unwrap_or(Vec3::ZERO).to_array();
                    vertices.push(LineVertex { position, color });
                }
            }
        }
    }
    vertices
}

fn align_to(value: u32, alignment: u32) -> u32 {
    if alignment == 0 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

/// A 1×1 placeholder environment so the bind group is always valid.
fn create_solid_environment(device: &wgpu::Device, sampler: &wgpu::Sampler, layout: &wgpu::BindGroupLayout) -> wgpu::BindGroup {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("sc.viewport.env_default"),
        size: wgpu::Extent3d { width: 1, height: 1, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("sc.viewport.env_default_bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry { binding: 0, resource: wgpu::BindingResource::TextureView(&view) },
            wgpu::BindGroupEntry { binding: 1, resource: wgpu::BindingResource::Sampler(sampler) },
        ],
    })
}

/// Build a box-filtered mip chain from a linear image, down to 1×1.
fn build_mip_chain(image: &ImageF32) -> Vec<Vec<[f32; 4]>> {
    let mut levels = Vec::new();
    let mut current = image.pixels.clone();
    let (mut width, mut height) = (image.width.max(1), image.height.max(1));
    levels.push(current.clone());

    while width > 1 || height > 1 {
        let next_width = (width / 2).max(1);
        let next_height = (height / 2).max(1);
        let mut next = vec![[0.0f32; 4]; (next_width * next_height) as usize];
        for y in 0..next_height {
            for x in 0..next_width {
                let mut accumulator = [0.0f32; 4];
                let mut samples = 0.0f32;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let source_x = (x * 2 + dx).min(width - 1);
                        let source_y = (y * 2 + dy).min(height - 1);
                        if let Some(pixel) = current.get((source_y * width + source_x) as usize) {
                            for (slot, value) in accumulator.iter_mut().zip(pixel.iter()) {
                                *slot += *value;
                            }
                            samples += 1.0;
                        }
                    }
                }
                if samples > 0.0 {
                    for slot in &mut accumulator {
                        *slot /= samples;
                    }
                }
                if let Some(slot) = next.get_mut((y * next_width + x) as usize) {
                    *slot = accumulator;
                }
            }
        }
        levels.push(next.clone());
        current = next;
        width = next_width;
        height = next_height;
    }

    levels
}

const SHADER: &str = include_str!("viewport.wgsl");
const LINE_SHADER: &str = include_str!("line.wgsl");
const BACKGROUND_SHADER: &str = include_str!("background.wgsl");
