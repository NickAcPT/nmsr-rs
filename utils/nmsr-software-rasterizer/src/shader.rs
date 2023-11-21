use glam::{Vec2, Vec4, Vec3};
use image::Rgba;

/* struct VertexInput {
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
} */

pub struct VertexInput {
    pub position: Vec4,
    pub tex_coord: Vec2,
    pub normal: Vec3,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexOutput {
    pub position: Vec4,
    pub tex_coord: Vec2,
    pub normal: Vec3,
}

pub struct SunInformation {
    pub direction: Vec3,
    pub intensity: f32,
    pub ambient: f32,
}

pub struct ShaderState {
    pub transform: glam::Mat4,
    pub texture: image::RgbaImage,
    pub sun: SunInformation,
}

const MAX_LIGHT: f32 = 1.0;

pub fn vertex_shader(vertex: VertexInput, state: &ShaderState) -> VertexOutput {
    let result = VertexOutput {
        tex_coord: vertex.tex_coord,
        position: state.transform * vertex.position,
        normal: vertex.normal,
    };

    result
}

fn compute_sun_lighting(color: Vec4, normal: Vec3, state: &ShaderState) -> Vec4 {
    let sun_direction: Vec3 = state.sun.direction.normalize();
    let sun_dot: f32 = normal.dot(-sun_direction);

    let sun_color: Vec4 = Vec4::new(1.0, 1.0, 1.0, 1.0)
        * (state.sun.intensity * sun_dot).clamp(state.sun.ambient, MAX_LIGHT);

    color * sun_color
}

pub fn fragment_shader(vertex: VertexOutput, state: &ShaderState) -> Vec4 {
    return Vec3::X.extend(1.0);
    
    let color = *state.texture.get_pixel(
        (vertex.tex_coord.x * state.texture.width() as f32) as u32,
        (vertex.tex_coord.y * state.texture.height() as f32) as u32,
    );

    let color = Vec4::new(
        color[0] as f32 / 255.0,
        color[1] as f32 / 255.0,
        color[2] as f32 / 255.0,
        color[3] as f32 / 255.0,
    );

    if color.w == 0.0 {
        return Vec4::ZERO;
    }

    compute_sun_lighting(color, vertex.normal, state)
}
