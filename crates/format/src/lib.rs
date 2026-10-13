//! SubstanceCraft native project format (`.sccraft`).
//!
//! Serialises the whole [`Project`] — scene, texture sets, layer stacks, shader and paint images —
//! as RON, written crash-safely through `craft-atomic`. A version header lets future readers reject
//! or migrate newer files instead of silently misreading them.

#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

use std::path::Path;

use serde::{Deserialize, Serialize};
use substancecraft_doc::Project;
use thiserror::Error;

/// The current format version.
pub const FORMAT_VERSION: u32 = 1;

/// The native file extension.
pub const EXTENSION: &str = "sccraft";

/// Errors from saving or loading a project.
#[derive(Debug, Error)]
pub enum FormatError {
    /// The operating system refused a read or write.
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    /// The file is not valid RON or does not match the schema.
    #[error("could not parse project: {0}")]
    Parse(String),
    /// The project could not be encoded.
    #[error("could not encode project: {0}")]
    Encode(String),
    /// The file was written by a newer, unsupported format version.
    #[error("unsupported project version {0}")]
    Version(u32),
}

/// The on-disk document: a version header plus the project.
#[derive(Serialize, Deserialize)]
struct Document {
    version: u32,
    project: Project,
}

/// Serialise a project to RON bytes.
pub fn to_bytes(project: &Project) -> Result<Vec<u8>, FormatError> {
    let document = Document { version: FORMAT_VERSION, project: project.clone() };
    let text = ron::ser::to_string_pretty(&document, ron::ser::PrettyConfig::default()).map_err(|err| FormatError::Encode(err.to_string()))?;
    Ok(text.into_bytes())
}

/// Parse a project from RON bytes.
pub fn from_bytes(bytes: &[u8]) -> Result<Project, FormatError> {
    let document: Document = ron::de::from_bytes(bytes).map_err(|err| FormatError::Parse(err.to_string()))?;
    if document.version > FORMAT_VERSION {
        return Err(FormatError::Version(document.version));
    }
    Ok(document.project)
}

/// Save a project to a `.sccraft` file, crash-safely.
pub fn save(project: &Project, path: impl AsRef<Path>) -> Result<(), FormatError> {
    let bytes = to_bytes(project)?;
    craft_atomic::atomic_write(path.as_ref(), &bytes)?;
    Ok(())
}

/// Load a project from a `.sccraft` file.
pub fn load(path: impl AsRef<Path>) -> Result<Project, FormatError> {
    let bytes = std::fs::read(path.as_ref())?;
    from_bytes(&bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use substancecraft_doc::{Effect, Filter, Layer, Scene, Shader};

    fn populated() -> Project {
        let mesh = substancecraft_geom::primitives::uv_sphere(1.0, 6, 8).expect("sphere");
        let mut project = Project::from_scene("hero", Scene::from_mesh(mesh));
        let set = &mut project.texture_sets[0];
        set.layers.push(Layer::fill("base", [0.5, 0.2, 0.1, 1.0], 0.4));
        set.layers.push(Layer::effect("grunge", Effect::Filter(Filter::Invert)));
        set.shader = Shader::MetalRough;
        set.shader_params.metallic = 0.8;
        set.shader_params.roughness = 0.3;
        set.shader_params.base_color_tint = [0.9, 1.0, 0.8];
        project
    }

    #[test]
    fn round_trips_through_bytes() {
        let project = populated();
        let bytes = to_bytes(&project).expect("encode");
        let back = from_bytes(&bytes).expect("decode");
        assert_eq!(project, back);
    }

    #[test]
    fn round_trips_through_a_file() {
        let project = populated();
        let path = std::env::temp_dir().join(format!("sc_format_{}.{EXTENSION}", std::process::id()));
        save(&project, &path).expect("save");
        let back = load(&path).expect("load");
        assert_eq!(project, back);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn newer_versions_are_rejected() {
        let project = populated();
        let mut text = String::from_utf8(to_bytes(&project).expect("encode")).expect("utf8");
        // Rewrite the version to something newer.
        text = text.replace("version: 1", "version: 999");
        assert!(matches!(from_bytes(text.as_bytes()), Err(FormatError::Version(999))));
    }

    #[test]
    fn garbage_is_an_error_not_a_panic() {
        assert!(matches!(from_bytes(b"not a project"), Err(FormatError::Parse(_))));
    }

    #[test]
    fn empty_texture_set_list_still_loads() {
        let project = Project::new("empty");
        let bytes = to_bytes(&project).expect("encode");
        let back = from_bytes(&bytes).expect("decode");
        assert_eq!(back.texture_sets.len(), 1);
    }
}
