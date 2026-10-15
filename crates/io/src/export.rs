//! Texture-set export: composite each channel and write files with presets, packing and naming.

use std::path::{Path, PathBuf};

use substancecraft_doc::{Channel, Material, TextureSet};
use substancecraft_raster::RasterImage;

use crate::IoError;

/// The output image format.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExportFormat {
    /// 8-bit PNG.
    Png8,
    /// 16-bit PNG.
    Png16,
    /// 8-bit TGA.
    Tga,
    /// 32-bit float EXR.
    Exr,
}

impl ExportFormat {
    /// Every format, in UI order.
    pub const ALL: [ExportFormat; 4] = [ExportFormat::Png8, ExportFormat::Png16, ExportFormat::Tga, ExportFormat::Exr];

    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            ExportFormat::Png8 => "png8",
            ExportFormat::Png16 => "png16",
            ExportFormat::Tga => "tga",
            ExportFormat::Exr => "exr",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            ExportFormat::Png8 => "PNG 8-bit",
            ExportFormat::Png16 => "PNG 16-bit",
            ExportFormat::Tga => "TGA 8-bit",
            ExportFormat::Exr => "EXR 32-bit",
        }
    }

    /// Parse a format from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        ExportFormat::ALL.into_iter().find(|format| format.id() == id)
    }

    /// The file extension (without the dot).
    pub fn extension(self) -> &'static str {
        match self {
            ExportFormat::Png8 | ExportFormat::Png16 => "png",
            ExportFormat::Tga => "tga",
            ExportFormat::Exr => "exr",
        }
    }
}

/// A packed output map: source channels for R/G/B/A (`None` writes a neutral 1.0).
#[derive(Clone, Debug, PartialEq)]
pub struct Packing {
    /// The output name pattern (may contain `$textureSet`/`$udim`).
    pub name: String,
    /// Red source channel.
    pub r: Option<Channel>,
    /// Green source channel.
    pub g: Option<Channel>,
    /// Blue source channel.
    pub b: Option<Channel>,
    /// Alpha source channel.
    pub a: Option<Channel>,
}

/// A named export configuration.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportPreset {
    /// Stable id.
    pub id: String,
    /// Human-readable label.
    pub label: String,
    /// Output format.
    pub format: ExportFormat,
    /// Channels to export individually.
    pub channels: Vec<Channel>,
    /// Packed outputs.
    pub packings: Vec<Packing>,
}

/// Export settings.
#[derive(Clone, Debug)]
pub struct ExportSettings {
    /// Output resolution in pixels.
    pub resolution: u32,
    /// Output format (overrides the preset's format when set).
    pub format: Option<ExportFormat>,
    /// Naming template (`$textureSet`, `$channel`, `$udim`). Empty uses `$textureSet_$channel`.
    pub naming: String,
    /// The preset id.
    pub preset: String,
    /// Whether to emit one file per UV tile (`$udim` appended).
    pub udim: bool,
    /// Bake the height channel into the exported normal map.
    pub height_to_normal: bool,
    /// Strength of the height-into-normal perturbation.
    pub normal_strength: f32,
    /// Write DirectX-style normals (green flipped) instead of OpenGL.
    pub directx_normal: bool,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            resolution: 2048,
            format: None,
            naming: String::new(),
            preset: "openpbr".to_string(),
            udim: false,
            height_to_normal: false,
            normal_strength: 1.0,
            directx_normal: false,
        }
    }
}

