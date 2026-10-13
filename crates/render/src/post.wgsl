// SubstanceCraft viewport post-effects (clean-room, WGSL): a cheap bloom and a vignette.
//
// The scene image is already tonemapped and sRGB-encoded, so these are applied in display space.

struct Post {
    // x, y = texel size; z = vignette; w = bloom
    data: vec4<f32>,
};

@group(0) @binding(0) var scene_texture: texture_2d<f32>;
@group(0) @binding(1) var scene_sampler: sampler;
@group(0) @binding(2) var<uniform> post: Post;

const BLOOM_THRESHOLD: f32 = 0.6;

@vertex
fn vs_post(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(vec2<f32>(-1.0, -1.0), vec2<f32>(3.0, -1.0), vec2<f32>(-1.0, 3.0));
    return vec4<f32>(positions[index], 0.0, 1.0);
}

@fragment
fn fs_post(@builtin(position) frag: vec4<f32>) -> @location(0) vec4<f32> {
    let uv = frag.xy * post.data.xy;
    var color = textureSample(scene_texture, scene_sampler, uv).rgb;

    if (post.data.w > 0.001) {
        var bloom = vec3<f32>(0.0);
        for (var i = 0u; i < 8u; i = i + 1u) {
            let angle = f32(i) * 0.7853982;
            let offset = vec2<f32>(cos(angle), sin(angle)) * 5.0 * post.data.xy;
            let sampled = textureSample(scene_texture, scene_sampler, uv + offset).rgb;
            bloom += max(sampled - vec3<f32>(BLOOM_THRESHOLD), vec3<f32>(0.0));
        }
        color += (bloom / 8.0) * post.data.w;
    }

    let centered = frag.xy * post.data.xy - vec2<f32>(0.5);
    let vignette = 1.0 - post.data.z * smoothstep(0.3, 0.9, length(centered) * 1.41421356);
    color *= vignette;

    return vec4<f32>(clamp(color, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
