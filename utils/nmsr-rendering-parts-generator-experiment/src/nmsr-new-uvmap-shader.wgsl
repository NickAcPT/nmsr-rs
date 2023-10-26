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
fn fs_main(vertex: VertexOutput, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    //backingface:if (front_facing) {
    //backingface:    discard;
    //backingface:}
    //frontface:if (!front_facing) {
    //frontface:    discard;
    //frontface:}
    
    // Using 6 bits for each u and v coordinate since Minecraft uses at most 64x64 textures for skins
    var MAX_VALUE_PER_UV = 64.0;
    
    // We have 8 bits reserved for shading, meaning we can have 256 different shading values
    var MAX_VALUE_PER_SHADING = 255.0;
    
    // We have 12 bits reserved for depth, meaning we can have 4096 different depth values
    var MAX_VALUE_PER_DEPTH = 4095.0;

    var u = u32(floor(vertex.tex_coord.x * MAX_VALUE_PER_UV));
    var v = u32(floor(vertex.tex_coord.y * MAX_VALUE_PER_UV));
    var shading = u32(compute_sun_lighting(vertex.normal) * MAX_VALUE_PER_SHADING);
    var camera_distance = vertex.position.z / vertex.position.w;

    var near = 0.1;
    var far = 100.0;
    //iso:near = -100.0;

    var depth = (far - camera_distance) / (far - near);
    //iso:depth = (2.0 * vertex.position.z * near + far - near) / (far - near);
    
    var final_depth = u32(depth * MAX_VALUE_PER_DEPTH);

    // Our Red channel is composed of the 6 bits of the u coordinate + 2 bits from the v coordinate
    // U is used as-is because our coordinates are 0-63
    // 0   1   2   3   4   5   6   7
    // [    ---- u ----    ]   [ v ]
    // Our Green channel is composed of the 4 remaining bits of the v coordinate + 4 bits from the shading
    // V is used as-is because our coordinates are 0-63
    // 0   1   2   3   4   5   6   7
    // [  -- v --  ]   [  -- s --  ]
    // Our Blue channel is composed of the 4 remaining bits of the shading + 4 bits from the depth
    // 0   1   2   3   4   5   6   7
    // [  -- s --  ]   [  -- d --  ]
    // Our Alpha channel is composed of the 8 remaining bits of the depth
    // 0   1   2   3   4   5   6   7
    // [          -- d --          ]
    
    var final_number = ((final_depth & 0x1FFFu) << 20u) | ((shading & 0xFFu) << 12u) | ((v & 0x3Fu) << 6u) | (u & 0x3Fu);
    // Final number is in rgba bits
    var r = final_number & 0xFFu;
    var g = (final_number >> 8u) & 0xFFu;
    var b = (final_number >> 16u) & 0xFFu;
    var a = (final_number >> 24u) & 0xFFu;
    
    return vec4<f32>(f32(r) / 255.0, f32(g) / 255.0, f32(b) / 255.0, f32(a) / 255.0);
}