// Vertex Shader

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
    @location(3) dark_color: vec4<f32>,
}

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) dark_color: vec4<f32>,
}

@group(0) @binding(0) var<uniform> scene: mat4x4<f32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;

    out.position = scene * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coords = in.uv;
    out.color = in.color;
    out.dark_color = in.dark_color;

    return out;
}

// Fragment Shader

@group(1) @binding(0) var tex: texture_2d<f32>;
@group(1) @binding(1) var tex_sampler: sampler;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_color = textureSample(tex, tex_sampler, in.tex_coords);

    let blended_rgb = ((tex_color.a - 1.0) * in.dark_color.a + 1.0 - tex_color.rgb) * in.dark_color.rgb + tex_color.rgb * in.color.rgb;
    let blended_a = tex_color.a * in.color.a;

    return vec4<f32>(blended_rgb, blended_a);
}
