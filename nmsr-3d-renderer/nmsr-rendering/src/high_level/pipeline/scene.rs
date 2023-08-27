use std::{
    collections::HashMap,
    fmt::Debug,
    ops::{Deref, DerefMut},
};

use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat, Vec2, Vec3};
use image::RgbaImage;
use itertools::Itertools;
use nmsr_player_parts::{
    parts::{
        part::{Part, PartAnchorInfo},
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::FaceUv,
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};
use tracing::{instrument, trace_span};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    AddressMode, BindGroupDescriptor, BindGroupEntry, Color, CommandEncoder, Extent3d, FilterMode,
    IndexFormat, LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
    SamplerDescriptor, TextureView,
};

use crate::high_level::pipeline::SceneContext;
use crate::{
    errors::{NMSRRenderingError, Result},
    low_level::primitives::{cube::Cube, mesh::Mesh, part_primitive::PartPrimitive},
};
use crate::{high_level::camera::Camera, low_level::primitives::mesh::PrimitiveDispatch};

use super::{GraphicsContext, SceneContextWrapper, SceneTexture};

#[derive(Debug, Copy, Clone)]
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

#[derive(Copy, Clone, Pod, Zeroable)]
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
    pub fn new<P>(
        graphics_context: &GraphicsContext,
        mut scene_context: T,
        mut camera: Camera,
        sun: SunInformation,
        viewport_size: Size,
        part_context: &PlayerPartProviderContext,
        body_parts: P,
    ) -> Self
    where
        P: IntoIterator<Item = PlayerBodyPartType> + Debug,
    {
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

        Self {
            camera,
            viewport_size,
            scene_context,
            textures: HashMap::new(),
            computed_body_parts,
            sun_information: Default::default(),
        }
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
    }

    pub fn viewport_size_mut(&mut self) -> &mut Size {
        &mut self.viewport_size
    }

    pub fn parts(&self) -> &[Part] {
        &self.computed_body_parts
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
    fn collect_player_parts<P>(
        part_provider_context: &PlayerPartProviderContext,
        body_parts: P,
    ) -> Vec<Part>
    where
        P: IntoIterator<Item = PlayerBodyPartType> + Debug,
    {
        let mut parts = body_parts
            .into_iter()
            .flat_map(|part| PlayerPartsProvider::Minecraft.get_parts(part_provider_context, part))
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

            let texture_sampler = device.create_sampler(&SamplerDescriptor {
                label: Some(texture.into()),
                address_mode_u: AddressMode::ClampToEdge,
                address_mode_v: AddressMode::ClampToEdge,
                address_mode_w: AddressMode::ClampToEdge,
                mag_filter: FilterMode::Nearest,
                min_filter: FilterMode::Nearest,
                mipmap_filter: FilterMode::Nearest,
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
                        resource: wgpu::BindingResource::TextureView(&texture_view),
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

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(format!("Render pass for {}", texture).as_str()),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &*attachment,
                    resolve_target,
                    ops: Operations {
                        load: load_op,
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &textures.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: depth_load_opt,
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });

            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, transform_bind_group, &[]);
            rpass.set_bind_group(1, &texture_sampler_bind_group, &[]);
            rpass.set_bind_group(2, sun_bind_group, &[]);
            rpass.set_index_buffer(index_buf.slice(..), IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, vertex_buf.slice(..));
            rpass.draw_indexed(0..(index_data.len() as u32), 0, 0..1);

            load_op = LoadOp::Load;
            depth_load_opt = LoadOp::Load;
        }

        queue.submit(Some(encoder.finish()));

        // Explicitly drop the smaa frame so that it is resolved before we copy it to the output buffer.
        drop(smaa_frame);

        let u32_size = std::mem::size_of::<u32>() as u32;

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
                    bytes_per_row: Some(u32_size * self.viewport_size.width),
                    rows_per_image: Some(self.viewport_size.height),
                },
            },
            Extent3d {
                width: self.viewport_size.width,
                height: self.viewport_size.height,
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

    pub async fn copy_output_texture(&self, graphics_context: &GraphicsContext) -> Result<Vec<u8>> {
        self.scene_context
            .copy_output_texture(graphics_context)
            .await
    }

    fn update_scene_context(
        camera: &mut Camera,
        sun: &SunInformation,
        viewport_size: Size,
        scene_context: &mut SceneContext,
        graphics_context: &GraphicsContext,
    ) {
        camera.set_aspect_ratio(viewport_size.width as f32 / viewport_size.height as f32);
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
        self.computed_body_parts = Self::collect_player_parts(part_context, body_parts);

        self.parts()
    }
}

pub fn primitive_convert(part: &Part) -> PrimitiveDispatch {
    (match part {
        Part::Cube {
            position,
            rotation,
            anchor,
            size,
            face_uvs,
            ..
        } => {
            // Compute center of cube
            let center = *position + *size / 2.0;

            let translation = anchor
                .or_else(|| Some(PartAnchorInfo { anchor: Vec3::ZERO }))
                .unwrap();

            let translation_mat = Mat4::from_translation(translation.anchor);
            let neg_translation_mat = Mat4::from_translation(-translation.anchor);

            let rotation = Mat4::from_quat(Quat::from_euler(
                glam::EulerRot::YXZ,
                rotation.y.to_radians(),
                rotation.x.to_radians(),
                rotation.z.to_radians(),
            ));

            let model_transform = translation_mat * rotation * neg_translation_mat;

            let texture_size = part.get_texture().get_texture_size();

            Cube::new(
                center,
                *size,
                model_transform,
                uv(&face_uvs.north, texture_size),
                uv(&face_uvs.south, texture_size),
                uv(&face_uvs.up, texture_size),
                uv(&face_uvs.down, texture_size),
                uv(&face_uvs.west, texture_size),
                uv(&face_uvs.east, texture_size),
            )
        }
        Part::Quad { .. } => {
            unreachable!()
        }
    })
    .into()
}

fn uv(face_uvs: &FaceUv, texture_size: (u32, u32)) -> [Vec2; 2] {
    let texture_size = Vec2::new(texture_size.0 as f32, texture_size.1 as f32);

    let mut top_left = face_uvs.top_left.to_uv(texture_size);
    let mut bottom_right = face_uvs.bottom_right.to_uv(texture_size);

    let small_offset = 0.001; //Vec2::ONE / texture_size / 32.0;//001;

    top_left += small_offset;
    bottom_right -= small_offset;
    [top_left, bottom_right]
}
