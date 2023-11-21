use glam::{Vec2, Vec3, Vec4};
use image::Rgba;

use crate::camera::Camera;

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

#[derive(Clone, Copy, Debug)]
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
    pub old_w: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SunInformation {
    pub direction: Vec3,
    pub intensity: f32,
    pub ambient: f32,
}

pub struct ShaderState {
    pub camera: Camera,
    pub texture: image::RgbaImage,
    pub sun: SunInformation,
}

impl ShaderState {
    pub fn new(camera: Camera, texture: image::RgbaImage, sun: SunInformation) -> Self {
        let mut result = Self {
            camera,
            texture,
            sun,
        };
        
        result.update();
        
        result
    }

    pub fn update(&mut self) {
        self.camera.update_mvp();
    }
}

const MAX_LIGHT: f32 = 1.0;

pub fn vertex_shader(vertex: VertexInput, state: &ShaderState) -> VertexOutput {
    VertexOutput {
        tex_coord: vertex.tex_coord,
        position: state.camera.get_cached_view_projection_matrix() * vertex.position,
        normal: vertex.normal,
        old_w: 0.0,
    }
}

fn compute_sun_lighting(color: Vec4, normal: Vec3, state: &ShaderState) -> Vec4 {
    let sun_direction: Vec3 = state.sun.direction.normalize();
    let sun_dot: f32 = normal.dot(-sun_direction);

    let sun_color =
        Vec3::splat((state.sun.intensity * sun_dot).clamp(state.sun.ambient, MAX_LIGHT));

    color * sun_color.extend(1.0)
}

pub fn fragment_shader(vertex: VertexOutput, state: &ShaderState) -> Vec4 {
    let Some(color) = state.texture.get_pixel_checked(
        (vertex.tex_coord.x * state.texture.width() as f32) as u32,
        (vertex.tex_coord.y * state.texture.height() as f32) as u32,
    ) else {
        return Vec4::ZERO;
    };

    let color = unsafe { std::mem::transmute::<Rgba<u8>, [u8; 4]>(*color) };

    let color = Vec4::new(
        f32::from(color[0]),
        f32::from(color[1]),
        f32::from(color[2]),
        f32::from(color[3]),
    ) / 255.0;

    if color.w == 0.0 {
        return Vec4::ZERO;
    }

    compute_sun_lighting(color, vertex.normal, state)
}
