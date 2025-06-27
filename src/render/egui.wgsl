struct Transform {
    scale: vec2<f32>,
    translation: vec2<f32>,
}

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
}

@group(0) @binding(0) var<uniform> transform: Transform;
@group(1) @binding(0) var image_texture: texture_2d<f32>;
@group(1) @binding(1) var image_sampler: sampler;

// 0-1 linear  from  0-1 sRGB gamma.
fn linear_from_gamma_rgb(srgb: vec3<f32>) -> vec3<f32> {
    let cutoff = srgb < vec3<f32>(0.04045);
    let lower = srgb / vec3<f32>(12.92);
    let higher = pow((srgb + vec3<f32>(0.055)) / vec3<f32>(1.055), vec3<f32>(2.4));
    return select(higher, lower, cutoff);
}

// 0-1 sRGB gamma  from  0-1 linear.
fn gamma_from_linear_rgb(rgb: vec3<f32>) -> vec3<f32> {
    let cutoff = rgb < vec3<f32>(0.0031308);
    let lower = rgb * vec3<f32>(12.92);
    let higher = vec3<f32>(1.055) * pow(rgb, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(higher, lower, cutoff);
}

// 0-1 sRGBA gamma  from  0-1 linear.
fn gamma_from_linear_rgba(linear_rgba: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(gamma_from_linear_rgb(linear_rgba.rgb), linear_rgba.a);
}

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let position = in.position * transform.scale + transform.translation;
    // Not sure why Egui does vertex color interpolation in sRGB but here we do it the same way as well.
    return VertexOutput(vec4<f32>(position, 0.0, 1.0), in.color, in.uv);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texture_color_linear = textureSample(image_texture, image_sampler, in.uv);
    // We un-premultiply Egui-managed textures on CPU, because Bevy doesn't premultiply it's own images, so here we pre-multiply everything.
    let texture_color_linear_premultiplied = vec4<f32>(texture_color_linear.rgb * texture_color_linear.a, texture_color_linear.a);
    let texture_color_gamma_premultiplied = gamma_from_linear_rgba(texture_color_linear_premultiplied);

    // Quoting the Egui's glsl shader:
    // "We multiply the colors in gamma space, because that's the only way to get text to look right."
    let color_gamma = texture_color_gamma_premultiplied * in.color;

    return vec4<f32>(linear_from_gamma_rgb(color_gamma.rgb), color_gamma.a);
}
