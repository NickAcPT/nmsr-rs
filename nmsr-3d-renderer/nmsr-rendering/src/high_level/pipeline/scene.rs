use super::{textures::SceneTexture, GraphicsContext, SceneContextWrapper};
use crate::{
    errors::{NMSRRenderingError, Result},
    high_level::{camera::Camera, pipeline::SceneContext, utils::parts::primitive_convert},
    low_level::primitives::{mesh::Mesh, part_primitive::PartPrimitive},
};
use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use image::RgbaImage;
use itertools::Itertools;
use nmsr_player_parts::{
    model::ArmorMaterial,
    parts::{
        part::Part,
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};
use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
};
use tracing::{instrument, trace_span};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, BindGroupDescriptor, BindGroupEntry, Color, CommandEncoder, Extent3d, FilterMode,
    IndexFormat, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    SamplerDescriptor, StoreOp, TextureView,
};

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Scene<T = SceneContextWrapper>
where
    T: Deref<Target = SceneContext> + Send + Sync,
{
    camera: Camera,
    viewport_size: Size,
    scene_context: T,
    textures: HashMap<PlayerPartTextureType, SceneTexture>,
    computed_body_parts: Vec<Part>,
    sun_information: SunInformation,
}

#[derive(Copy, Clone, Pod, Zeroable, Debug)]
#[repr(C)]
pub struct SunInformation {
    pub direction: Vec3,
    pub intensity: f32,
    pub ambient: f32,
    _padding_0: f32,
    _padding_1: f64,
}

impl Default for SunInformation {
    fn default() -> Self {
        Self {
            direction: Vec3::ONE,
            intensity: 1.0,
            ambient: Self::DEFAULT_AMBIENT_LIGHT,
            _padding_0: 0.0,
            _padding_1: 0.0,
        }
    }
}

impl SunInformation {
    pub const DEFAULT_AMBIENT_LIGHT: f32 = 0.1;

    pub fn new(direction: Vec3, intensity: f32, ambient: f32) -> Self {
        Self {
            direction,
            intensity,
            ambient,
            ..Default::default()
        }
    }
}

type ExtraRenderFunc<'a> =
    Box<dyn FnOnce(&TextureView, &mut CommandEncoder, &mut Camera, &mut SunInformation) + 'a>;

