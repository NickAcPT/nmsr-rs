use glam::{Affine3A, Vec2, Vec3};

use crate::low_level::primitives::{PartPrimitive, Vertex};
use crate::low_level::quad::Quad;

pub struct Cube {
    quads: Vec<Quad>,
    transform: Affine3A,
}

impl Cube {
    /// Create a new cube with the given parameters
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: Vec3,
        size: Vec3,
        front_uv: Vec2,
        back_uv: Vec2,
        top_uv: Vec2,
        bottom_uv: Vec2,
        left_uv: Vec2,
        right_uv: Vec2,
    ) -> Self {
        // Generate the quads for the cube. Z- is front, Z+ is back, Y+ is top, Y- is bottom, X+ is left, X- is right
        let left_x = center.x + size.x / 2.0;
        let right_x = center.x - size.x / 2.0;

        let bottom_y = center.y - size.y / 2.0;
        let top_y = center.y + size.y / 2.0;

        let back_z = center.z + size.z / 2.0;
        let front_z = center.z - size.z / 2.0;



        let front_quad = Quad::new(
            /* top left */ Vec3::new(left_x, top_y, front_z),
            /* top right */ Vec3::new(right_x, top_y, front_z),
            /* bottom left */ Vec3::new(left_x, bottom_y, front_z),
            /* bottom right */ Vec3::new(right_x, bottom_y, front_z),
            front_uv,
            back_uv,
        );

        Cube {
            quads: vec![
                front_quad,
            ],
            transform: Affine3A::IDENTITY,
        }
    }
}

impl PartPrimitive for Cube {
    fn get_vertices(&self) -> Vec<Vertex> {
        self.quads.iter().flat_map(|quad| quad.get_vertices()).collect()
    }

    fn get_indices(&self) -> Vec<u16> {
        // Go through all quads, get their indices, and add them to the list
        // Be sure to offset the indices by the number of vertices we've already added
        let mut indices = Vec::new();
        let mut offset = 0;

        for quad in &self.quads {
            let quad_indices = quad.get_indices();
            indices.extend(quad_indices.iter().map(|index| index + offset));
            offset += quad.get_vertices().len() as u16;
        }

        indices
    }
}