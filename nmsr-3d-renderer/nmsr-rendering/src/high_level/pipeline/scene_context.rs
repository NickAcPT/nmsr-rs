use bytemuck::Pod;
use std::mem;
use tokio::sync::oneshot::channel;
use tracing::{instrument, trace_span, Span};

use glam::Mat4;
use image::buffer::ConvertBuffer;
use image::RgbaImage;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, BufferDescriptor,
    BufferUsages, Extent3d, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView,
};

use crate::errors::{NMSRRenderingError, Result};
use crate::high_level::camera::Camera;
use crate::high_level::pipeline::graphics_context::GraphicsContext;

use super::scene::SunInformation;

#[derive(Debug)]
pub(crate) struct SceneContextTextures {
    pub(crate) depth_texture: SceneTexture,
    pub(crate) output_texture: SceneTexture,
    pub(crate) multisampled_output_texture: Option<SceneTexture>,
    pub(crate) texture_output_buffer: Buffer
}

#[derive(Debug)]
pub struct SceneContext {
    pub transform_matrix_buffer: Buffer,
    pub transform_bind_group: BindGroup,
    pub sun_information_buffer: Buffer,
    pub sun_information_bind_group: BindGroup,
    pub(crate) textures: Option<SceneContextTextures>,
}

#[derive(Debug)]
pub struct SceneTexture {
    pub(crate) texture: Texture,
    pub(crate) view: TextureView,
}

impl SceneContext {
    pub fn new(context: &GraphicsContext) -> Self {
        let device = &context.device;

        let (transform_matrix_buffer, transform_bind_group) = create_buffer_and_bind_group(
            device,
            "Transform Matrix",
            &context.layouts.transform_bind_group_layout,
            Mat4::IDENTITY.as_ref(),
        );

        let (sun_information_buffer, sun_information_bind_group) = create_buffer_and_bind_group(
            device,
            "Sun",
            &context.layouts.sun_bind_group_layout,
            &[SunInformation::default()],
        );
        
        Self {
            transform_bind_group,
            transform_matrix_buffer,
            sun_information_buffer,
            sun_information_bind_group,
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

    fn set_sun_information(&self, context: &GraphicsContext, sun_information: &SunInformation) {
        let binding = [*sun_information];
        let data = bytemuck::cast_slice(&binding);
        context
            .queue
            .write_buffer(&self.sun_information_buffer, 0, data);
    }

    #[instrument(skip(self, graphics_context, camera, sun, viewport_size))]
    pub(crate) fn init(
        &mut self,
        graphics_context: &GraphicsContext,
        camera: &mut Camera,
        sun: &SunInformation,
        viewport_size: super::scene::Size,
    ) {
        // Setup camera matrix
        self.set_camera_parameters(graphics_context, camera);

        // Setup sun information
        self.set_sun_information(graphics_context, sun);

        // Setup our depth texture
        let depth_texture = create_texture(
            graphics_context,
            viewport_size.width,
            viewport_size.height,
            GraphicsContext::DEPTH_TEXTURE_FORMAT,
            TextureUsages::RENDER_ATTACHMENT,
            Some("Depth Texture"),
            graphics_context.sample_count,
        );

        // Setup our output texture for multisampling if we need to use it
        let multisampled_output_texture = if graphics_context.sample_count > 1 {
            Some(create_texture(
                graphics_context,
                viewport_size.width,
                viewport_size.height,
                graphics_context.texture_format,
                TextureUsages::RENDER_ATTACHMENT,
                Some("MultiSampled Output Texture"),
                graphics_context.sample_count,
            ))
        } else {
            None
        };

        // Setup our output texture
        let output_texture = create_texture(
            graphics_context,
            viewport_size.width,
            viewport_size.height,
            graphics_context.texture_format,
            TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
            Some("Final Output Texture"),
            1,
        );
        
        let u32_size = mem::size_of::<u32>() as u32;
        let output_buffer_size = (u32_size * viewport_size.width * viewport_size.height) as wgpu::BufferAddress;
        let output_buffer_desc = BufferDescriptor {
            size: output_buffer_size,
            usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
            label: Some("Output Texture Buffer"),
            mapped_at_creation: false,
        };
        
        let texture_output_buffer = graphics_context.device.create_buffer(&output_buffer_desc);

        // Save our textures
        self.textures = Some(SceneContextTextures {
            depth_texture,
            output_texture,
            multisampled_output_texture,
            texture_output_buffer
        })
    }

    pub(crate) fn upload_texture(
        context: &GraphicsContext,
        image: &RgbaImage,
        label: Option<&str>,
    ) -> SceneTexture {
        let mut image: RgbaImage = image.convert();

        premultiply_alpha(&mut image);
        
        let format = if context.texture_format.is_srgb() {
            TextureFormat::Rgba8UnormSrgb
        } else {
            TextureFormat::Rgba8Unorm
        };

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
                format,
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
        
    ) -> Result<Vec<u8>> {
        let textures = self.try_textures()?;
        
        Self::read_buffer(&graphics_context.device, &textures.texture_output_buffer).await
    }
    
    #[instrument(skip(device, output_buffer))]
    async fn read_buffer(
        device: &wgpu::Device,
        output_buffer: &wgpu::Buffer,
    ) -> Result<Vec<u8>> {
        let span_guard = trace_span!(parent: Span::current(), "buffer_slice_wait").entered();
        let buffer_slice = output_buffer.slice(..);

        let (tx, rx) = channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            tx.send(result).unwrap();
        });
        device.poll(wgpu::Maintain::Wait);
        rx.await??;
        drop(span_guard);

        let data = buffer_slice.get_mapped_range();

        trace_span!("image_from_raw").in_scope(|| {
            let vec = data.to_vec();

            drop(data);
            output_buffer.unmap();

            Ok(vec)
        })
    }
}

fn premultiply_alpha(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let alpha = pixel[3] as f32 / 255.0;
        pixel[0] = (pixel[0] as f32 * alpha) as u8;
        pixel[1] = (pixel[1] as f32 * alpha) as u8;
        pixel[2] = (pixel[2] as f32 * alpha) as u8;
    }
}

#[instrument(skip(image))]
fn unmultiply_alpha(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let alpha = pixel[3] as f32 / 255.0;
        if alpha > 0.0 {
            pixel[0] = (pixel[0] as f32 / alpha) as u8;
            pixel[1] = (pixel[1] as f32 / alpha) as u8;
            pixel[2] = (pixel[2] as f32 / alpha) as u8;
        }
    }
}

#[instrument(skip(context, usage))]
fn create_texture(
    context: &GraphicsContext,
    width: u32,
    height: u32,
    format: TextureFormat,
    usage: TextureUsages,
    label: Option<&str>,
    sample_count: u32,
) -> SceneTexture {
    let texture = context.device.create_texture(&TextureDescriptor {
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: TextureDimension::D2,
        format,
        usage,
        label,
        view_formats: &[],
    });
    let view = texture.create_view(&Default::default());

    SceneTexture { texture, view }
}

#[instrument(skip(device, layout, value))]
fn create_buffer_and_bind_group<T: Pod>(
    device: &wgpu::Device,
    label: &str,
    layout: &BindGroupLayout,
    value: &[T],
) -> (Buffer, BindGroup) {
    let buffer = device.create_buffer_init(&BufferInitDescriptor {
        label: Some((label.to_owned() + " Buffer").as_str()),
        contents: bytemuck::cast_slice(value),
        usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
    });

    let transform_bind_group = device.create_bind_group(&BindGroupDescriptor {
        label: Some((label.to_owned() + " Bind group").as_str()),
        layout: layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    (buffer, transform_bind_group)
}
