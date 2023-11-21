use glam::Vec3;
use image::{imageops, ImageBuffer, Luma, Rgb32FImage, RgbaImage};
use nmsr_rendering::{
    high_level::{
        parts::{provider::{
            minecraft::MinecraftPlayerPartsProvider, PartsProvider, PlayerPartProviderContext, PlayerPartsProvider,
        }, part::Part},
        utils::parts::primitive_convert, types::PlayerBodyPartType, IntoEnumIterator,
    },
    low_level::primitives::{
        mesh::{PrimitiveDispatch, Mesh},
        part_primitive::PartPrimitive,
        quad::Quad,
        vertex::{Vertex, VertexUvCoordinates},
    },
};

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
        let context: PlayerPartProviderContext<()> = PlayerPartProviderContext {
            model: nmsr_rendering::high_level::model::PlayerModel::Alex,
            has_hat_layer: true,
            has_layers: true,
            has_cape: false,
            arm_rotation: 10.0,
            shadow_y_pos: None,
            shadow_is_square: false,
            armor_slots: None,
            ears_features: None,
        };
        
        let providers = [
            PlayerPartsProvider::Minecraft,
            PlayerPartsProvider::Ears,
        ];

        let parts = providers
            .iter()
            .flat_map(|provider| { 
                PlayerBodyPartType::iter().flat_map(|part| provider.get_parts(&context, part))
             })
            .collect::<Vec<Part>>();
        
        let parts = parts
            .into_iter()
            .map(|p| primitive_convert(&p))
            .collect::<Vec<_>>();
        
        let part = Mesh::new(parts);

        let depth_buffer = ImageBuffer::from_raw(
            size.width,
            size.height,
            [1.0].repeat((size.width * size.height) as usize),
        )
        .unwrap();

        Self {
            size,
            textures: Textures {
                depth_buffer,
                output: RgbaImage::new(size.width, size.height),
            },
            primitive: PrimitiveDispatch::Mesh(part),
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

    pub(crate) fn draw(&mut self, state: &ShaderState) -> () {
        self.draw_primitives(state)
    }
}
