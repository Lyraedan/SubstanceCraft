//! Headless GPU rendering: an offscreen device for snapshots, tests and headless export.

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
    let mut renderer = ViewportRenderer::new(&headless.device);
    renderer.set_scene(&headless.device, &headless.queue, scene);
    renderer.resize(&headless.device, width, height);
    renderer.render(&headless.device, &headless.queue, camera, settings);
    renderer.read_pixels(&headless.device, &headless.queue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::Scene;

    #[test]
    fn renders_a_lit_sphere_offscreen() {
        // Skip cleanly where no GPU adapter exists (e.g. some CI sandboxes).
        let Ok(headless) = Headless::new() else {
            return;
        };

        let mesh = substancecraft_geom::primitives::uv_sphere(1.0, 32, 48).expect("sphere");
        let scene = Scene::from_mesh(mesh);
        let mut camera = OrbitCamera::default();
        camera.focus(scene.mesh.bounds());
        let settings = RenderSettings::default();

        let pixels = render_rgba_with(&headless, &scene, &camera, &settings, 128, 128).expect("render");
        assert_eq!(pixels.len(), 128 * 128 * 4);

        // The sphere must be visible: at least one pixel differs from the background clear colour.
        let background = [settings.background[0], settings.background[1], settings.background[2]];
        let has_surface = pixels.as_chunks::<4>().0.iter().any(|px| {
            let [r, g, b, _] = *px;
            let r = r as f32 / 255.0;
            let g = g as f32 / 255.0;
            let b = b as f32 / 255.0;
            (r - background[0]).abs() + (g - background[1]).abs() + (b - background[2]).abs() > 0.05
        });
        assert!(has_surface, "the rendered sphere is not visible");
    }
}
