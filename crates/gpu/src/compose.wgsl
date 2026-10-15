// SubstanceCraft GPU compositor pass (clean-room, WGSL).
//
// Reads the accumulated channel texture and blends one constant fill layer over it, matching the
// CPU oracle's per-channel blend + coverage model. The separable modes follow the W3C Compositing
// and Blending specification; the non-separable modes work on the full RGB triple.

struct Params {
    src: vec4<f32>,
    // x = coverage, y = blend mode, z = 1 when the channel is scalar
    control: vec4<f32>,
};

@group(0) @binding(0) var acc: texture_2d<f32>;
@group(0) @binding(1) var<uniform> params: Params;

fn color_dodge(b: f32, s: f32) -> f32 {
    if (b <= 0.0) {
        return 0.0;
    }
    if (s >= 1.0) {
        return 1.0;
    }
    return min(b / (1.0 - s), 1.0);
}

fn color_burn(b: f32, s: f32) -> f32 {
    if (b >= 1.0) {
        return 1.0;
    }
    if (s <= 0.0) {
        return 0.0;
    }
    return 1.0 - min((1.0 - b) / s, 1.0);
}

fn hard_light(b: f32, s: f32) -> f32 {
    if (s <= 0.5) {
        return 2.0 * b * s;
    }
    return 1.0 - 2.0 * (1.0 - b) * (1.0 - s);
}

fn soft_light_d(b: f32) -> f32 {
    if (b <= 0.25) {
        return ((16.0 * b - 12.0) * b + 4.0) * b;
    }
    return sqrt(b);
}

fn soft_light(b: f32, s: f32) -> f32 {
    if (s <= 0.5) {
        return b - (1.0 - 2.0 * s) * b * (1.0 - b);
    }
    return b + (2.0 * s - 1.0) * (soft_light_d(b) - b);
}

fn blend_sep(mode: u32, b: f32, s: f32) -> f32 {
    switch mode {
        case 0u: {
            return s;
        }
        case 1u: {
            return b * s;
        }
        case 2u: {
            return b + s - b * s;
        }
        case 3u: {
            return hard_light(s, b);
        }
        case 4u: {
            return b + s;
        }
        case 5u: {
            return b - s;
        }
        case 6u: {
            return min(b, s);
        }
        case 7u: {
            return max(b, s);
        }
        case 8u: {
            return color_dodge(b, s);
        }
        case 9u: {
            return color_burn(b, s);
        }
        case 10u: {
            return b + s - 1.0;
        }
        case 11u: {
            return hard_light(b, s);
        }
        case 12u: {
            return soft_light(b, s);
        }
        case 13u: {
            if (s <= 0.5) {
                return color_burn(b, 2.0 * s);
            }
            return color_dodge(b, 2.0 * s - 1.0);
        }
        case 14u: {
            return b + 2.0 * s - 1.0;
        }
        case 15u: {
            if (s <= 0.5) {
                return min(b, 2.0 * s);
            }
            return max(b, 2.0 * s - 1.0);
        }
        case 16u: {
            if (b + s >= 1.0) {
                return 1.0;
            }
            return 0.0;
        }
        case 17u: {
            return abs(b - s);
        }
        case 18u: {
            return b + s - 2.0 * b * s;
        }
        case 19u: {
            if (s <= 0.0) {
                return 1.0;
            }
            return b / s;
        }
        default: {
            return s;
        }
    }
}

fn lum(c: vec3<f32>) -> f32 {
    return 0.3 * c.r + 0.59 * c.g + 0.11 * c.b;
}

fn clip_color(c: vec3<f32>) -> vec3<f32> {
    let l = lum(c);
    let n = min(c.r, min(c.g, c.b));
    let x = max(c.r, max(c.g, c.b));
    var out = c;
    if (n < 0.0 && (l - n) != 0.0) {
        let scale = l / (l - n);
        out = vec3<f32>(l) + (c - vec3<f32>(l)) * scale;
    }
    if (x > 1.0 && (x - l) != 0.0) {
        let scale = (1.0 - l) / (x - l);
        out = vec3<f32>(l) + (c - vec3<f32>(l)) * scale;
    }
    return out;
}

fn set_lum(c: vec3<f32>, l: f32) -> vec3<f32> {
    return clip_color(c + vec3<f32>(l - lum(c)));
}

fn sat(c: vec3<f32>) -> f32 {
    return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
}

fn set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
    let cmax = max(c.r, max(c.g, c.b));
    let cmin = min(c.r, min(c.g, c.b));
    if (cmax <= cmin) {
        return vec3<f32>(0.0);
    }
    let cmid = c.r + c.g + c.b - cmax - cmin;
    let mid = (cmid - cmin) * s / (cmax - cmin);
    var out = vec3<f32>(0.0);
    if (c.r >= cmax) {
        out.r = s;
    } else if (c.r > cmin) {
        out.r = mid;
    }
    if (c.g >= cmax) {
        out.g = s;
    } else if (c.g > cmin) {
        out.g = mid;
    }
    if (c.b >= cmax) {
        out.b = s;
    } else if (c.b > cmin) {
        out.b = mid;
    }
    return out;
}

fn blend_rgb_mode(mode: u32, b: vec3<f32>, s: vec3<f32>) -> vec3<f32> {
    switch mode {
        case 20u: {
            return set_lum(set_sat(s, sat(b)), lum(b));
        }
        case 21u: {
            return set_lum(set_sat(b, sat(s)), lum(b));
        }
        case 22u: {
            return set_lum(s, lum(b));
        }
        case 23u: {
            return set_lum(b, lum(s));
        }
        case 24u: {
            if (lum(b) <= lum(s)) {
                return b;
            }
            return s;
        }
        case 25u: {
            if (lum(b) >= lum(s)) {
                return b;
            }
            return s;
        }
        default: {
            return vec3<f32>(blend_sep(mode, b.r, s.r), blend_sep(mode, b.g, s.g), blend_sep(mode, b.b, s.b));
        }
    }
}

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[index], 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let coord = vec2<i32>(i32(frag.x), i32(frag.y));
    let base = textureLoad(acc, coord, 0);
    let mode = u32(params.control.y + 0.5);
    let blended = blend_rgb_mode(mode, base.rgb, params.src.rgb);
    let coverage = clamp(params.control.x, 0.0, 1.0);
    let out = mix(base.rgb, blended, vec3<f32>(coverage));
    return vec4<f32>(clamp(out, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
