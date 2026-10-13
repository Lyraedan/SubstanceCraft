//! Crate layering enforcement.
//!
//! A crate may depend only on crates at the same or a lower layer, and nothing below the UI
//! layer may use an egui/winit/native-dialog crate. Each new crate must be registered here.

use std::collections::BTreeMap;
use std::process::{Command, ExitCode};

/// The layer of every workspace crate. Apps and xtask are not layered.
pub const LAYERS: &[(&str, u8)] = &[
    ("substancecraft-geom", 0),
    ("substancecraft-color", 0),
    ("substancecraft-raster", 0),
    ("substancecraft-codecs", 0),
    ("substancecraft-tablet", 0),
    ("substancecraft-doc", 1),
    ("substancecraft-format", 1),
    ("substancecraft-algo", 2),
    ("substancecraft-paint", 2),
    ("substancecraft-material", 2),
    ("substancecraft-bake", 2),
    ("substancecraft-compose", 3),
    ("substancecraft-gpu", 3),
    ("substancecraft-render", 3),
    ("substancecraft-io", 4),
    ("substancecraft-engine", 5),
    ("substancecraft-ui", 6),
    ("substancecraft-automation", 6),
];

/// Crates only the UI layer (and above) may depend on.
const UI_CRATES: &[&str] = &["egui", "eframe", "egui-wgpu", "egui-winit", "egui_extras", "winit", "rfd", "craft-ui", "craft-quit", "craft-settings"];

/// The UI layer number.
const UI_LAYER: u8 = 6;

/// Verify layering and return a process exit code.
pub fn run() -> ExitCode {
    let metadata = match cargo_metadata() {
        Ok(metadata) => metadata,
        Err(err) => {
            eprintln!("layers: {err}");
            return ExitCode::FAILURE;
        }
    };

    let layer_of: BTreeMap<&str, u8> = LAYERS.iter().copied().collect();
    let mut violations: Vec<String> = Vec::new();
    let mut seen: BTreeMap<&str, ()> = BTreeMap::new();

    let packages = metadata["packages"].as_array().cloned().unwrap_or_default();
    for package in &packages {
        let Some(name) = package["name"].as_str() else { continue };
        let Some(&layer) = layer_of.get(name) else { continue };
        seen.insert(name, ());

        let dependencies = package["dependencies"].as_array().cloned().unwrap_or_default();
        for dependency in &dependencies {
            let Some(dep_name) = dependency["name"].as_str() else { continue };

            if let Some(&dep_layer) = layer_of.get(dep_name)
                && dep_layer > layer
            {
                violations.push(format!("{name} (L{layer}) depends on {dep_name} (L{dep_layer}): a crate may only depend on lower layers"));
            }

            if layer < UI_LAYER && UI_CRATES.contains(&dep_name) {
                violations.push(format!(
                    "{name} (L{layer}) depends on the UI crate {dep_name}: only L{UI_LAYER} and above may use egui/winit/native-dialog crates"
                ));
            }
        }
    }

    for &(name, _) in LAYERS {
        if !seen.contains_key(name) {
            violations.push(format!("{name} is registered in xtask::layers but not a workspace member"));
        }
    }

    if violations.is_empty() {
        println!("layers: ok — {} crates registered", LAYERS.len());
        ExitCode::SUCCESS
    } else {
        for violation in &violations {
            eprintln!("layers: {violation}");
        }
        ExitCode::FAILURE
    }
}

fn cargo_metadata() -> Result<serde_json::Value, String> {
    let output = Command::new("cargo")
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .output()
        .map_err(|err| format!("could not run cargo metadata: {err}"))?;

    if !output.status.success() {
        return Err("cargo metadata failed".to_string());
    }

    serde_json::from_slice(&output.stdout).map_err(|err| format!("could not parse cargo metadata: {err}"))
}
