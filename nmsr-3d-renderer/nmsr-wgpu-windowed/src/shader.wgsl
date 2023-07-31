struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) @invariant position: vec4<f32>,
};

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;


@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1)@binding(1)
var s_diffuse: sampler;

@vertex
fn vs_main(
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coord = tex_coord;
    result.position = transform * position;
    return result;
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec4<f32> = textureSample(t_diffuse, s_diffuse, vertex.tex_coord);
    if (color.a == 0.0) {
        discard;
    }
    return color;
}