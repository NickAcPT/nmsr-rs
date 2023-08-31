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
    color: vec4<f32>,
    normal: vec3<f32>,
) -> vec4<f32> {
    var sun_direction: vec3<f32> = normalize(sun.direction);
    var sun_dot: f32 = dot(normal, -sun_direction);
    
    var sun_color: vec3<f32> = vec3<f32>(1.0, 1.0, 1.0) * clamp(sun.intensity * sun_dot, sun.ambient, MAX_LIGHT);
    
    return color * vec4<f32>(sun_color, 1.0);
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec4<f32> = textureSample(
        texture,
        texture_sampler,
        vec2<f32>(vertex.tex_coord)
    );
    
    if (color.a == 0.0) {
        discard;
    }
    
    return compute_sun_lighting(color, vertex.normal);
}