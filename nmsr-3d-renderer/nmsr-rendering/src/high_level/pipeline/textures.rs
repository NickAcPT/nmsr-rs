use wgpu::{Buffer, Texture, TextureView};

use super::scene::Size;

#[derive(Debug)]
pub struct SceneTexture {
    pub(crate) texture: Texture,
    pub(crate) view: TextureView,
}
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
