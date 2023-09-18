use crate::{
    errors::Result,
    high_level::pipeline::textures::{unmultiply_alpha, BufferDimensions},
};

use bytemuck::Pod;
use tokio::sync::oneshot::channel;
use tracing::{instrument, trace_span};
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BindGroupLayout, Buffer, BufferSlice,
    BufferUsages,
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
        layout,
        entries: &[BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    });
    (buffer, transform_bind_group)
}

#[instrument(name = "buffer_slice_wait", skip(output_buffer, device))]
async fn wait_for_buffer_slice<'a>(
    output_buffer: &'a Buffer,
    device: &wgpu::Device,
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

//#[instrument(skip_all)]
pub async fn read_buffer(
    device: &wgpu::Device,
    output_buffer: &wgpu::Buffer,
    dimensions: &BufferDimensions,
    cleanup_alpha: bool,
) -> Result<Vec<u8>> {
    let buffer_slice = wait_for_buffer_slice(output_buffer, device).await?;

    let data = buffer_slice.get_mapped_range();

    trace_span!("image_from_buffer").in_scope(|| {
        let mut bytes = Vec::with_capacity(dimensions.height * dimensions.unpadded_bytes_per_row);

        for chunk in data.chunks(dimensions.padded_bytes_per_row as usize) {
            bytes.extend_from_slice(&chunk[..dimensions.unpadded_bytes_per_row]);
        }

        drop(data);
        output_buffer.unmap();

        if cleanup_alpha {
            unmultiply_alpha(&mut bytes);
        }

        Ok(bytes)
    })
}
