//! Render a scene offscreen to a PNG — no window, no focus stealing. Used for visual checks.
//!
//! `cargo run -p substancecraft-ui --example snapshot -- \
//!     [output.png] [display] [projection] [env: synthetic|<path>] [model: sphere|<path>]`

use std::process::ExitCode;

use substancecraft_doc::Scene;
use substancecraft_geom::primitives::uv_sphere;
use substancecraft_render::{DisplayMode, Headless, OrbitCamera, Projection, RenderSettings, render_rgba_env, synthetic_environment};

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
    let mut args = std::env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "Build/snapshot.png".to_string());
    let display = args.next().and_then(|id| DisplayMode::from_id(&id)).unwrap_or_default();
    let projection = args.next().and_then(|id| Projection::from_id(&id)).unwrap_or_default();
    let environment = args.next();
    let model = args.next();

    let scene = match model.as_deref() {
        Some("sphere") | None => Scene::from_mesh(uv_sphere(1.0, 96, 128)?),
        Some("torus") => Scene::from_mesh(substancecraft_geom::primitives::torus(1.0, 0.35, 128, 64)?),
        Some(path) => substancecraft_io::load_scene(path)?,
    };

    let mut camera = OrbitCamera::default();
    camera.focus(scene.mesh.bounds());
    camera.projection = projection;

    let settings = RenderSettings { display, light_direction: [0.6, 0.9, 0.7], ..RenderSettings::default() };

    let environment = match environment.as_deref() {
        Some("synthetic") | Some("env") => Some(synthetic_environment(512, 256)),
        Some(path) => Some(substancecraft_codecs::load_environment(path)?),
        None => None,
    };

    let headless = Headless::new()?;
    let width = 960;
    let height = 640;
    let pixels = render_rgba_env(&headless, &scene, &camera, &settings, environment.as_ref(), width, height)?;

    if let Some(parent) = std::path::Path::new(&path).parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)?;
    }
    image::save_buffer(&path, &pixels, width, height, image::ExtendedColorType::Rgba8)?;
    Ok(path)
}
