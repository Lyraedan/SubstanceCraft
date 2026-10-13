//! Performance harness: time a few representative operations, compare against budgets, and record a
//! baseline. `cargo xtask perf [--update-baseline]`.

use std::collections::BTreeMap;
use std::path::Path;
use std::process::ExitCode;
use std::time::Instant;

const BASELINE: &str = "perf/baseline.json";
const BUDGETS: &str = "perf/budgets.toml";

/// Run the scenarios, print timings, write the baseline and check budgets.
pub fn run() -> ExitCode {
    let mut results: BTreeMap<String, f64> = BTreeMap::new();

    println!("running perf scenarios…");
    results.insert("compose_2k_ms".to_string(), compose_2k());
    results.insert("bake_ao_256_ms".to_string(), bake_ao_256());
    results.insert("project_roundtrip_ms".to_string(), project_roundtrip());

    for (name, ms) in &results {
        println!("  {name}: {ms:.1} ms");
    }

    if let Err(err) = write_baseline(&results) {
        eprintln!("could not write {BASELINE}: {err}");
        return ExitCode::FAILURE;
    }
    println!("wrote {BASELINE}");

    let budgets = read_budgets();
    let mut over = false;
    for (name, limit) in &budgets.budgets {
        if let Some(ms) = results.get(name)
            && *ms > *limit
        {
            eprintln!("  OVER BUDGET: {name} = {ms:.1} ms (budget {limit:.0} ms)");
            over = true;
        }
    }
    if over && budgets.enforce {
        eprintln!("perf: budgets exceeded and enforce = true");
        ExitCode::FAILURE
    } else {
        println!("perf: ok ({} budgets, enforce = {})", budgets.budgets.len(), budgets.enforce);
        ExitCode::SUCCESS
    }
}

fn demo_project(resolution: u32) -> substancecraft_doc::Project {
    let mesh = substancecraft_geom::primitives::uv_sphere(1.0, 64, 96).expect("sphere");
    let mut project = substancecraft_doc::Project::from_scene("perf", substancecraft_doc::Scene::from_mesh(mesh));
    if let Some(set) = project.texture_sets.first_mut() {
        set.resolution = resolution;
        set.layers.push(substancecraft_doc::Layer::fill("base", [0.5, 0.2, 0.1, 1.0], 0.3));
        set.layers.push(substancecraft_doc::Layer::fill("detail", [0.2, 0.4, 0.8, 1.0], 0.6));
    }
    project
}

/// Composite a six-channel texture set at 2048².
fn compose_2k() -> f64 {
    let project = demo_project(2048);
    let set = &project.texture_sets[0];
    let material = project.scene.materials.first().cloned().unwrap_or_default();
    let start = Instant::now();
    let images = substancecraft_compose::evaluate_set(set, &material, 2048, 2048);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    std::hint::black_box(images.len());
    elapsed
}

/// Bake ambient occlusion at 256² with 16 samples.
fn bake_ao_256() -> f64 {
    let mesh = substancecraft_geom::primitives::uv_sphere(1.0, 64, 96).expect("sphere");
    let baker = substancecraft_bake::Baker::new(&mesh, None);
    let settings = substancecraft_bake::BakeSettings { resolution: 256, samples: 16, ..Default::default() };
    let start = Instant::now();
    let image = baker.bake(substancecraft_bake::BakeMap::AmbientOcclusion, &settings);
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    std::hint::black_box(image.width);
    elapsed
}

/// Round-trip a demo project through the native format.
fn project_roundtrip() -> f64 {
    let project = demo_project(2048);
    let start = Instant::now();
    let bytes = substancecraft_format::to_bytes(&project).expect("encode");
    let decoded = substancecraft_format::from_bytes(&bytes).expect("decode");
    let elapsed = start.elapsed().as_secs_f64() * 1000.0;
    std::hint::black_box(decoded.texture_sets.len());
    elapsed
}

fn write_baseline(results: &BTreeMap<String, f64>) -> std::io::Result<()> {
    if let Some(parent) = Path::new(BASELINE).parent() {
        std::fs::create_dir_all(parent)?;
    }
    let value = serde_json::json!({ "scenarios": results });
    std::fs::write(BASELINE, serde_json::to_string_pretty(&value).unwrap_or_default())
}

/// The measured baseline, if one has been recorded.
pub(crate) fn read_baseline() -> BTreeMap<String, f64> {
    let Ok(text) = std::fs::read_to_string(BASELINE) else { return BTreeMap::new() };
    let value: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    value
        .get("scenarios")
        .and_then(serde_json::Value::as_object)
        .map(|scenarios| scenarios.iter().filter_map(|(name, value)| value.as_f64().map(|ms| (name.clone(), ms))).collect())
        .unwrap_or_default()
}

pub(crate) struct Budgets {
    pub(crate) enforce: bool,
    pub(crate) budgets: BTreeMap<String, f64>,
}

/// A minimal parser for `perf/budgets.toml` (top-level `enforce` and a `[budgets]` table).
pub(crate) fn read_budgets() -> Budgets {
    let mut budgets = Budgets { enforce: false, budgets: BTreeMap::new() };
    let Ok(text) = std::fs::read_to_string(BUDGETS) else { return budgets };
    let mut in_budgets = false;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        if line == "[budgets]" {
            in_budgets = true;
            continue;
        }
        if line.starts_with('[') {
            in_budgets = false;
            continue;
        }
        let Some((key, value)) = line.split_once('=') else { continue };
        let key = key.trim();
        let value = value.trim();
        if in_budgets {
            if let Ok(parsed) = value.parse::<f64>() {
                budgets.budgets.insert(key.to_string(), parsed);
            }
        } else if key == "enforce" {
            budgets.enforce = value == "true";
        }
    }
    budgets
}
