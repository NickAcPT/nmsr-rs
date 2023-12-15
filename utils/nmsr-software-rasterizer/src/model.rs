
use std::fmt::Debug;

use image::{ImageBuffer, Luma, RgbaImage};

use crate::shader::ShaderState;

#[derive(Clone, Copy, Debug)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

impl From<(u32, u32)> for Size {
    fn from((width, height): (u32, u32)) -> Self {
        Self { width, height }
    }
}

#[derive(Debug)]
pub struct Textures {
    pub depth_buffer: ImageBuffer<Luma<f32>, Vec<f32>>,
    pub output: RgbaImage,
}

impl Textures {
    pub fn clear_depth(&mut self) {
        let width = self.output.width() as usize;
        let buf = self.depth_buffer.as_flat_samples_mut();
        for full_row in buf.samples.chunks_exact_mut(buf.layout.width as usize) {
            let (row, suffix) = full_row.split_at_mut(width);
            row.fill(1.0);
            suffix.fill(0.0);
        }
    }
}

pub struct RenderEntry {
    pub size: Size,
    pub textures: Textures,
}


impl Debug for RenderEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RenderEntry").finish()
    }
}

impl RenderEntry {
    pub fn new(size: Size) -> Self {
        // Align and pad width for SIMD
        const ALIGN: u32 = 4;
        let depth_width = (size.width + ALIGN - 1) / ALIGN * ALIGN + ALIGN;
        let mut depth_buffer_vec = Vec::with_capacity(depth_width as usize * size.height as usize);
        unsafe {
            depth_buffer_vec.set_len(depth_buffer_vec.capacity());
        }
        let depth_buffer =
            ImageBuffer::from_raw(depth_width, size.height, depth_buffer_vec).unwrap();

        Self {
            size,
            textures: Textures {
                depth_buffer,
                output: RgbaImage::new(size.width, size.height),
            },
        }
        
        /* let full_quad = Quad::new_with_normal(
            Vec3::new(-1.0, 0.0, 0.0),
            Vec3::new(1.0, 0.0, 0.0),
            Vec3::new(-1.0, -1.0, 0.0),
            Vec3::new(1.0, -1.0, 0.0),
            VertexUvCoordinates::new(0.0, 1.0),
            VertexUvCoordinates::new(1.0, 1.0),
            VertexUvCoordinates::new(0.0, 0.0),
            VertexUvCoordinates::new(1.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        );
        
        let depth_buffer = ImageBuffer::from_raw(size.width, size.height, [1.0].repeat((size.width * size.height) as usize)).unwrap();
        
        
        Self {
            size,
            textures: Textures {
                depth_buffer,
                output: RgbaImage::new(size.width, size.height),
            },
            primitive: PrimitiveDispatch::Quad(full_quad)
        } */
    }
        
    pub fn dump(&self) {
        self.textures.output.save("output.png").unwrap();
    }

    pub fn draw(&mut self, state: &ShaderState) {
        self.draw_primitives(state)
    }
}
