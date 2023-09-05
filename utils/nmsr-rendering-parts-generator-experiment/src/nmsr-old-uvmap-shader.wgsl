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
    
    var camera_distance = vertex.position.z / vertex.position.w;

    var near = 0.1;
    var far = 100.0;

    var depth = 1.0 - ((camera_distance - near) / (far - near));
    
    var dim = textureDimensions(texture);
    
    var x = vertex.tex_coord.x;
    var y = vertex.tex_coord.y;
    return vec4<f32>(x, 1.0 - y, depth, 1.0);
}