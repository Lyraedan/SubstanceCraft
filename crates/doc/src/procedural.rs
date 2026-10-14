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

    /// The generator's editable parameters (for editors and the control channel).
    pub fn params(self) -> Vec<Param> {
        match self {
            Generator::Perlin { scale, octaves, seed } | Generator::Simplex { scale, octaves, seed } => vec![
                Param::float("scale", "Scale", scale, 1.0, 256.0),
                Param::int("octaves", "Octaves", octaves, 1, 8),
                Param::int("seed", "Seed", seed, 0, 999_999),
            ],
            Generator::Voronoi { cells, seed } => {
                vec![Param::float("cells", "Cells", cells, 1.0, 64.0), Param::int("seed", "Seed", seed, 0, 999_999)]
            }
            Generator::White { seed } => vec![Param::int("seed", "Seed", seed, 0, 999_999)],
        }
    }

    /// Return a copy with one parameter set (clamped to its range). Unknown keys leave it unchanged.
    pub fn set_param(self, key: &str, value: f32) -> Self {
        let Some(param) = self.params().into_iter().find(|param| param.key == key) else {
            return self;
        };
        let value = param.clamped(value);
        match self {
            Generator::Perlin { scale, octaves, seed } => Generator::Perlin {
                scale: pick(key, "scale", value, scale),
                octaves: pick_int(key, "octaves", value, octaves),
                seed: pick_int(key, "seed", value, seed),
            },
            Generator::Simplex { scale, octaves, seed } => Generator::Simplex {
                scale: pick(key, "scale", value, scale),
                octaves: pick_int(key, "octaves", value, octaves),
                seed: pick_int(key, "seed", value, seed),
            },
            Generator::Voronoi { cells, seed } => {
                Generator::Voronoi { cells: pick(key, "cells", value, cells), seed: pick_int(key, "seed", value, seed) }
            }
            Generator::White { seed } => Generator::White { seed: pick_int(key, "seed", value, seed) },
        }
    }
}

/// A generator or filter parameter, exposed for editors and the control channel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Param {
    /// Stable key used by `layer.set_effect`.
    pub key: &'static str,
    /// A label for the UI.
    pub label: &'static str,
    /// The current value.
    pub value: f32,
    /// Inclusive minimum.
    pub min: f32,
    /// Inclusive maximum.
    pub max: f32,
    /// Whether the value is integral (shown as a whole number).
    pub integer: bool,
}

impl Param {
    fn float(key: &'static str, label: &'static str, value: f32, min: f32, max: f32) -> Self {
        Self { key, label, value, min, max, integer: false }
    }

    fn int(key: &'static str, label: &'static str, value: u32, min: u32, max: u32) -> Self {
        Self { key, label, value: value as f32, min: min as f32, max: max as f32, integer: true }
    }

    /// The value clamped to the parameter's inclusive range.
    pub fn clamped(self, value: f32) -> f32 {
        value.clamp(self.min, self.max)
    }
}

fn pick(key: &str, want: &str, value: f32, current: f32) -> f32 {
    if key == want { value } else { current }
}

fn pick_int(key: &str, want: &str, value: f32, current: u32) -> u32 {
    if key == want { value.round().max(0.0) as u32 } else { current }
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

    /// The filter's editable parameters (for editors and the control channel).
    pub fn params(self) -> Vec<Param> {
        match self {
            Filter::Levels { in_black, in_white, gamma, out_black, out_white } => vec![
                Param::float("in_black", "In Black", in_black, 0.0, 1.0),
                Param::float("in_white", "In White", in_white, 0.0, 1.0),
                Param::float("gamma", "Gamma", gamma, 0.01, 10.0),
                Param::float("out_black", "Out Black", out_black, 0.0, 1.0),
                Param::float("out_white", "Out White", out_white, 0.0, 1.0),
            ],
            Filter::Blur { radius } => vec![Param::int("radius", "Radius", radius, 1, 64)],
            Filter::Invert => Vec::new(),
            Filter::Contrast { amount } => vec![Param::float("amount", "Amount", amount, 0.0, 4.0)],
            Filter::Brightness { amount } => vec![Param::float("amount", "Amount", amount, -1.0, 1.0)],
        }
    }

    /// Return a copy with one parameter set (clamped to its range). Unknown keys leave it unchanged.
    pub fn set_param(self, key: &str, value: f32) -> Self {
        let Some(param) = self.params().into_iter().find(|param| param.key == key) else {
            return self;
        };
        let value = param.clamped(value);
        match self {
            Filter::Levels { in_black, in_white, gamma, out_black, out_white } => Filter::Levels {
                in_black: pick(key, "in_black", value, in_black),
                in_white: pick(key, "in_white", value, in_white),
                gamma: pick(key, "gamma", value, gamma),
                out_black: pick(key, "out_black", value, out_black),
                out_white: pick(key, "out_white", value, out_white),
            },
            Filter::Blur { radius } => Filter::Blur { radius: pick_int(key, "radius", value, radius) },
            Filter::Invert => Filter::Invert,
            Filter::Contrast { amount } => Filter::Contrast { amount: pick(key, "amount", value, amount) },
            Filter::Brightness { amount } => Filter::Brightness { amount: pick(key, "amount", value, amount) },
        }
    }
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

    /// The effect's editable parameters.
    pub fn params(self) -> Vec<Param> {
        match self {
            Effect::Generator(generator) => generator.params(),
            Effect::Filter(filter) => filter.params(),
        }
    }

    /// Return a copy with one parameter set. Unknown keys leave it unchanged.
    pub fn set_param(self, key: &str, value: f32) -> Self {
        match self {
            Effect::Generator(generator) => Effect::Generator(generator.set_param(key, value)),
            Effect::Filter(filter) => Effect::Filter(filter.set_param(key, value)),
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

    #[test]
    fn params_expose_keys_and_ranges() {
        let perlin = Generator::default_perlin();
        let keys: Vec<&str> = perlin.params().iter().map(|param| param.key).collect();
        assert_eq!(keys, ["scale", "octaves", "seed"]);
        assert!(Generator::default_white().params().iter().all(|param| param.integer));
        assert!(Filter::Invert.params().is_empty());
    }

    #[test]
    fn set_param_clamps_and_ignores_unknown_keys() {
        let generator = Generator::default_perlin().set_param("scale", 1_000_000.0);
        assert_eq!(generator, Generator::Perlin { scale: 256.0, octaves: 4, seed: 0 });
        let generator = generator.set_param("octaves", 99.0);
        assert_eq!(generator, Generator::Perlin { scale: 256.0, octaves: 8, seed: 0 });
        let unchanged = generator.set_param("nope", 1.0);
        assert_eq!(unchanged, generator);
    }

    #[test]
    fn effect_set_param_routes_to_the_member() {
        let effect = Effect::Generator(Generator::default_voronoi()).set_param("cells", 0.0);
        assert_eq!(effect, Effect::Generator(Generator::Voronoi { cells: 1.0, seed: 0 }));
        let filter = Effect::Filter(Filter::default_contrast()).set_param("amount", 10.0);
        assert_eq!(filter, Effect::Filter(Filter::Contrast { amount: 4.0 }));
    }
}
