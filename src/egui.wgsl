struct Transform {
    scale: vec2<f32>;
    translation: vec2<f32>;
};

struct VertexInput {
    [[location(0)]] position: vec2<f32>;
    [[location(1)]] uv: vec2<f32>;
    [[location(2)]] color: vec4<f32>;
};

struct VertexOutput {
    [[builtin(position)]] position: vec4<f32>;
    [[location(0)]] color: vec4<f32>;
    [[location(1)]] uv: vec2<f32>;
};

[[group(0), binding(0)]] var<uniform> transform: Transform;
[[group(1), binding(0)]] var image_texture: texture_2d<f32>;
[[group(1), binding(1)]] var image_sampler: sampler;

fn linear_from_srgb(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let lower = srgb / 12.92;
    let higher = pow((srgb + 0.055) / 1.055, vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

[[stage(vertex)]]
fn vs_main(in: VertexInput) -> VertexOutput {
    let position = in.position * transform.scale + transform.translation;
    let color = vec4<f32>(linear_from_srgba(in.color.rgb), in.color.a);
    return VertexOutput(vec4<f32>(position, 0.0, 1.0), color, in.uv);
}

[[stage(fragment)]]
fn fs_main(in: VertexOutput) -> [[location(0)]] vec4<f32> {
    return in.color * textureSample(image_texture, image_sampler, in.uv);
}
