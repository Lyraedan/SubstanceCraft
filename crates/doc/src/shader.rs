//! Shader selection and parameters (pure data).
//!
//! A texture set is shaded with one shader at a time. The shader name and its parameters live here;
//! the viewport renderer reads them to shade the active set.

/// The shading model assigned to a texture set.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Shader {
    /// OpenPBR surface (the default).
    #[default]
    OpenPbr,
    /// Legacy metal/rough (glTF-style) shader.
    MetalRough,
    /// Legacy specular/glossiness shader.
    SpecGloss,
}

impl Shader {
    /// Every shader, in UI order.
    pub const ALL: [Shader; 3] = [Shader::OpenPbr, Shader::MetalRough, Shader::SpecGloss];

    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            Shader::OpenPbr => "openpbr",
            Shader::MetalRough => "metal_rough",
            Shader::SpecGloss => "spec_gloss",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Shader::OpenPbr => "OpenPBR",
            Shader::MetalRough => "Metal / Rough",
            Shader::SpecGloss => "Spec / Gloss",
        }
    }

    /// Parse a shader from its id.
    pub fn from_id(id: &str) -> Option<Self> {
        Shader::ALL.into_iter().find(|shader| shader.id() == id)
    }
}

/// Editable shader parameters. These override the imported material for the active set's viewport
/// shading.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShaderParams {
    /// Base-colour tint (multiplied into the composited base colour).
    pub base_color_tint: [f32; 3],
    /// Metalness in `0..=1`.
    pub metallic: f32,
    /// Roughness in `0..=1`.
    pub roughness: f32,
    /// Reflectance / specular level in `0..=1` (spec/gloss).
    pub specular: f32,
}

impl Default for ShaderParams {
    fn default() -> Self {
        Self { base_color_tint: [1.0, 1.0, 1.0], metallic: 0.1, roughness: 0.6, specular: 0.5 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shader_ids_round_trip() {
        for shader in Shader::ALL {
            assert_eq!(Shader::from_id(shader.id()), Some(shader));
        }
        assert_eq!(Shader::from_id("nope"), None);
    }

    #[test]
    fn defaults_are_sane() {
        let params = ShaderParams::default();
        assert!((0.0..=1.0).contains(&params.metallic) && (0.0..=1.0).contains(&params.roughness));
    }
}
