//! Headless GPU rendering: an offscreen device for snapshots, tests and headless export.

use std::f32::consts::PI;

use glam::Vec3;
use substancecraft_codecs::ImageF32;
use substancecraft_doc::Scene;

use crate::camera::OrbitCamera;
use crate::renderer::{RenderError, ViewportRenderer};
use crate::settings::RenderSettings;

/// A GPU device and queue created without a window, for offscreen rendering.
pub struct Headless {
    /// The wgpu device.
    pub device: wgpu::Device,
    /// The wgpu queue.
    pub queue: wgpu::Queue,
}

impl Headless {
    /// Create a headless device using the system's default adapter.
    pub fn new() -> Result<Self, RenderError> {
        let instance = wgpu::Instance::default();
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default())).map_err(|_| RenderError::NoAdapter)?;
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default())).map_err(|err| RenderError::Device(err.to_string()))?;
        Ok(Self { device, queue })
    }
}

/// Render a scene headlessly and return tightly packed RGBA8 pixels (top-left origin).
pub fn render_rgba(scene: &Scene, camera: &OrbitCamera, settings: &RenderSettings, width: u32, height: u32) -> Result<Vec<u8>, RenderError> {
    let headless = Headless::new()?;
    render_rgba_with(&headless, scene, camera, settings, width, height)
}

/// Render a scene with an existing headless context.
pub fn render_rgba_with(
    headless: &Headless,
    scene: &Scene,
    camera: &OrbitCamera,
    settings: &RenderSettings,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RenderError> {
    render_rgba_env(headless, scene, camera, settings, None, width, height)
}

/// Render a scene with an optional equirectangular environment map.
pub fn render_rgba_env(
    headless: &Headless,
    scene: &Scene,
    camera: &OrbitCamera,
    settings: &RenderSettings,
    environment: Option<&ImageF32>,
    width: u32,
    height: u32,
) -> Result<Vec<u8>, RenderError> {
    let mut renderer = ViewportRenderer::new(&headless.device);
    renderer.set_scene(&headless.device, &headless.queue, scene);
    if let Some(environment) = environment {
        renderer.set_environment(&headless.device, &headless.queue, environment);
    }
    renderer.resize(&headless.device, width, height);
    renderer.render(&headless.device, &headless.queue, camera, settings);
    renderer.read_pixels(&headless.device, &headless.queue)
}

/// A small synthetic equirectangular environment (gradient sky, warm sun, dark ground), for tests
/// and demos where no HDRI file is available.
pub fn synthetic_environment(width: u32, height: u32) -> ImageF32 {
    let width = width.max(4);
    let height = height.max(2);
    let sun_direction = Vec3::new(0.5, 0.8, 0.6).normalize_or_zero();
    let sky = [0.30, 0.40, 0.55];
    let ground = [0.05, 0.045, 0.04];

    let mut pixels = Vec::with_capacity((width * height) as usize);
    for y in 0..height {
        let v = (y as f32 + 0.5) / height as f32;
        let theta = v * PI;
        for x in 0..width {
            let u = (x as f32 + 0.5) / width as f32;
            let phi = u * 2.0 * PI;
            let direction = Vec3::new(theta.sin() * phi.cos(), theta.cos(), theta.sin() * phi.sin());
            let t = (direction.y * 0.5 + 0.5).clamp(0.0, 1.0);
            let sun = direction.dot(sun_direction).max(0.0).powf(64.0) * 8.0;
            pixels.push([
                ground[0] + (sky[0] - ground[0]) * t + sun,
                ground[1] + (sky[1] - ground[1]) * t + sun * 0.95,
                ground[2] + (sky[2] - ground[2]) * t + sun * 0.85,
                1.0,
            ]);
        }
    }
    ImageF32 { width, height, pixels }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sphere_scene() -> Scene {
        let mesh = substancecraft_geom::primitives::uv_sphere(1.0, 32, 48).expect("sphere");
        Scene::from_mesh(mesh)
    }

    #[test]
    fn renders_a_lit_sphere_offscreen() {
        // Skip cleanly where no GPU adapter exists (e.g. some CI sandboxes).
        let Ok(headless) = Headless::new() else {
            return;
        };

        let scene = sphere_scene();
        let mut camera = OrbitCamera::default();
        camera.focus(scene.mesh.bounds());
        let settings = RenderSettings::default();

        let pixels = render_rgba_with(&headless, &scene, &camera, &settings, 128, 128).expect("render");
        assert_eq!(pixels.len(), 128 * 128 * 4);

        // The sphere must be visible: at least one pixel differs from the background clear colour.
        let background = [settings.background[0], settings.background[1], settings.background[2]];
        assert!(has_surface(&pixels, background), "the rendered sphere is not visible");
    }

    #[test]
    fn renders_with_an_environment_map() {
        let Ok(headless) = Headless::new() else {
            return;
        };

        let scene = sphere_scene();
        let mut camera = OrbitCamera::default();
        camera.focus(scene.mesh.bounds());
        let environment = synthetic_environment(256, 128);

        let pixels = render_rgba_env(&headless, &scene, &camera, &RenderSettings::default(), Some(&environment), 128, 128).expect("render");
        assert_eq!(pixels.len(), 128 * 128 * 4);

        let background = RenderSettings::default().background;
        assert!(has_surface(&pixels, background), "the environment-lit scene is not visible");
    }

    fn has_surface(pixels: &[u8], background: [f32; 3]) -> bool {
        pixels.as_chunks::<4>().0.iter().any(|px| {
            let [r, g, b, _] = *px;
            let r = r as f32 / 255.0;
            let g = g as f32 / 255.0;
            let b = b as f32 / 255.0;
            (r - background[0]).abs() + (g - background[1]).abs() + (b - background[2]).abs() > 0.05
        })
    }
}
