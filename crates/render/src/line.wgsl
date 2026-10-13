// SubstanceCraft line shader for the ground grid and wireframe edges (clean-room, WGSL).

struct Camera {
    view_proj: mat4x4<f32>,
    inv_view_proj: mat4x4<f32>,
    camera_pos: vec4<f32>,
    light_dir: vec4<f32>,
    light_color: vec4<f32>,
    ambient_sky: vec4<f32>,
    ambient_ground: vec4<f32>,
    params: vec4<f32>,
    env_params: vec4<f32>,
};

@group(0) @binding(0) var<uniform> camera: Camera;

struct LineOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec3<f32>,
};

@vertex
fn vs_line(@location(0) position: vec3<f32>, @location(1) color: vec3<f32>) -> LineOut {
    var out: LineOut;
    out.clip_position = camera.view_proj * vec4<f32>(position, 1.0);
    out.color = color;
    return out;
}

@fragment
fn fs_line(in: LineOut) -> @location(0) vec4<f32> {
    return vec4<f32>(clamp(in.color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
