// SubstanceCraft GPU compositor pass (clean-room, WGSL).
//
// Reads the accumulated channel texture and blends one constant fill layer over it, matching the
// CPU oracle's per-channel blend + coverage model.

struct Params {
    src: vec4<f32>,
    // x = coverage, y = blend mode, z = 1 when the channel is scalar
    control: vec4<f32>,
};

@group(0) @binding(0) var acc: texture_2d<f32>;
@group(0) @binding(1) var<uniform> params: Params;

fn blend_channel(mode: u32, b: f32, s: f32) -> f32 {
    switch mode {
        case 0u: {
            return s;
        }
        case 1u: {
            return b * s;
        }
        case 2u: {
            return 1.0 - (1.0 - b) * (1.0 - s);
        }
        case 3u: {
            if (b < 0.5) {
                return 2.0 * b * s;
            }
            return 1.0 - 2.0 * (1.0 - b) * (1.0 - s);
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
        default: {
            return max(b, s);
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
    let blended = vec3<f32>(blend_channel(mode, base.r, params.src.r), blend_channel(mode, base.g, params.src.g), blend_channel(mode, base.b, params.src.b));
    let coverage = clamp(params.control.x, 0.0, 1.0);
    let out = mix(base.rgb, blended, vec3<f32>(coverage));
    return vec4<f32>(clamp(out, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
