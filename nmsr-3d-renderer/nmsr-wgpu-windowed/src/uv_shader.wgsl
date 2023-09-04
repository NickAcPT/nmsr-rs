struct VertexInput {
    @location(0) position: vec4<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) normal: vec3<f32>,
};

struct SunInformation {
    direction: vec3<f32>,
    intensity: f32,
    ambient: f32,
}

@group(0)
@binding(0)
var<uniform> transform: mat4x4<f32>;

@group(1)
@binding(0)
var texture: texture_2d<f32>;

@group(1)
@binding(1)
var texture_sampler: sampler;

@group(2)
@binding(0)
var<uniform> sun: SunInformation;

@vertex
fn vs_main(
    vertex: VertexInput,
) -> VertexOutput {
    var result: VertexOutput;
    result.tex_coord = vertex.tex_coord;
    result.position = transform * vertex.position;
    result.normal = vertex.normal;
    return result;
}
const MAX_LIGHT: f32 = 1.0;

fn compute_sun_lighting(
    normal: vec3<f32>,
) -> f32 {
    var sun_direction: vec3<f32> = normalize(sun.direction);
    var sun_dot: f32 = dot(normal, -sun_direction);

    return clamp(sun.intensity * sun_dot, sun.ambient, MAX_LIGHT);
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    // Using 6 bits for each u and v coordinate since Minecraft uses at most 64x64 textures for skins
    var MAX_VALUE_PER_UV = 63.0;
    
    // We have 8 bits reserved for shading, meaning we can have 256 different shading values
    var MAX_VALUE_PER_SHADING = 255.0;

    var u = u32(vertex.tex_coord.x * MAX_VALUE_PER_UV);
    var v = u32(vertex.tex_coord.y * MAX_VALUE_PER_UV);
    var shading = u32(compute_sun_lighting(vertex.normal) * MAX_VALUE_PER_SHADING);
    var camera_distance = clamp(((transform * vec4<f32>(0.0, 0.0, 0.0, 1.0)).z - vertex.position.z) * vertex.position.w, 0.0, 1.0);

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

    var proj_r = transform[2][2];
    var proj_q = transform[2][3];

    var near = proj_q / (proj_r + 1.0);
    var far = proj_q / proj_r;

    var depth = (1.0 / camera_distance - 1.0 / near) / (1.0 / far - 1.0 / near);

    return vec4<f32>(depth, depth, depth, 1.0);//vec4<f32>(f32(r) / 256.0, f32(g) / 256.0, 0.0, 1.0);
}