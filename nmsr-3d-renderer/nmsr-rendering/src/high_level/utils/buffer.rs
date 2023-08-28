use bytemuck::Pod;
use tracing::instrument;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, BufferUsages,
};

#[instrument(skip(device, layout, value))]
pub fn create_buffer_and_bind_group<T: Pod>(
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
