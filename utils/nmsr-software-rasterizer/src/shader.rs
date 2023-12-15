use std::{sync::Arc, fmt::Debug};

use glam::{Vec2, Vec4, Vec3A};
use image::{Rgba, RgbaImage};
use nmsr_rendering::{high_level::{parts::{provider::{PlayerPartProviderContext, PlayerPartsProvider, PartsProvider}, part::Part}, types::PlayerBodyPartType, utils::parts::primitive_convert}, low_level::primitives::mesh::{PrimitiveDispatch, Mesh}};

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
    pub normal: Vec3A,
}

#[derive(Clone, Copy, Debug)]
pub struct VertexOutput {
    pub position: Vec4,
    pub tex_coord: Vec2,
    pub normal: Vec3A,
    pub old_w_recip: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SunInformation {
    pub direction: Vec3A,
    pub intensity: f32,
    pub ambient: f32,
}

impl SunInformation {
    pub fn new(direction: Vec3A, intensity: f32, ambient: f32) -> Self { Self { direction, intensity, ambient } }
}

impl Debug for ShaderState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShaderState").finish()
    }
}

pub struct ShaderState {
    pub camera: Camera,
    pub texture: Arc<RgbaImage>,
    pub texture_size: (f32, f32),
    pub sun: SunInformation,
    pub primitive: PrimitiveDispatch,
}

impl ShaderState {
    pub fn new(camera: Camera, texture: Arc<RgbaImage>, sun: SunInformation, context: &PlayerPartProviderContext<()>, parts: &[PlayerBodyPartType]) -> Self {
        let providers = [
            PlayerPartsProvider::Minecraft,
            #[cfg(feature = "ears")]
            PlayerPartsProvider::Ears,
        ];

        let parts = providers
            .iter()
            .flat_map(|provider| { 
                parts.iter().flat_map(|part| provider.get_parts(&context, *part))
             })
            .collect::<Vec<Part>>();
        
        let parts = parts
            .into_iter()
            .map(|p| primitive_convert(&p))
            .collect::<Vec<_>>();
        
        Self::new_with_primitive(camera, texture, sun, PrimitiveDispatch::Mesh(Mesh::new(parts)))
    }
    
    pub fn new_with_primitive(camera: Camera, texture: Arc<RgbaImage>, sun: SunInformation, primitive: PrimitiveDispatch) -> Self {
        let mut result: ShaderState = Self {
            camera,
            texture_size: (texture.width() as f32, texture.height() as f32),
            texture,
            sun,
            primitive
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
        old_w_recip: 0.0,
    }
}

fn compute_sun_lighting(color: &[u8; 4], normal: Vec3A, state: &ShaderState) -> [u8; 4] {
    let sun_direction: Vec3A = state.sun.direction.normalize();
    let sun_dot: f32 = normal.dot(-sun_direction);

    let sun_color = (state.sun.intensity * sun_dot).clamp(state.sun.ambient, MAX_LIGHT);

    [
        (color[0] as f32 * sun_color) as u8,
        (color[1] as f32 * sun_color) as u8,
        (color[2] as f32 * sun_color) as u8,
        color[3],
    ]
}

pub fn fragment_shader(vertex: VertexOutput, state: &ShaderState) -> [u8; 4] {
    let (w, h) = state.texture_size;
    
    let Some(color) = state.texture.get_pixel_checked(
        (vertex.tex_coord.x * w) as u32,
        (vertex.tex_coord.y * h) as u32,
    ) else {
        return [0; 4];
    };

    let color = unsafe { std::mem::transmute::<Rgba<u8>, [u8; 4]>(*color) };

    if std::intrinsics::unlikely(color[3] == 0) {
        return [0; 4];
    }

    compute_sun_lighting(&color, vertex.normal, state)
}
