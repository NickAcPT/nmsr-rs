struct VertexOutput {
    @location(0) tex_coord: vec2<f32>,
    @builtin(position) position: vec4<f32>,
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
    
    // Using 6 bits for each u and v coordinate since Minecraft uses at most 64x64 textures for skins
    var MAX_VALUE_PER_UV = 63.0;

    var u = u32(vertex.tex_coord.x * MAX_VALUE_PER_UV);
    var v = u32(vertex.tex_coord.y * MAX_VALUE_PER_UV);
    var shading = 0u;
    var depth = 0u;

    // Our Red channel is composed of the 6 bits of the u coordinate + 2 bits from the v coordinate
    // U is used as-is because our coordinates are 0-63
    // 0   1   2   3   4   5   6   7
    // [    ---- u ----    ]   [ v ]
    var r = (u | (v >> 6u)) & 0xFFu;
    
    // Our Green channel is composed of the 4 remaining bits of the v coordinate + 4 bits from the shading
    // U is used as-is because our coordinates are 0-63
    // 0   1   2   3   4   5   6   7
    // [  -- v --  ]   [  -- s --  ]
    var g = ((v >> 2u) | (shading >> 4u)) & 0xFFu;
    
    return vec4<f32>(f32(r) / 256.0, f32(g) / 256.0, 0.0, color.a);
}