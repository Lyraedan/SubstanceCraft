//! Render a scene offscreen to a PNG — no window, no focus stealing. Used for visual checks.
//!
//! `cargo run -p substancecraft-ui --example snapshot -- [output.png]`

use std::process::ExitCode;

use substancecraft_doc::Scene;
use substancecraft_geom::primitives::uv_sphere;
use substancecraft_render::{OrbitCamera, RenderSettings, render_rgba};

fn main() -> ExitCode {
    match run() {
        Ok(path) => {
            println!("wrote {path}");
            ExitCode::SUCCESS
        }
        Err(err) => {
            eprintln!("snapshot failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, Box<dyn std::error::Error>> {
    let path = std::env::args().nth(1).unwrap_or_else(|| "Build/snapshot.png".to_string());

    let mesh = uv_sphere(1.0, 96, 128)?;
    let scene = Scene::from_mesh(mesh);

    let mut camera = OrbitCamera::default();
    camera.focus(scene.mesh.bounds());

    let settings = RenderSettings { light_direction: [0.6, 0.9, 0.7], ..RenderSettings::default() };

    let width = 960;
    let height = 640;
    let pixels = render_rgba(&scene, &camera, &settings, width, height)?;

    if let Some(parent) = std::path::Path::new(&path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    image::save_buffer(&path, &pixels, width, height, image::ExtendedColorType::Rgba8)?;
    Ok(path)
}
