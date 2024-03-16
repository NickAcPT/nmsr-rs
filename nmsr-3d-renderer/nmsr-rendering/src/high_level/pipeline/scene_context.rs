use super::{
    scene::{Size, SunInformation},
    textures::{
        create_texture, premultiply_alpha, BufferDimensions, SceneContextTextures, SceneTexture,
    },
};
use crate::{
    errors::{NMSRRenderingError, Result},
    high_level::{
        camera::Camera,
        pipeline::graphics_context::GraphicsContext,
        utils::buffer::{create_buffer_and_bind_group, read_buffer},
    },
};

use derive_more::{Debug, Deref, DerefMut, From};
use glam::Mat4;
use image::{buffer::ConvertBuffer, RgbaImage};
use smaa::SmaaTarget;
use tracing::{instrument, trace_span};
use wgpu::{
    util::DeviceExt, BindGroup, Buffer, BufferDescriptor, BufferUsages, TextureDescriptor,
    TextureDimension, TextureFormat, TextureUsages,
};

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

        let camera_size = camera.get_size().unwrap_or_default();

        let msaa_sample_count = graphics_context
            .multisampling_strategy
            .get_msaa_sample_count();

        let needs_texture_resize = self
            .textures
            .as_ref()
            .map_or(true, |textures| textures.camera_size != camera_size);

        let needs_output_buffer_resize = self
            .textures
            .as_ref()
            .map_or(true, |textures| textures.viewport_size != viewport_size);

        let output = if needs_output_buffer_resize {
            let output_buffer_dimensions = BufferDimensions::new(
                viewport_size.width as usize,
                viewport_size.height as usize,
                graphics_context
                    .texture_format
                    .block_copy_size(None)
                    .unwrap_or(4) as usize,
            );

            let output_buffer_desc = BufferDescriptor {
                size: output_buffer_dimensions.size(),
                usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
                label: Some("Output Texture Buffer"),
                mapped_at_creation: false,
            };

            let texture_output_buffer = trace_span!("create_output_buffer")
                .in_scope(|| graphics_context.device.create_buffer(&output_buffer_desc));

            Some((output_buffer_dimensions, texture_output_buffer))
        } else {
            None
        };

        if needs_texture_resize {
            let old_textures = self.textures.take();

            let old_output =
                old_textures.map(|t| (t.texture_output_buffer_dimensions, t.texture_output_buffer));

            // Setup our depth texture
            let depth_texture = create_texture(
                graphics_context,
                camera_size.width,
                camera_size.height,
                GraphicsContext::DEPTH_TEXTURE_FORMAT,
                TextureUsages::RENDER_ATTACHMENT,
                Some("Depth Texture"),
                msaa_sample_count,
            );

            // Setup our output texture for multisampling if we need to use it
            let multisampled_output_texture = if msaa_sample_count > 1 {
                Some(create_texture(
                    graphics_context,
                    camera_size.width,
                    camera_size.height,
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
                camera_size.width,
                camera_size.height,
                graphics_context.texture_format,
                TextureUsages::RENDER_ATTACHMENT | TextureUsages::COPY_SRC,
                Some("Final Output Texture"),
                1,
            );

            if let Some(target) = self.smaa_target.as_mut() {
                let _guard = trace_span!("resize_smaa_target").entered();
                target.resize(
                    &graphics_context.device,
                    camera_size.width,
                    camera_size.height,
                );
            } else {
                let _guard = trace_span!("create_smaa_target").entered();
                let smaa_target = SmaaTarget::new(
                    &graphics_context.device,
                    &graphics_context.queue,
                    camera_size.width,
                    camera_size.height,
                    graphics_context.texture_format,
                    graphics_context.multisampling_strategy.get_smaa_mode(),
                );

                self.smaa_target.replace(smaa_target);
            }

            let final_output = output.or(old_output);

            if let Some((texture_output_buffer_dimensions, texture_output_buffer)) = final_output {
                // Save our textures
                self.textures = Some(SceneContextTextures {
                    depth_texture,
                    output_texture,
                    multisampled_output_texture,
                    texture_output_buffer,
                    camera_size,
                    viewport_size,
                    texture_output_buffer_dimensions,
                });
            }
        } else if let Some((texture_output_buffer_dimensions, texture_output_buffer)) = output {
            let textures = self.textures.take();

            if let Some(mut textures) = textures {
                textures.texture_output_buffer = texture_output_buffer;
                textures.texture_output_buffer_dimensions = texture_output_buffer_dimensions;
                textures.viewport_size = viewport_size;

                self.textures = Some(textures);
            }
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
            wgpu::util::TextureDataOrder::LayerMajor,
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
        graphics_context: &GraphicsContext<'_>,
        cleanup_alpha: bool,
    ) -> Result<Vec<u8>> {
        let textures = self.try_textures()?;

        read_buffer(
            &graphics_context.device,
            &textures.texture_output_buffer,
            &textures.texture_output_buffer_dimensions,
            cleanup_alpha,
        )
        .await
    }
}