impl<T> Scene<T>
where
    T: DerefMut<Target = SceneContext> + Send + Sync,
{
    const RECTANGLE_SHADOW_BYTES: &'static [u8] = include_bytes!("shadow_rectangle.png");
    const SQUARE_SHADOW_BYTES: &'static [u8] = include_bytes!("shadow_square.png");

    pub fn get_shadow_bytes(is_square: bool) -> &'static [u8] {
        if is_square {
            Self::SQUARE_SHADOW_BYTES
        } else {
            Self::RECTANGLE_SHADOW_BYTES
        }
    }

    pub fn new<M: ArmorMaterial>(
        graphics_context: &GraphicsContext,
        mut scene_context: T,
        mut camera: Camera,
        sun: SunInformation,
        viewport_size: Size,
        part_context: &PlayerPartProviderContext<M>,
        body_parts: &[PlayerBodyPartType],
    ) -> Self {
        // Initialize our camera with the viewport size
        Self::update_scene_context(
            &mut camera,
            &sun,
            viewport_size,
            &mut scene_context,
            graphics_context,
        );

        // Compute the body parts we need to render
        let computed_body_parts = Self::collect_player_parts(part_context, body_parts);

        let mut scene = Self {
            camera,
            viewport_size,
            scene_context,
            textures: HashMap::new(),
            computed_body_parts,
            sun_information: sun,
        };

        if part_context.shadow_y_pos.is_some() {
            let shadow_bytes = Self::get_shadow_bytes(part_context.shadow_is_square);

            // We need to render the shadow, so upload the shadow texture already
            let shadow_image =
                image::load_from_memory_with_format(shadow_bytes, image::ImageFormat::Png)
                    .expect("Failed to load shadow texture");

            let shadow_image = shadow_image
                .as_rgba8()
                .expect("Failed to convert shadow texture to RGBA8");

            scene.set_texture(
                graphics_context,
                PlayerPartTextureType::Shadow,
                shadow_image,
            );
        }

        scene
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn sun_information_mut(&mut self) -> &mut SunInformation {
        &mut self.sun_information
    }

    pub fn viewport_size_mut(&mut self) -> &mut Size {
        &mut self.viewport_size
    }

    pub fn parts(&self) -> &[Part] {
        &self.computed_body_parts
    }

    pub fn has_texture(&self, texture_type: PlayerPartTextureType) -> Result<bool> {
        Ok(self.textures.contains_key(&texture_type))
    }

    pub fn set_texture(
        &mut self,
        graphics_context: &GraphicsContext,
        texture_type: PlayerPartTextureType,
        texture: &RgbaImage,
    ) {
        let texture =
            SceneContext::upload_texture(graphics_context, texture, Some(texture_type.into()));
        self.textures.insert(texture_type, texture);
    }

    #[instrument(skip(part_provider_context))]
    fn collect_player_parts<C: ArmorMaterial>(
        part_provider_context: &PlayerPartProviderContext<C>,
        body_parts: &[PlayerBodyPartType],
    ) -> Vec<Part> {
        let providers = [
            PlayerPartsProvider::Minecraft,
            #[cfg(feature = "ears")]
            PlayerPartsProvider::Ears,
        ];

        let mut parts = providers
            .iter()
            .flat_map(|provider| {
                body_parts
                    .iter()
                    .flat_map(|part| provider.get_parts(part_provider_context, *part))
            })
            .collect::<Vec<Part>>();

        // Sort the parts by texture. This allows us to render all parts with the same texture in one go.
        parts.sort_by_key(|p| p.get_texture());

        parts
    }

    pub fn render(&mut self, graphics_context: &GraphicsContext) -> Result<()> {
        self.render_with_extra(graphics_context, None)
    }

    #[instrument(skip(self, graphics_context, extra_rendering))]
    pub fn render_with_extra(
        &mut self,
        graphics_context: &GraphicsContext,
        extra_rendering: Option<ExtraRenderFunc>,
    ) -> Result<()> {
        let pipeline = &graphics_context.pipeline;
        let device = &graphics_context.device;
        let queue = &graphics_context.queue;
        let smaa_target = self.scene_context.smaa_target.take();

        let mut smaa_target = match smaa_target {
            Some(target) => target,
            _ => unreachable!("SMAA target is always initialized"),
        };

        let transform_bind_group = &self.scene_context.transform_bind_group;
        let sun_bind_group = &self.scene_context.sun_information_bind_group;

        let textures = self
            .scene_context
            .textures
            .as_ref()
            .ok_or(NMSRRenderingError::SceneContextTexturesNotInitialized)?;

        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let surface_texture = graphics_context
            .surface
            .as_ref()
            .and_then(|s| s.get_current_texture().ok());

        let surface_texture_view = surface_texture.as_ref().map(|t| {
            t.texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        });

        let final_view = surface_texture_view
            .as_ref()
            .unwrap_or(&textures.output_texture.view);

        let smaa_frame = smaa_target.start_frame(device, queue, final_view);

        let (attachment, resolve_target) =
            if let Some(multisampled_view) = &textures.multisampled_output_texture {
                (&multisampled_view.view, Some(&*smaa_frame))
            } else {
                (&*smaa_frame, None)
            };

        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Scene rendering (NMSR)"),
        });

        let (mut load_op, mut depth_load_opt) =
            (LoadOp::Clear(Color::TRANSPARENT), LoadOp::Clear(1.0));

        for (texture, parts) in &self
            .computed_body_parts
            .iter()
            .group_by(|p| p.get_texture())
        {
            let _pass_span =
                trace_span!("render_pass", texture = Into::<&str>::into(texture)).entered();

            let texture_view = &self
                .textures
                .get(&texture)
                .ok_or(NMSRRenderingError::SceneContextTextureNotSet(texture))?
                .view;

            let filter = if texture.is_shadow() {
                FilterMode::Linear
            } else {
                FilterMode::Nearest
            };

            let texture_sampler = device.create_sampler(&SamplerDescriptor {
                label: Some(texture.into()),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                mipmap_filter: filter,
                lod_min_clamp: 0.0,
                lod_max_clamp: 0.0,
                compare: None,
                anisotropy_clamp: 1,
                border_color: None,
            });

            let texture_sampler_bind_group = device.create_bind_group(&BindGroupDescriptor {
                layout: &graphics_context.layouts.skin_sampler_bind_group_layout,
                entries: &[
                    BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(texture_view),
                    },
                    BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&texture_sampler),
                    },
                ],
                label: Some(texture.into()),
            });

            let parts = parts.collect::<Vec<&Part>>();

            let to_render: Vec<_> = trace_span!("part_convert")
                .in_scope(|| parts.iter().map(|&p| primitive_convert(p)).collect());

            let to_render = Mesh::new(to_render);

            let (vertex_data, index_data) = (to_render.get_vertices(), to_render.get_indices());

            let vertex_buf = trace_span!("vertex_buffer_create").in_scope(|| {
                device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&vertex_data),
                    usage: wgpu::BufferUsages::VERTEX,
                })
            });

            let index_buf = trace_span!("index_buffer_create").in_scope(|| {
                device.create_buffer_init(&BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&index_data),
                    usage: wgpu::BufferUsages::INDEX,
                })
            });

            let store_depth = if !texture.is_shadow() {
                StoreOp::Store
            } else {
                StoreOp::Discard
            };

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(format!("Render pass for {}", texture).as_str()),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: attachment,
                    resolve_target,
                    ops: Operations {
                        load: load_op,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &textures.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: depth_load_opt,
                        store: store_depth,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, transform_bind_group, &[]);
            rpass.set_bind_group(1, &texture_sampler_bind_group, &[]);
            rpass.set_bind_group(2, sun_bind_group, &[]);
            rpass.set_index_buffer(index_buf.slice(..), IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, vertex_buf.slice(..));
            rpass.draw_indexed(0..(index_data.len() as u32), 0, 0..1);

            load_op = LoadOp::Load;
            if store_depth == StoreOp::Store {
                depth_load_opt = LoadOp::Load;
            }
        }

        queue.submit(Some(encoder.finish()));

        // Explicitly drop the smaa frame so that it is resolved before we copy it to the output buffer.
        drop(smaa_frame);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

        encoder.copy_texture_to_buffer(
            wgpu::ImageCopyTexture {
                aspect: wgpu::TextureAspect::All,
                texture: &textures.output_texture.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
            },
            wgpu::ImageCopyBuffer {
                buffer: &textures.texture_output_buffer,
                layout: wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(
                        textures
                            .texture_output_buffer_dimensions
                            .padded_bytes_per_row,
                    ),
                    rows_per_image: None,
                },
            },
            Extent3d {
                width: textures.viewport_size.width,
                height: textures.viewport_size.height,
                depth_or_array_layers: 1,
            },
        );

        queue.submit(Some(encoder.finish()));

        if let Some(extra_rendering) = extra_rendering {
            let mut extra_encoder =
                device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

            extra_rendering(
                final_view,
                &mut extra_encoder,
                &mut self.camera,
                &mut self.sun_information,
            );

            queue.submit(Some(extra_encoder.finish()));
        }

        if let Some(surface_texture) = surface_texture {
            surface_texture.present();
        }

        self.scene_context.smaa_target = Some(smaa_target);

        Ok(())
    }

    pub async fn copy_output_texture(
        &self,
        graphics_context: &GraphicsContext<'_>,
        cleanup_alpha: bool,
    ) -> Result<Vec<u8>> {
        self.scene_context
            .copy_output_texture(graphics_context, cleanup_alpha)
            .await
    }

    fn update_scene_context(
        camera: &mut Camera,
        sun: &SunInformation,
        viewport_size: Size,
        scene_context: &mut SceneContext,
        graphics_context: &GraphicsContext,
    ) {
        if camera.get_size().is_none() {
            camera.set_size(Some(viewport_size));
        }

        scene_context.init(graphics_context, camera, sun, viewport_size);
    }

    pub fn update(&mut self, graphics_context: &GraphicsContext) {
        Self::update_scene_context(
            &mut self.camera,
            &self.sun_information,
            self.viewport_size,
            &mut self.scene_context,
            graphics_context,
        );
    }

    pub fn rebuild_parts(
        &mut self,
        part_context: &PlayerPartProviderContext,
        body_parts: Vec<PlayerBodyPartType>,
    ) -> &[Part] {
        self.computed_body_parts = Self::collect_player_parts(part_context, &body_parts);

        self.parts()
    }
}
