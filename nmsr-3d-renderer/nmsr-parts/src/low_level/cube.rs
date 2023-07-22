use glam::{Affine3A, Vec2, Vec3};
use crate::low_level::primitives::{PartPrimitive, Vertex};

use crate::low_level::quad::Quad;

struct Cube {
    quads: Vec<Quad>,
    transform: Affine3A,
}

impl Cube {
    /// Create a new cube with the given parameters
    pub(crate) fn new(
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
        let left_x = center.x - size.x / 2.0;
        let right_x = center.x + size.x / 2.0;

        let bottom_y = center.y - size.y / 2.0;
        let top_y = center.y + size.y / 2.0;

        let front_z = center.z - size.z / 2.0;
        let back_z = center.z + size.z / 2.0;

        let front_quad = Quad::new(
            Vec3::new(left_x, top_y, front_z),
            Vec3::new(right_x, top_y, front_z),
            Vec3::new(left_x, bottom_y, front_z),
            Vec3::new(right_x, bottom_y, front_z),
            front_uv,
            back_uv,
        );

        let back_quad = Quad::new(
            Vec3::new(left_x, top_y, back_z),
            Vec3::new(right_x, top_y, back_z),
            Vec3::new(left_x, bottom_y, back_z),
            Vec3::new(right_x, bottom_y, back_z),
            back_uv,
            front_uv,
        );

        let top_quad = Quad::new(
            Vec3::new(left_x, top_y, front_z),
            Vec3::new(right_x, top_y, front_z),
            Vec3::new(left_x, top_y, back_z),
            Vec3::new(right_x, top_y, back_z),
            top_uv,
            bottom_uv,
        );

        let bottom_quad = Quad::new(
            Vec3::new(left_x, bottom_y, front_z),
            Vec3::new(right_x, bottom_y, front_z),
            Vec3::new(left_x, bottom_y, back_z),
            Vec3::new(right_x, bottom_y, back_z),
            bottom_uv,
            top_uv,
        );

        let left_quad = Quad::new(
            Vec3::new(left_x, top_y, front_z),
            Vec3::new(left_x, top_y, back_z),
            Vec3::new(left_x, bottom_y, front_z),
            Vec3::new(left_x, bottom_y, back_z),
            left_uv,
            right_uv,
        );

        let right_quad = Quad::new(
            Vec3::new(right_x, top_y, front_z),
            Vec3::new(right_x, top_y, back_z),
            Vec3::new(right_x, bottom_y, front_z),
            Vec3::new(right_x, bottom_y, back_z),
            right_uv,
            left_uv,
        );

        Cube {
            quads: vec![
                front_quad,
                back_quad,
                top_quad,
                bottom_quad,
                left_quad,
                right_quad,
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
        self.quads.iter().flat_map(|quad| quad.get_indices()).collect()
    }
}