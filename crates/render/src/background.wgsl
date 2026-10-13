// SubstanceCraft procedural sky background: a fullscreen pass shading a soft vertical gradient
// by the view direction (clean-room, WGSL).

struct Camera {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    // x = exposure stops, y = display mode, z = tone map
    params: vec4<f32>,
    // x = has environment, y = max mip level
    env_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var env_texture: texture_2d<f32>;
@group(1) @binding(1) var env_sampler: sampler;

const PI: f32 = 3.14159265359;

struct BgOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) ndc: vec2<f32>,
};

@vertex
fn vs_background(@builtin(vertex_index) index: u32) -> BgOut {
    var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    let p = positions[index];
    var out: BgOut;
    out.clip_position = vec4<f32>(p, 0.0, 1.0);
    out.ndc = p;
    return out;
}

fn gamma_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * 12.92;
    let higher = 1.055 * pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, cutoff);
}

fn tonemap_aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fs_background(in: BgOut) -> @location(0) vec4<f32> {
    let far = camera.inv_view_proj * vec4<f32>(in.ndc, 1.0, 1.0);
    let direction = normalize(far.xyz / far.w - camera.camera_pos.xyz);

    if (camera.env_params.x > 0.5) {
        let u = atan2(direction.z, direction.x) * (0.5 / PI) + 0.5;
        let v = acos(clamp(direction.y, -1.0, 1.0)) / PI;
        let env = textureSampleLevel(env_texture, env_sampler, vec2<f32>(u, v), 0.0).rgb;
        let exposed = env * exp2(camera.params.x);
        return vec4<f32>(gamma_from_linear(tonemap_aces(exposed)), 1.0);
    }

    let t = clamp(direction.y * 0.5 + 0.5, 0.0, 1.0);
    var color = mix(camera.ambient_ground.rgb, camera.ambient_sky.rgb, smoothstep(0.0, 1.0, t));
    // Brighten toward the horizon for a studio-backdrop feel.
    let horizon = mix(camera.ambient_ground.rgb, camera.ambient_sky.rgb, 0.5) * 1.2;
    color = mix(color, horizon, 1.0 - abs(direction.y));
    return vec4<f32>(gamma_from_linear(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}
