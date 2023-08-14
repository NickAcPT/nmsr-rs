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

const THRESHOLD: f32 = 0.001;

fn round_coord(coord: f32) -> i32 {
    // So, here's the thing. When we are multisampling, the coordinates get all weird.
    // So, we need to do some stuff to make sure that we are sampling the right texels.
    
    // First thing, if we are too close to the next texel, we need to round up, otherwise we round down.
    // If not, we just truncate the coordinate.
    
    return i32(trunc(coord));
    //var coord_int: i32 = i32(coord);
    //var coord_frac: f32 = coord - f32(coord_int);
    //
    //if (coord_frac > 1.0 - THRESHOLD) {
    //    coord_int += 1;
    //    
    //    return coord_int;
    //} else if (coord_frac < -THRESHOLD) {
    //    coord_int -= 1;
    //    return coord_int;
    //}
    //
    //return i32(trunc(coord));
}

fn round_text_coord(coord: vec2<f32>) -> vec2<i32> {
    var u = coord.x;
    var v = coord.y;
    
    return vec2<i32>(round_coord(u), round_coord(v));
}

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    var color: vec4<f32> = textureLoad(
        texture,
        round_text_coord(vertex.tex_coord),
        0
    );
    
    //if (color.a > 0.0) {
    //    color.r /= color.a;
    //    color.g /= color.a;
    //    color.b /= color.a;
    //} 
    
    if (color.a == 0.0) {
        discard;
    }
    
    return compute_sun_lighting(color, vertex.normal);
}