/// The built-in export presets.
pub fn presets() -> Vec<ExportPreset> {
    vec![
        ExportPreset {
            id: "openpbr".to_string(),
            label: "OpenPBR".to_string(),
            format: ExportFormat::Png8,
            channels: Channel::ALL.to_vec(),
            packings: Vec::new(),
        },
        ExportPreset {
            id: "gltf".to_string(),
            label: "glTF".to_string(),
            format: ExportFormat::Png8,
            channels: vec![Channel::BaseColor, Channel::Normal, Channel::Emissive],
            packings: vec![
                Packing {
                    name: "$textureSet_metallicRoughness".to_string(),
                    r: None,
                    g: Some(Channel::Roughness),
                    b: Some(Channel::Metallic),
                    a: None,
                },
                Packing { name: "$textureSet_occlusion".to_string(), r: Some(Channel::AmbientOcclusion), g: None, b: None, a: None },
            ],
        },
        ExportPreset {
            id: "unreal".to_string(),
            label: "Unreal".to_string(),
            format: ExportFormat::Png8,
            channels: vec![Channel::BaseColor, Channel::Normal],
            packings: vec![Packing {
                name: "$textureSet_ORM".to_string(),
                r: Some(Channel::AmbientOcclusion),
                g: Some(Channel::Roughness),
                b: Some(Channel::Metallic),
                a: None,
            }],
        },
        ExportPreset {
            id: "unity".to_string(),
            label: "Unity".to_string(),
            format: ExportFormat::Png8,
            channels: vec![Channel::BaseColor, Channel::Normal],
            packings: vec![Packing {
                name: "$textureSet_Mask".to_string(),
                r: Some(Channel::Metallic),
                g: Some(Channel::Roughness),
                b: Some(Channel::AmbientOcclusion),
                a: None,
            }],
        },
        ExportPreset {
            id: "specgloss".to_string(),
            label: "Spec/Gloss".to_string(),
            format: ExportFormat::Png8,
            channels: vec![Channel::BaseColor, Channel::Normal, Channel::Roughness],
            packings: Vec::new(),
        },
    ]
}

/// Look up a preset by id.
pub fn preset(id: &str) -> Option<ExportPreset> {
    presets().into_iter().find(|preset| preset.id == id)
}

/// Export a texture set to `dir`, returning the written paths.
pub fn export_set(set: &TextureSet, material: &Material, settings: &ExportSettings, dir: &Path) -> Result<Vec<PathBuf>, IoError> {
    export_set_ctx(set, material, settings, dir, &substancecraft_compose::EvalContext::default())
}

/// Export a texture set to `dir`, using `context` for projections and mesh-map fills.
pub fn export_set_ctx(
    set: &TextureSet,
    material: &Material,
    settings: &ExportSettings,
    dir: &Path,
    context: &substancecraft_compose::EvalContext<'_>,
) -> Result<Vec<PathBuf>, IoError> {
    let preset = preset(&settings.preset).or_else(|| preset("openpbr")).ok_or_else(|| IoError::UnsupportedFormat(settings.preset.clone()))?;
    let format = settings.format.unwrap_or(preset.format);
    let resolution = settings.resolution.clamp(16, 8192);
    let mut channels = substancecraft_compose::evaluate_set_ctx(set, material, resolution, resolution, context);

    // Optionally bake the height channel into the normal map, then flip for DirectX.
    if settings.height_to_normal
        && let Some(height) = channels.iter().find(|(channel, _)| *channel == Channel::Height).map(|(_, image)| image.clone())
    {
        let normal = channels.iter().find(|(channel, _)| *channel == Channel::Normal).map(|(_, image)| image.clone());
        let combined = height_to_normal(normal.as_ref(), &height, settings.normal_strength);
        match channels.iter_mut().find(|(channel, _)| *channel == Channel::Normal) {
            Some(slot) => slot.1 = combined,
            None => channels.push((Channel::Normal, combined)),
        }
    }
    if settings.directx_normal
        && let Some(slot) = channels.iter_mut().find(|(channel, _)| *channel == Channel::Normal)
    {
        slot.1 = flip_normal_green(&slot.1);
    }

    if let Err(err) = std::fs::create_dir_all(dir) {
        return Err(IoError::Io(err));
    }

    let naming = if settings.naming.trim().is_empty() { "$textureSet_$channel".to_string() } else { settings.naming.clone() };
    let tiles: Vec<Option<u32>> =
        if settings.udim && set.uv_tiles.len() > 1 { set.uv_tiles.iter().map(|tile| Some(tile.udim())).collect() } else { vec![None] };

    let mut written = Vec::new();
    for tile in tiles {
        for channel in &preset.channels {
            let Some((_, image)) = channels.iter().find(|(kind, _)| kind == channel) else { continue };
            let base = apply_naming(&naming, &set.name, Some(channel.id()), tile);
            let path = dir.join(format!("{base}.{}", format.extension()));
            write_image(image, format, &path)?;
            written.push(path);
        }
        for packing in &preset.packings {
            let Some(image) = pack(&channels, packing) else { continue };
            let base = apply_naming(&packing.name, &set.name, None, tile);
            let path = dir.join(format!("{base}.{}", format.extension()));
            write_image(&image, format, &path)?;
            written.push(path);
        }
    }
    Ok(written)
}

