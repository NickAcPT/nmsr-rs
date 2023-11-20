use glam::Vec3;
use image::{RgbaImage, Rgb32FImage, Luma, ImageBuffer, imageops};
use nmsr_rendering::low_level::primitives::{vertex::{Vertex, VertexUvCoordinates}, part_primitive::PartPrimitive, mesh::PrimitiveDispatch, quad::Quad};

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

pub struct Textures {
    pub depth_buffer: ImageBuffer<Luma<f32>, Vec<f32>>,
    pub output: RgbaImage,
}

pub struct RenderEntry {
    pub size: Size,
    pub textures: Textures,
    pub primitive: PrimitiveDispatch,
}

impl RenderEntry {
    pub fn new(size: Size) -> Self {
        let full_quad = Quad::new_with_normal(
            Vec3::new(-1.0, 1.0, 0.0),
            Vec3::new(1.0, 1.0, 0.0),
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
        }
    }
    
    pub fn dump(&self) {
        self.textures.output.save("output.png").unwrap();
    }

    pub(crate) fn draw(&mut self, state: &ShaderState) -> () {
        self.draw_primitives(state)
    }
}
