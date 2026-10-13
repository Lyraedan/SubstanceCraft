//! Procedural content descriptions (pure data).
//!
//! These are the *parameters* of a generator or filter. The algorithms that evaluate them live in
//! `substancecraft-material`; keeping the data here lets the layer model describe an effect without
//! depending on the imaging code.

use serde::{Deserialize, Serialize};

/// A procedural texture generator.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Generator {
    /// Fractal gradient (Perlin) noise.
    Perlin {
        /// Base frequency.
        scale: f32,
        /// Number of octaves.
        octaves: u32,
        /// Random seed.
        seed: u32,
    },
    /// Fractal simplex noise.
    Simplex {
        /// Base frequency.
        scale: f32,
        /// Number of octaves.
        octaves: u32,
        /// Random seed.
        seed: u32,
    },
    /// Worley / cellular noise (F1 distance).
    Voronoi {
        /// Cell density.
        cells: f32,
        /// Random seed.
        seed: u32,
    },
    /// White noise.
    White {
        /// Random seed.
        seed: u32,
    },
}

impl Generator {
    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            Generator::Perlin { .. } => "perlin",
            Generator::Simplex { .. } => "simplex",
            Generator::Voronoi { .. } => "voronoi",
            Generator::White { .. } => "white",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Generator::Perlin { .. } => "Perlin Noise",
            Generator::Simplex { .. } => "Simplex Noise",
            Generator::Voronoi { .. } => "Voronoi",
            Generator::White { .. } => "White Noise",
        }
    }

    /// Parse a generator id into a default-parametered generator.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "perlin" => Generator::default_perlin(),
            "simplex" => Generator::default_simplex(),
            "voronoi" => Generator::default_voronoi(),
            "white" => Generator::default_white(),
            _ => return None,
        })
    }

    /// The default Perlin noise.
    pub fn default_perlin() -> Self {
        Generator::Perlin { scale: 8.0, octaves: 4, seed: 0 }
    }

    /// The default simplex noise.
    pub fn default_simplex() -> Self {
        Generator::Simplex { scale: 8.0, octaves: 4, seed: 0 }
    }

    /// The default Voronoi noise.
    pub fn default_voronoi() -> Self {
        Generator::Voronoi { cells: 8.0, seed: 0 }
    }

    /// The default white noise.
    pub fn default_white() -> Self {
        Generator::White { seed: 0 }
    }

    /// Every generator id, in UI order.
    pub const IDS: [&'static str; 4] = ["perlin", "simplex", "voronoi", "white"];
}

/// A filter applied to the pixels below it.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Filter {
    /// Input/output level remap with gamma.
    Levels {
        /// Input black point.
        in_black: f32,
        /// Input white point.
        in_white: f32,
        /// Gamma.
        gamma: f32,
        /// Output black point.
        out_black: f32,
        /// Output white point.
        out_white: f32,
    },
    /// A separable box blur.
    Blur {
        /// Radius in pixels.
        radius: u32,
    },
    /// Invert the colour channels.
    Invert,
    /// Contrast around mid-grey.
    Contrast {
        /// Multiplier (1 = unchanged).
        amount: f32,
    },
    /// Additive brightness.
    Brightness {
        /// Offset (-1 = black, 1 = white).
        amount: f32,
    },
}

impl Filter {
    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            Filter::Levels { .. } => "levels",
            Filter::Blur { .. } => "blur",
            Filter::Invert => "invert",
            Filter::Contrast { .. } => "contrast",
            Filter::Brightness { .. } => "brightness",
        }
    }

    /// A human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Filter::Levels { .. } => "Levels",
            Filter::Blur { .. } => "Blur",
            Filter::Invert => "Invert",
            Filter::Contrast { .. } => "Contrast",
            Filter::Brightness { .. } => "Brightness",
        }
    }

    /// Parse a filter id into a default-parametered filter.
    pub fn from_id(id: &str) -> Option<Self> {
        Some(match id {
            "levels" => Filter::default_levels(),
            "blur" => Filter::default_blur(),
            "invert" => Filter::Invert,
            "contrast" => Filter::default_contrast(),
            "brightness" => Filter::default_brightness(),
            _ => return None,
        })
    }

    /// The default levels.
    pub fn default_levels() -> Self {
        Filter::Levels { in_black: 0.0, in_white: 1.0, gamma: 1.0, out_black: 0.0, out_white: 1.0 }
    }

    /// The default blur.
    pub fn default_blur() -> Self {
        Filter::Blur { radius: 4 }
    }

    /// The default contrast.
    pub fn default_contrast() -> Self {
        Filter::Contrast { amount: 1.0 }
    }

    /// The default brightness.
    pub fn default_brightness() -> Self {
        Filter::Brightness { amount: 0.0 }
    }

    /// Every filter id, in UI order.
    pub const IDS: [&'static str; 5] = ["levels", "blur", "invert", "contrast", "brightness"];
}

/// An effect: either a generator (fills) or a filter (transforms the pixels below).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Effect {
    /// A generated texture.
    Generator(Generator),
    /// A filter over the pixels below.
    Filter(Filter),
}

impl Effect {
    /// A stable id.
    pub fn id(self) -> &'static str {
        match self {
            Effect::Generator(generator) => generator.id(),
            Effect::Filter(filter) => filter.id(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generator_ids_round_trip() {
        for id in Generator::IDS {
            let generator = Generator::from_id(id).expect("generator");
            assert_eq!(generator.id(), id);
        }
        assert_eq!(Generator::from_id("nope"), None);
    }

    #[test]
    fn filter_ids_round_trip() {
        for id in Filter::IDS {
            let filter = Filter::from_id(id).expect("filter");
            assert_eq!(filter.id(), id);
        }
        assert_eq!(Filter::from_id("nope"), None);
    }

    #[test]
    fn effect_id_matches_member() {
        assert_eq!(Effect::Generator(Generator::default_perlin()).id(), "perlin");
        assert_eq!(Effect::Filter(Filter::Invert).id(), "invert");
    }
}
