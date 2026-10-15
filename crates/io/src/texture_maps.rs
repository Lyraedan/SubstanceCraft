//! Match loose texture files to channels and import them as one material.
//!
//! Files are matched by name; glossiness/smoothness maps are inverted to roughness and DirectX
//! normal maps are converted to OpenGL (green flipped), so the imported images are ready to use.

use std::path::{Path, PathBuf};

use substancecraft_doc::Channel;
use substancecraft_raster::RasterImage;

use crate::IoError;

/// Guess the channel a texture file belongs to from its name, if any.
pub fn match_channel(name: &str) -> Option<Channel> {
    let stem = Path::new(name).file_stem().and_then(|stem| stem.to_str()).unwrap_or(name).to_ascii_lowercase();
    let has = |keys: &[&str]| keys.iter().any(|key| stem.contains(key));
    // Unreal-style suffixes: `_N` (normal), `_D` (diffuse), `_R` (roughness), `_M` (metallic).
    if stem.ends_with("_n") || stem.ends_with("_nrm") || stem.ends_with("_nor") {
        return Some(Channel::Normal);
    }
    if has(&["normal", "nrm", "nor"]) {
        return Some(Channel::Normal);
    }
    if has(&["rough", "gloss", "smooth"]) {
        return Some(Channel::Roughness);
    }
    if has(&["metal"]) {
        return Some(Channel::Metallic);
    }
    if has(&["height", "disp", "bump"]) {
        return Some(Channel::Height);
    }
    if has(&["occlusion", "ao"]) {
        return Some(Channel::AmbientOcclusion);
    }
    if has(&["emiss", "emit"]) {
        return Some(Channel::Emissive);
    }
    if has(&["basecolor", "base_color", "albedo", "diffuse", "diff", "color", "col", "bc"]) {
        return Some(Channel::BaseColor);
    }
    None
}

/// Load and match image files to channels (first file per channel wins). Gloss/smoothness maps are
/// inverted to roughness and DirectX normal maps are flipped to OpenGL.
pub fn import_texture_set(files: &[PathBuf]) -> Result<Vec<(Channel, RasterImage)>, IoError> {
    let mut matched: Vec<(Channel, RasterImage)> = Vec::new();
    for path in files {
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
        let Some(channel) = match_channel(name) else { continue };
        if matched.iter().any(|(existing, _)| *existing == channel) {
            continue;
        }
        let loaded = substancecraft_codecs::load_rgba8(path)?;
        if loaded.width == 0 || loaded.height == 0 {
            continue;
        }
        let mut image = RasterImage::new(loaded.width, loaded.height);
        for (slot, value) in image.pixels.iter_mut().zip(loaded.pixels) {
            *slot = value;
        }
        let lower = name.to_ascii_lowercase();
        if channel == Channel::Roughness && (lower.contains("gloss") || lower.contains("smooth")) {
            for pixel in &mut image.pixels {
                pixel[0] = 1.0 - pixel[0];
                pixel[1] = 1.0 - pixel[1];
                pixel[2] = 1.0 - pixel[2];
            }
        }
        if channel == Channel::Normal && (lower.contains("dx") || lower.contains("directx")) {
            for pixel in &mut image.pixels {
                pixel[1] = 1.0 - pixel[1];
            }
        }
        matched.push((channel, image));
    }
    Ok(matched)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_common_texture_names() {
        assert_eq!(match_channel("Rock_BaseColor.png"), Some(Channel::BaseColor));
        assert_eq!(match_channel("rock_base_color.png"), Some(Channel::BaseColor));
        assert_eq!(match_channel("T_Rock_N.png"), Some(Channel::Normal));
        assert_eq!(match_channel("material_roughness.png"), Some(Channel::Roughness));
        assert_eq!(match_channel("wall_glossiness.png"), Some(Channel::Roughness));
        assert_eq!(match_channel("crate_ao.png"), Some(Channel::AmbientOcclusion));
        assert_eq!(match_channel("preview.png"), None);
    }

    #[test]
    fn missing_files_are_an_error_not_a_panic() {
        let files = vec![PathBuf::from("/definitely/not/here_basecolor.png")];
        assert!(import_texture_set(&files).is_err());
    }
}
