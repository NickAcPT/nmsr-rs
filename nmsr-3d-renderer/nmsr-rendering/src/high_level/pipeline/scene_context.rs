use bytemuck::Pod;
use derive_more::{Debug, Deref, DerefMut, From};
use smaa::SmaaTarget;
use tokio::sync::oneshot::channel;
use tracing::{instrument, trace_span};

use glam::Mat4;
use image::buffer::ConvertBuffer;
use image::RgbaImage;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::{
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, BufferDescriptor,
    BufferSlice, BufferUsages, Texture, TextureDescriptor, TextureDimension, TextureFormat,
    TextureUsages, TextureView,
};

use crate::errors::{NMSRRenderingError, Result};
use crate::high_level::camera::Camera;
use crate::high_level::pipeline::graphics_context::GraphicsContext;

use super::scene::{Size, SunInformation};

#[derive(Debug, Clone)]
pub(crate) struct BufferDimensions {
    pub height: usize,
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: u32,
}

impl BufferDimensions {
    #[allow(dead_code)]
    pub fn new(width: usize, height: usize) -> Self {
        let bytes_per_pixel = std::mem::size_of::<u32>();
        let unpadded_bytes_per_row = width * bytes_per_pixel;
        let align: usize = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT as usize;
        let padded_bytes_per_row_padding = (align - unpadded_bytes_per_row % align) % align;
        let padded_bytes_per_row = (unpadded_bytes_per_row + padded_bytes_per_row_padding) as u32;

        Self {
            height,
            unpadded_bytes_per_row,
            padded_bytes_per_row,
        }
    }

    pub fn size(&self) -> u64 {
        self.padded_bytes_per_row as u64 * self.height as u64
    }
}

#[derive(Debug)]
pub(crate) struct SceneContextTextures {
    pub(crate) depth_texture: SceneTexture,
    pub(crate) output_texture: SceneTexture,
    pub(crate) multisampled_output_texture: Option<SceneTexture>,
    pub(crate) texture_output_buffer: Buffer,
    pub(crate) texture_output_buffer_dimensions: BufferDimensions,
    pub(crate) size: Size,
}

#[derive(Debug)]
pub struct SceneContext {
    pub transform_matrix_buffer: Buffer,
    pub transform_bind_group: BindGroup,
    pub sun_information_buffer: Buffer,
    pub sun_information_bind_group: BindGroup,
    pub(crate) textures: Option<SceneContextTextures>,
    #[debug(skip)]
    pub(crate) smaa_target: Option<SmaaTarget>,
}

#[derive(Deref, DerefMut, From)]
pub struct SceneContextWrapper(SceneContext);

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
            smaa_target: None,
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
        viewport_size: Size,
    ) {
        // Setup camera matrix
        self.set_camera_parameters(graphics_context, camera);

        // Setup sun information
        self.set_sun_information(graphics_context, sun);

        let msaa_sample_count = graphics_context
            .multisampling_strategy
            .get_msaa_sample_count();

        let needs_texture_resize = self
            .textures
            .as_ref()
            .map_or(true, |textures| textures.size != viewport_size);

        if needs_texture_resize {
            drop(self.textures.take());

            // Setup our depth texture
            let depth_texture = create_texture(
                graphics_context,
                viewport_size.width,
                viewport_size.height,
                GraphicsContext::DEPTH_TEXTURE_FORMAT,
                TextureUsages::RENDER_ATTACHMENT,
                Some("Depth Texture"),
                msaa_sample_count,
            );

            // Setup our output texture for multisampling if we need to use it
            let multisampled_output_texture = if msaa_sample_count > 1 {
                Some(create_texture(
                    graphics_context,
                    viewport_size.width,
                    viewport_size.height,
                    graphics_context.texture_format,
                    TextureUsages::RENDER_ATTACHMENT,
                    Some("MultiSampled Output Texture"),
                    msaa_sample_count,
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

            if let Some(target) = self.smaa_target.as_mut() {
                let _guard = trace_span!("resize_smaa_target").entered();
                target.resize(
                    &graphics_context.device,
                    viewport_size.width,
                    viewport_size.height,
                );
            } else {
                let _guard = trace_span!("create_smaa_target").entered();
                let smaa_target = SmaaTarget::new(
                    &graphics_context.device,
                    &graphics_context.queue,
                    viewport_size.width,
                    viewport_size.height,
                    graphics_context.texture_format,
                    graphics_context.multisampling_strategy.get_smaa_mode(),
                );

                self.smaa_target.replace(smaa_target);
            }

            let output_buffer_dimensions =
                BufferDimensions::new(viewport_size.width as usize, viewport_size.height as usize);

            let output_buffer_desc = BufferDescriptor {
                size: output_buffer_dimensions.size(),
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                label: Some("Output Texture Buffer"),
                mapped_at_creation: false,
            };

            let texture_output_buffer = trace_span!("create_output_buffer")
                .in_scope(|| graphics_context.device.create_buffer(&output_buffer_desc));

            // Save our textures
            self.textures = Some(SceneContextTextures {
                depth_texture,
                output_texture,
                multisampled_output_texture,
                texture_output_buffer,
                size: viewport_size,
                texture_output_buffer_dimensions: output_buffer_dimensions
            });
        }
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

    pub async fn copy_output_texture(&self, graphics_context: &GraphicsContext) -> Result<Vec<u8>> {
        let textures = self.try_textures()?;

        Self::read_buffer(&graphics_context.device, &textures.texture_output_buffer, &textures.texture_output_buffer_dimensions).await
    }

    #[instrument(skip_all)]
    async fn read_buffer(device: &wgpu::Device, output_buffer: &wgpu::Buffer, dimensions: &BufferDimensions) -> Result<Vec<u8>> {
        let buffer_slice = wait_for_buffer_slice(output_buffer, device).await?;

        let data = buffer_slice.get_mapped_range();

        trace_span!("image_from_raw").in_scope(|| {
            let mut bytes = Vec::with_capacity(dimensions.height * dimensions.unpadded_bytes_per_row);

            for chunk in data.chunks(dimensions.padded_bytes_per_row as usize) {
                bytes.extend_from_slice(&chunk[..dimensions.unpadded_bytes_per_row]);
            }

            drop(data);
            output_buffer.unmap();

            unmultiply_alpha(&mut bytes);

            Ok(bytes)
        })
    }
}

#[instrument(name = "buffer_slice_wait", skip(output_buffer, device))]
async fn wait_for_buffer_slice<'a>(
    output_buffer: &'a Buffer,
    device: &'a wgpu::Device,
) -> Result<BufferSlice<'a>> {
    let buffer_slice = output_buffer.slice(..);
    let (tx, rx) = channel();
    buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
        tx.send(result).unwrap();
    });
    device.poll(wgpu::Maintain::Wait);
    rx.await??;
    Ok(buffer_slice)
}

fn premultiply_alpha(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let alpha = pixel[3] as f32 / 255.0;
        pixel[0] = (pixel[0] as f32 * alpha) as u8;
        pixel[1] = (pixel[1] as f32 * alpha) as u8;
        pixel[2] = (pixel[2] as f32 * alpha) as u8;
    }
}

fn unmultiply_alpha(image: &mut [u8]) {
    for pixel in image.chunks_exact_mut(4) {
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