/// Replace naming tokens.
fn apply_naming(template: &str, texture_set: &str, channel: Option<&str>, udim: Option<u32>) -> String {
    let mut name = template.replace("$textureSet", texture_set).replace("$texture_set", texture_set);
    name = name.replace("$channel", channel.unwrap_or(""));
    if let Some(udim) = udim {
        name = name.replace("$udim", &udim.to_string());
    } else {
        name = name.replace("$udim", "");
    }
    name.trim_matches('_').to_string()
}

/// Build a packed image from the channel images.
fn pack(channels: &[(Channel, RasterImage)], packing: &Packing) -> Option<RasterImage> {
    let find = |channel: Option<Channel>| channel.and_then(|channel| channels.iter().find(|(kind, _)| *kind == channel).map(|(_, image)| image));
    let r = find(packing.r);
    let g = find(packing.g);
    let b = find(packing.b);
    let a = find(packing.a);
    let base = r.or(g).or(b).or(a)?;
    let (width, height) = (base.width, base.height);
    let mut image = RasterImage::filled(width, height, [1.0, 1.0, 1.0, 1.0]);
    for y in 0..height {
        for x in 0..width {
            let sample = |source: Option<&RasterImage>| source.map(|image| image.pixel(x, y)[0]).unwrap_or(1.0);
            image.set_pixel(x, y, [sample(r), sample(g), sample(b), sample(a)]);
        }
    }
    Some(image)
}

/// Bake a height map into a tangent-space normal map (finite-difference gradient).
fn height_to_normal(normal: Option<&RasterImage>, height: &RasterImage, strength: f32) -> RasterImage {
    let (width, height_px) = (height.width, height.height);
    let mut out = RasterImage::filled(width, height_px, [0.5, 0.5, 1.0, 1.0]);
    if width == 0 || height_px == 0 {
        return out;
    }
    let sample = |x: i64, y: i64| -> f32 {
        let x = x.rem_euclid(width as i64) as u32;
        let y = y.rem_euclid(height_px as i64) as u32;
        height.pixel(x, y)[0]
    };
    for y in 0..height_px {
        for x in 0..width {
            let dhdu = (sample(x as i64 + 1, y as i64) - sample(x as i64 - 1, y as i64)) * 0.5;
            let dhdv = (sample(x as i64, y as i64 + 1) - sample(x as i64, y as i64 - 1)) * 0.5;
            let base = normal
                .map(|image| {
                    let pixel = image.pixel(x, y);
                    [pixel[0] * 2.0 - 1.0, pixel[1] * 2.0 - 1.0, pixel[2] * 2.0 - 1.0]
                })
                .unwrap_or([0.0, 0.0, 1.0]);
            let nx = base[0] - dhdu * strength;
            let ny = base[1] - dhdv * strength;
            let nz = base[2].max(1e-3);
            let length = (nx * nx + ny * ny + nz * nz).sqrt().max(1e-6);
            out.set_pixel(x, y, [(nx / length) * 0.5 + 0.5, (ny / length) * 0.5 + 0.5, (nz / length) * 0.5 + 0.5, 1.0]);
        }
    }
    out
}

/// Flip a normal map's green channel (OpenGL ↔ DirectX).
fn flip_normal_green(image: &RasterImage) -> RasterImage {
    let mut out = image.clone();
    for pixel in &mut out.pixels {
        pixel[1] = 1.0 - pixel[1];
    }
    out
}

/// Write an image in the requested format.
fn write_image(image: &RasterImage, format: ExportFormat, path: &Path) -> Result<(), IoError> {
    match format {
        ExportFormat::Png8 => substancecraft_codecs::save_png(path, image.width, image.height, &image.to_rgba8_linear())?,
        ExportFormat::Png16 => substancecraft_codecs::save_png16(path, image.width, image.height, &to_u16(image))?,
        ExportFormat::Tga => substancecraft_codecs::save_tga(path, image.width, image.height, &image.to_rgba8_linear())?,
        ExportFormat::Exr => substancecraft_codecs::save_exr(path, image.width, image.height, &to_f32(image))?,
    }
    Ok(())
}

fn to_u16(image: &RasterImage) -> Vec<u16> {
    let mut out = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        for channel in pixel {
            out.push((channel.clamp(0.0, 1.0) * 65535.0 + 0.5) as u16);
        }
    }
    out
}

