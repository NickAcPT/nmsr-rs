use std::mem;
use tokio::sync::oneshot::channel;

use glam::Mat4;
use image::buffer::ConvertBuffer;
use image::RgbaImage;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, Buffer, BufferDescriptor, BufferUsages,
    Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    TextureView, TextureViewDescriptor,
};

use crate::errors::{Result, NMSRRenderingError};
use crate::high_level::camera::Camera;
use crate::high_level::pipeline::graphics_context::GraphicsContext;

#[derive(Debug)]
pub(crate) struct SceneContextTextures {
    pub(crate) depth_texture: SceneTexture,
    pub(crate) output_texture: SceneTexture,
}

#[derive(Debug)]
pub struct SceneContext {
    pub transform_matrix_buffer: Buffer,
    pub transform_bind_group: BindGroup,
    pub(crate) textures: Option<SceneContextTextures>,
}

#[derive(Debug)]
pub struct SceneTexture {
    pub(crate) texture: Texture,
    pub(crate) view: TextureView,
}

impl SceneTexture {
    pub async fn copy_texture_from_gpu(
        &self,
        graphics_context: &GraphicsContext,
        width: u32,
        height: u32,
    ) -> Result<RgbaImage> {
        let device = &graphics_context.device;
        let queue = &graphics_context.queue;

        let u32_size = mem::size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * width * height) as wgpu::BufferAddress;
        let output_buffer_desc = BufferDescriptor {
            size: output_buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            label: None,
            mapped_at_creation: false,
        };
        let output_buffer = device.create_buffer(&output_buffer_desc);

        let mut encoder = device.create_command_encoder(&Default::default());

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(u32_size * width),
                    rows_per_image: Some(height),
                },
            },
            Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        async fn read_buffer(
            device: &wgpu::Device,
            output_buffer: &wgpu::Buffer,
            width: u32,
            height: u32,
        ) -> Result<RgbaImage> {
            let buffer_slice = output_buffer.slice(..);

            let (tx, rx) = channel();
            buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
                tx.send(result).unwrap();
            });
            device.poll(wgpu::Maintain::Wait);
            rx.await??;

            let data = buffer_slice.get_mapped_range();

            RgbaImage::from_raw(width, height, data.to_vec()).ok_or(NMSRRenderingError::ImageFromRawError)
        }

        read_buffer(device, &output_buffer, width, height).await
    }
}

impl SceneContext {
    pub fn new(context: &GraphicsContext) -> Self {
        let device = &context.device;

        let (transform_matrix_buffer, transform_bind_group) =
            create_transform_buffer_and_bind_group(device, context);

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

    pub(crate) fn upload_texture(
        context: &GraphicsContext,
        image: &RgbaImage,
        label: Option<&str>,
    ) -> SceneTexture {
        let image: RgbaImage = image.convert();

        let texture = context.device.create_texture_with_data(
            &context.queue,
            &TextureDescriptor {
                size: wgpu::Extent3d {
                    width: image.width(),
                    height: image.height(),
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8UnormSrgb,
                usage: TextureUsages::TEXTURE_BINDING,
                label,
                view_formats: &[],
            },
            image.as_raw(),
        );
        let view = texture.create_view(&Default::default());

        SceneTexture { texture, view }
    }

    pub(crate) fn try_textures(&self) -> Result<&SceneContextTextures> {
        self.textures
            .as_ref()
            .ok_or(NMSRRenderingError::SceneContextTexturesNotInitialized)
    }

    pub async fn copy_output_texture(
        &self,
        graphics_context: &GraphicsContext,
        width: u32,
        height: u32,
    ) -> Result<RgbaImage> {
        let textures = self.try_textures()?;
        let output_texture = &textures.output_texture;

        output_texture.copy_texture_from_gpu(graphics_context, width, height).await
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

fn create_transform_buffer_and_bind_group(
    device: &wgpu::Device,
    context: &GraphicsContext,
) -> (Buffer, BindGroup) {
    let transform_matrix_buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("Transform Matrix Buffer"),
        contents: bytemuck::cast_slice(Mat4::IDENTITY.as_ref()),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let transform_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some("Transform Bind Group"),
        layout: &context.layouts.transform_bind_group_layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: transform_matrix_buffer.as_entire_binding(),
        }],
    });
    (transform_matrix_buffer, transform_bind_group)
}
