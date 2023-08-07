use glam::Mat4;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, Buffer, Texture, TextureDescriptor, TextureDimension,
    TextureFormat, TextureUsages, TextureView, TextureViewDescriptor,
};

use crate::high_level::camera::Camera;
use crate::high_level::pipeline::graphics_context::GraphicsContext;

#[derive(Debug)]
struct SceneContextTextures {
    depth_texture: SceneTexture,
    output_texture: SceneTexture,
}

#[derive(Debug)]
pub struct SceneContext {
    pub transform_matrix_buffer: Buffer,
    pub transform_bind_group: BindGroup,
    textures: Option<SceneContextTextures>,
}

#[derive(Debug)]
pub struct SceneTexture {
    texture: Texture,
    view: TextureView,
}

impl SceneContext {
    pub fn new(context: &GraphicsContext) -> Self {
        let device = &context.device;

        let (transform_matrix_buffer, transform_bind_group) =
            create_transform_buffer_and_bing_group(device, context);

        Self {
            transform_bind_group,
            transform_matrix_buffer,
            textures: None,
        }
    }

    fn set_camera_parameters(&self, context: &GraphicsContext, camera: &mut Camera) {
        let matrix = camera.get_view_projection_matrix();
        context.queue.write_buffer(
            &self.transform_matrix_buffer,
            0,
            bytemuck::cast_slice(matrix.as_ref()),
        );
    }

    pub(crate) fn init(
        &mut self,
        graphics_context: &GraphicsContext,
        camera: &mut Camera,
        viewport_size: super::scene::Size,
    ) {
        // Setup camera matrix
        self.set_camera_parameters(graphics_context, camera);

        // Setup our depth texture
        let depth_texture = create_texture(
            graphics_context,
            viewport_size.width,
            viewport_size.height,
            GraphicsContext::DEPTH_TEXTURE_FORMAT,
            TextureUsages::RENDER_ATTACHMENT,
            Some("Depth Texture"),
        );

        // Setup our output texture
        let output_texture = create_texture(
            graphics_context,
            viewport_size.width,
            viewport_size.height,
            graphics_context.texture_format,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            Some("Output Texture"),
        );

        // Save our textures
        self.textures = Some(SceneContextTextures {
            depth_texture,
            output_texture,
        })
    }
}

fn create_texture(
    context: &GraphicsContext,
    width: u32,
    height: u32,
    format: TextureFormat,
    usage: TextureUsages,
    label: Option<&str>,
) -> SceneTexture {
    let texture = context.device.create_texture(&TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format,
        usage,
        label,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());

    SceneTexture { texture, view }
}

fn create_transform_buffer_and_bing_group(
    device: &wgpu::Device,
    context: &GraphicsContext,
) -> (Buffer, BindGroup) {
    let transform_matrix_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Transform Matrix Buffer"),
        contents: bytemuck::cast_slice(Mat4::IDENTITY.as_ref()),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let transform_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Transform Bind Group"),
        layout: &context.layouts.transform_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: transform_matrix_buffer.as_entire_binding(),
        }],
    });
    (transform_matrix_buffer, transform_bind_group)
}