fn to_f32(image: &RasterImage) -> Vec<f32> {
    let mut out = Vec::with_capacity(image.pixels.len() * 4);
    for pixel in &image.pixels {
        out.extend_from_slice(pixel);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use substancecraft_doc::{Layer, Material, TextureSet};

    fn textured_set() -> TextureSet {
        let mut set = TextureSet::new("body", vec![0]);
        set.layers.push(Layer::fill("base", [0.8, 0.1, 0.1, 1.0], 0.7));
        set
    }

    #[test]
    fn openpbr_exports_channels_and_round_trips() {
        let set = textured_set();
        let material = Material::default();
        let dir = std::env::temp_dir().join(format!("sc_export_{}", std::process::id()));
        let settings = ExportSettings { resolution: 8, preset: "openpbr".to_string(), ..Default::default() };
        let files = export_set(&set, &material, &settings, &dir).expect("export");
        assert!(!files.is_empty());

        let base_path = dir.join("body_base_color.png");
        assert!(base_path.exists(), "base color not written");
        // The base color channel is a red fill at 0.8/0.1/0.1.
        let loaded = substancecraft_codecs::load_rgba8(&base_path).expect("load");
        assert!((loaded.pixel(0, 0)[0] - 0.8).abs() < 2.0 / 255.0, "round-trip red mismatch: {:?}", loaded.pixel(0, 0));
        assert!((loaded.pixel(0, 0)[1] - 0.1).abs() < 2.0 / 255.0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreal_packs_orm() {
        let set = textured_set();
        let material = Material::default();
        let dir = std::env::temp_dir().join(format!("sc_export_pack_{}", std::process::id()));
        let settings = ExportSettings { resolution: 4, preset: "unreal".to_string(), ..Default::default() };
        let files = export_set(&set, &material, &settings, &dir).expect("export");
        let names: HashSet<String> = files.iter().map(|p| p.file_name().unwrap_or_default().to_string_lossy().to_string()).collect();
        assert!(names.contains("body_ORM.png"));
        // The fill sets every scalar channel to 0.7, so R (AO), G (roughness) and B (metallic)
        // all read 0.7 in the packed map.
        let orm = substancecraft_codecs::load_rgba8(dir.join("body_ORM.png")).expect("load");
        let pixel = orm.pixel(0, 0);
        for (index, label) in [(0, "AO in red"), (1, "roughness in green"), (2, "metallic in blue")] {
            assert!((pixel[index] - 0.7).abs() < 2.0 / 255.0, "{label}: {pixel:?}");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn naming_and_formats_round_trip() {
        assert_eq!(ExportFormat::from_id("exr"), Some(ExportFormat::Exr));
        assert_eq!(ExportFormat::from_id("nope"), None);
        assert_eq!(apply_naming("$textureSet_$channel", "body", Some("normal"), None), "body_normal");
        assert_eq!(apply_naming("$textureSet_$channel", "body", Some("normal"), Some(1002)), "body_normal");
        assert_eq!(apply_naming("$textureSet_$udim", "skin", None, Some(1001)), "skin_1001");
    }

    #[test]
    fn every_preset_has_a_known_id() {
        for entry in presets() {
            assert_eq!(preset(&entry.id).map(|found| found.id), Some(entry.id.clone()));
        }
    }

    #[test]
    fn height_bakes_into_normals_and_directx_flips_green() {
        let flat = RasterImage::filled(4, 4, [0.5, 0.5, 0.5, 1.0]);
        let normal = height_to_normal(None, &flat, 1.0);
        assert!((normal.pixel(0, 0)[2] - 1.0).abs() < 1e-3, "flat height should stay flat");

        let mut ramp = RasterImage::filled(4, 4, [0.5, 0.5, 0.5, 1.0]);
        for x in 0..4u32 {
            for y in 0..4u32 {
                let v = x as f32 / 3.0;
                ramp.set_pixel(x, y, [v, v, v, 1.0]);
            }
        }
        let tilted = height_to_normal(None, &ramp, 1.0);
        assert!((tilted.pixel(2, 2)[0] - 0.5).abs() > 0.01, "a height ramp should tilt the normal");

        let flipped = flip_normal_green(&normal);
        assert!((flipped.pixel(0, 0)[1] - (1.0 - normal.pixel(0, 0)[1])).abs() < 1e-6);
    }
}
