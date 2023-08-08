use std::{collections::HashMap, sync::mpsc::channel};

use glam::Vec2;
use image::RgbaImage;
use nmsr_player_parts::{
    parts::{
        part::Part,
        provider::{PartsProvider, PlayerPartProviderContext, PlayerPartsProvider},
        uv::FaceUv,
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};
use strum::IntoEnumIterator;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroupEntry, Color, LoadOp, Operations, RenderPassColorAttachment,
    RenderPassDepthStencilAttachment, IndexFormat, BindGroupDescriptor,
};

use crate::high_level::camera::Camera;
use crate::high_level::pipeline::SceneContext;
use crate::{
    errors::{NMSRRenderingError, Result},
    low_level::primitives::{cube::Cube, mesh::Mesh, part_primitive::PartPrimitive},
};

use super::{GraphicsContext, SceneTexture};

#[derive(Copy, Clone)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

pub struct Scene {
    camera: Camera,
    viewport_size: Size,
    scene_context: SceneContext,
    textures: HashMap<PlayerPartTextureType, SceneTexture>,
    player_part_provider_context: PlayerPartProviderContext,
    computed_body_parts: Vec<Part>,
}

impl Scene {
    pub fn new<T>(
        graphics_context: &GraphicsContext,
        mut scene_context: SceneContext,
        mut camera: Camera,
        viewport_size: Size,
        part_context: &PlayerPartProviderContext,
        body_parts: T,
    ) -> Self
    where
        T: IntoIterator<Item = PlayerBodyPartType>,
    {
        // Initialize our camera with the viewport size
        camera.set_aspect_ratio(viewport_size.width as f32 / viewport_size.height as f32);

        scene_context.init(graphics_context, &mut camera, viewport_size);

        // Compute the body parts we need to render
        let computed_body_parts = Self::collect_player_parts(part_context, body_parts);

        Self {
            camera,
            viewport_size,
            scene_context,
            textures: HashMap::new(),
            player_part_provider_context: *part_context,
            computed_body_parts,
        }
    }

    pub fn scene_context_mut(&mut self) -> &mut SceneContext {
        &mut self.scene_context
    }

    pub fn camera_mut(&mut self) -> &mut Camera {
        &mut self.camera
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

    fn collect_player_parts<T>(
        part_provider_context: &PlayerPartProviderContext,
        body_parts: T,
    ) -> Vec<Part>
    where
        T: IntoIterator<Item = PlayerBodyPartType>,
    {
        body_parts
            .into_iter()
            .flat_map(|part| PlayerPartsProvider::Minecraft.get_parts(part_provider_context, part))
            .collect()
    }

    pub fn render(&self, graphics_context: &GraphicsContext) -> Result<()> {
        let pipeline = &graphics_context.pipeline;
        let device = &graphics_context.device;
        let queue = &graphics_context.queue;
        let transform_bind_group = &self.scene_context.transform_bind_group;

        let skin_texture_view = &self
            .textures
            .get(&PlayerPartTextureType::Skin)
            .ok_or(NMSRRenderingError::SceneContextTextureNotSet(
                PlayerPartTextureType::Skin,
            ))?
            .view;

        let skin_bind_group = device.create_bind_group(&BindGroupDescriptor {
            layout: &graphics_context.layouts.skin_bind_group_layout,
            entries: &[BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(skin_texture_view),
            }],
            label: Some("diffuse_bind_group"),
        });

        let textures = self
            .scene_context
            .textures.as_ref()
            .ok_or(NMSRRenderingError::SceneContextTexturesNotInitialized)?;
        
        let to_render: Vec<_> = self
            .computed_body_parts
            .iter()
            .map(primitive_convert)
            .collect();
        
        let to_render = Mesh::new(to_render);

        let (vertex_data, index_data) = (to_render.get_vertices(), to_render.get_indices());

        let vertex_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&vertex_data),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let index_buf = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("Index Buffer"),
            contents: bytemuck::cast_slice(&index_data),
            usage: wgpu::BufferUsages::INDEX,
        });


        device.push_error_scope(wgpu::ErrorFilter::Validation);

        let mut encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        {
            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main render pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &textures.output_texture.view,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(Color::TRANSPARENT),
                        store: true,
                    },
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &textures.depth_texture.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            
            rpass.set_pipeline(pipeline);
            rpass.set_bind_group(0, transform_bind_group, &[]);
            rpass.set_bind_group(1, &skin_bind_group, &[]);
            rpass.set_index_buffer(index_buf.slice(..), IndexFormat::Uint16);
            rpass.set_vertex_buffer(0, vertex_buf.slice(..));
            rpass.draw_indexed(0..(index_data.len() as u32), 0, 0..1);
        }

        queue.submit(Some(encoder.finish()));
                
        Ok(())
    }
    
    pub async fn copy_output_texture(&self, graphics_context: &GraphicsContext, width: u32, height: u32) -> Result<RgbaImage> {
        self.scene_context.copy_output_texture(graphics_context, width, height).await
    }
}

fn primitive_convert(part: &Part) -> Box<dyn PartPrimitive> {
    Box::new(match part {
        Part::Cube {
            position,
            size,
            face_uvs,
            ..
        } => {
            // Compute center of cube
            let center = *position + *size / 2.0;

            Cube::new(
                center,
                *size,
                uv(&face_uvs.north),
                uv(&face_uvs.south),
                uv(&face_uvs.up),
                uv(&face_uvs.down),
                uv(&face_uvs.west),
                uv(&face_uvs.east),
            )
        }
        Part::Quad { .. } => {
            unreachable!()
        }
    })
}

fn uv(face_uvs: &FaceUv) -> [Vec2; 2] {
    let mut top_left = face_uvs.top_left.to_uv([64f32, 64f32].into());
    let mut bottom_right = face_uvs.bottom_right.to_uv([64f32, 64f32].into());
    let small_offset = 1f32 / 16f32 / 64f32;
    top_left += small_offset;
    bottom_right -= small_offset;
    [top_left, bottom_right]
}
