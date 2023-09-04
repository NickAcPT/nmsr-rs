use super::{scene::Size, GraphicsContext};

use image::RgbaImage;
use tracing::instrument;
use wgpu::{
    Buffer, Texture, TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
};

#[derive(Debug)]
pub struct SceneTexture {
    pub(crate) texture: Texture,
    pub(crate) view: TextureView,
}
#[derive(Debug, Clone)]
pub struct BufferDimensions {
    pub height: usize,
    pub unpadded_bytes_per_row: usize,
    pub padded_bytes_per_row: u32,
}

impl BufferDimensions {
    #[allow(dead_code)]
    pub fn new(width: usize, height: usize, pixel_unit: usize) -> Self {
        let bytes_per_pixel = pixel_unit;
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
    pub(crate) camera_size: Size,
    pub(crate) viewport_size: Size,
}

pub fn premultiply_alpha(image: &mut RgbaImage) {
    for pixel in image.pixels_mut() {
        let alpha = pixel[3] as f32 / 255.0;
        pixel[0] = (pixel[0] as f32 * alpha) as u8;
        pixel[1] = (pixel[1] as f32 * alpha) as u8;
        pixel[2] = (pixel[2] as f32 * alpha) as u8;
    }
}

pub fn unmultiply_alpha(image: &mut [u8]) {
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
pub fn create_texture(
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
