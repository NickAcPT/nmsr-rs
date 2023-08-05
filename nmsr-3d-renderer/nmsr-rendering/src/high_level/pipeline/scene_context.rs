use glam::Mat4;
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use wgpu::BindGroupDescriptor;

use crate::high_level::pipeline::graphics_context::GraphicsContext;

#[derive(Debug)]
pub struct SceneContext {
    pub transform_matrix_buffer: wgpu::Buffer,
    pub transform_bind_group: wgpu::BindGroup,
}

impl SceneContext {
    pub fn new(context: &GraphicsContext) -> Self {
        let device = &context.device;

        let (transform_matrix_buffer, transform_bind_group) =
            create_transform_buffer_and_bing_group(device, context);

        Self {
            transform_bind_group,
            transform_matrix_buffer,
        }
    }
}

fn create_transform_buffer_and_bing_group(
    device: &wgpu::Device,
    context: &GraphicsContext,
) -> (wgpu::Buffer, wgpu::BindGroup) {
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
