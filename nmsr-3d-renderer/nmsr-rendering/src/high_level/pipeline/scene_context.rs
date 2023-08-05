use std::borrow::Cow;
use std::mem::{self, size_of};
use std::sync::Arc;

use glam::{Mat4};
use wgpu::util::{DeviceExt, BufferInitDescriptor};
use wgpu::BindGroupDescriptor;

use crate::high_level::pipeline::graphics_context::GraphicsContext;

#[derive(Debug)]
pub struct SceneContext {
    pub context: Arc<GraphicsContext>,
    pub transform_matrix_buffer: wgpu::Buffer,
    pub transform_bind_group: wgpu::BindGroup,
}

impl SceneContext {
    
    pub fn new(context: Arc<GraphicsContext>) -> Self {
        let device = &context.device;

        let transform_matrix_buffer =
            device.create_buffer_init(&BufferInitDescriptor {
                label: Some("Transform Matrix Buffer"),
                contents: bytemuck::cast_slice(Mat4::IDENTITY.as_ref()),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let transform_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Transform Bind Group"),
            layout: &context.transform_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: transform_matrix_buffer.as_entire_binding(),
            }],
        });
        
        Self {
            context,
            transform_bind_group,
            transform_matrix_buffer
        }
    }
}
