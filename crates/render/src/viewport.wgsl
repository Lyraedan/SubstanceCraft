// SubstanceCraft viewport shader (clean-room, WGSL).
//
// A compact metal/rough PBR shading model with a hemisphere ambient and a single key light,
// plus display modes for inspecting the surface data. Tone mapping and sRGB encoding happen
// here; the offscreen target is Rgba8Unorm (gamma storage) so egui shows it directly.

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

struct Material {
    base_color: vec4<f32>,
    // x = metallic, y = roughness
    params: vec4<f32>,
};

@group(1) @binding(0) var<uniform> material: Material;

@group(2) @binding(0) var env_texture: texture_2d<f32>;
@group(2) @binding(1) var env_sampler: sampler;

const PI: f32 = 3.14159265359;

struct VertexOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
};

@vertex
fn vs_main(
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tangent: vec4<f32>,
) -> VertexOut {
    var out: VertexOut;
    out.world_position = position;
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.normal = normal;
    out.uv = uv;
    out.tangent = tangent;
    return out;
}

fn distribution_ggx(n_dot_h: f32, roughness: f32) -> f32 {
    let a = roughness * roughness;
    let a2 = a * a;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / max(PI * d * d, 1e-6);
}

fn geometry_schlick_ggx(n_dot_v: f32, roughness: f32) -> f32 {
    let r = roughness + 1.0;
    let k = (r * r) / 8.0;
    return n_dot_v / (n_dot_v * (1.0 - k) + k);
}

fn geometry_smith(n_dot_v: f32, n_dot_l: f32, roughness: f32) -> f32 {
    return geometry_schlick_ggx(n_dot_v, roughness) * geometry_schlick_ggx(n_dot_l, roughness);
}

fn fresnel_schlick(cos_theta: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn fresnel_schlick_roughness(cos_theta: f32, f0: vec3<f32>, roughness: f32) -> vec3<f32> {
    return f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(clamp(1.0 - cos_theta, 0.0, 1.0), 5.0);
}

fn sample_environment(direction: vec3<f32>, lod: f32) -> vec3<f32> {
    let d = normalize(direction);
    let u = atan2(d.z, d.x) * (0.5 / PI) + 0.5;
    let v = acos(clamp(d.y, -1.0, 1.0)) / PI;
    return textureSampleLevel(env_texture, env_sampler, vec2<f32>(u, v), lod).rgb;
}

fn tonemap_reinhard(x: vec3<f32>) -> vec3<f32> {
    return x / (vec3<f32>(1.0) + x);
}

fn tonemap_aces(x: vec3<f32>) -> vec3<f32> {
    let a = 2.51;
    let b = 0.03;
    let c = 2.43;
    let d = 0.59;
    let e = 0.14;
    return clamp((x * (a * x + b)) / (x * (c * x + d) + e), vec3<f32>(0.0), vec3<f32>(1.0));
}

fn tonemap_neutral(x: vec3<f32>) -> vec3<f32> {
    let start = 0.8;
    let d = 0.15;
    let peak = 1.0 + d;
    let elbow = start + d;
    let clamped = max(x, vec3<f32>(0.0));
    let scaled = (clamped - vec3<f32>(elbow)) / (peak - elbow);
    let shoulder = vec3<f32>(1.0) - pow(max(vec3<f32>(1.0) - scaled, vec3<f32>(0.0)), vec3<f32>(2.0));
    let compressed = vec3<f32>(elbow) + shoulder * (peak - elbow);
    return select(compressed, clamped, clamped < vec3<f32>(elbow));
}

fn gamma_from_linear(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * 12.92;
    let higher = 1.055 * pow(max(rgb, vec3<f32>(0.0)), vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(higher, lower, cutoff);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let base_color = material.base_color;
    let metallic = clamp(material.params.x, 0.0, 1.0);
    let roughness = clamp(material.params.y, 0.04, 1.0);
    let display_mode = i32(camera.params.y + 0.5);
    let tonemap_mode = i32(camera.params.z + 0.5);

    // Debug display modes: show raw data, no lighting or tone mapping.
    if (display_mode == 1) {
        return vec4<f32>(clamp(normalize(in.normal) * 0.5 + vec3<f32>(0.5), vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
    }
    if (display_mode == 2) {
        return vec4<f32>(vec3<f32>(roughness), 1.0);
    }
    if (display_mode == 3) {
        return vec4<f32>(vec3<f32>(metallic), 1.0);
    }
    if (display_mode == 4) {
        let grid = floor(in.uv * 16.0);
        let parity = (grid.x + grid.y) - 2.0 * floor((grid.x + grid.y) * 0.5);
        return vec4<f32>(mix(vec3<f32>(0.12), vec3<f32>(0.82), parity), 1.0);
    }

    let n = normalize(in.normal);
    let v = normalize(camera.camera_pos.xyz - in.world_position);
    let l = normalize(camera.light_dir.xyz);
    let h = normalize(v + l);
    let n_dot_l = max(dot(n, l), 0.0);
    let n_dot_v = max(dot(n, v), 1e-4);
    let n_dot_h = max(dot(n, h), 0.0);

    let f0 = mix(vec3<f32>(0.04), base_color.rgb, metallic);
    let f = fresnel_schlick(max(dot(h, v), 0.0), f0);
    let ndf = distribution_ggx(n_dot_h, roughness);
    let g = geometry_smith(n_dot_v, n_dot_l, roughness);
    let specular = (ndf * g * f) / max(4.0 * n_dot_v * n_dot_l, 1e-4);
    let kd = (vec3<f32>(1.0) - f) * (1.0 - metallic);
    let direct = (kd * base_color.rgb / PI + specular) * camera.light_color.rgb * n_dot_l;

    let hemisphere = mix(camera.ambient_ground.rgb, camera.ambient_sky.rgb, n.y * 0.5 + 0.5);
    var ambient = hemisphere * base_color.rgb * (1.0 - metallic * 0.5);
    if (camera.env_params.x > 0.5) {
        let f_ibl = fresnel_schlick_roughness(n_dot_v, f0, roughness);
        let kd_ibl = (vec3<f32>(1.0) - f_ibl) * (1.0 - metallic);
        let irradiance = sample_environment(n, camera.env_params.y);
        let reflection = sample_environment(reflect(-v, n), roughness * camera.env_params.y);
        ambient = kd_ibl * base_color.rgb * irradiance + reflection * f_ibl;
    }
    let color = direct + ambient;

    let exposed = color * exp2(camera.params.x);
    var mapped = exposed;
    if (tonemap_mode == 0) {
        mapped = clamp(exposed, vec3<f32>(0.0), vec3<f32>(1.0));
    } else if (tonemap_mode == 1) {
        mapped = tonemap_reinhard(exposed);
    } else if (tonemap_mode == 2) {
        mapped = tonemap_aces(exposed);
    } else if (tonemap_mode == 4) {
        mapped = tonemap_neutral(exposed);
    } else {
        mapped = tonemap_aces(exposed);
    }

    return vec4<f32>(gamma_from_linear(clamp(mapped, vec3<f32>(0.0), vec3<f32>(1.0))), 1.0);
}
