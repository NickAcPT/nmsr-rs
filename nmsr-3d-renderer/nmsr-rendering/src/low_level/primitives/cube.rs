use glam::{Vec2, Vec3};

use crate::low_level::primitives::mesh::Mesh;
use crate::low_level::primitives::part_primitive::PartPrimitive;
use crate::low_level::primitives::quad::Quad;
use crate::low_level::primitives::vertex::Vertex;

pub struct Cube {
    mesh: Mesh,
}

impl PartPrimitive for Cube {
    fn get_vertices(&self) -> Vec<Vertex> {
        self.mesh.get_vertices()
    }

    fn get_indices(&self) -> Vec<u16> {
        self.mesh.get_indices()
    }
}

impl Cube {
    //noinspection DuplicatedCode
    /// Create a new cube with the given parameters
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: Vec3,
        size: Vec3,
        front_face_uv: [Vec2; 2],
        back_face_uv: [Vec2; 2],
        top_face_uv: [Vec2; 2],
        bottom_face_uv: [Vec2; 2],
        left_face_uv: [Vec2; 2],
        right_face_uv: [Vec2; 2],
    ) -> Self {
        // Generate the quads for the cube. Y is up. Z is front. X is left.
        let x_left = center.x - size.x / 2.0;
        let x_right = center.x + size.x / 2.0;

        let y_up = center.y + size.y / 2.0;
        let y_down = center.y - size.y / 2.0;

        let z_front = center.z - size.z / 2.0;
        let z_back = center.z + size.z / 2.0;

        // Front is facing towards the negative Z axis (North).
        let front_quad = Quad::new(
            Vec3::new(x_right, y_up, z_front),
            Vec3::new(x_left, y_up, z_front),
            Vec3::new(x_right, y_down, z_front),
            Vec3::new(x_left, y_down, z_front),
            front_face_uv[0],
            front_face_uv[1],
        );

        // Back is facing towards the negative Z axis (South).
        let back_quad = Quad::new(
            Vec3::new(x_left, y_up, z_back),
            Vec3::new(x_right, y_up, z_back),
            Vec3::new(x_left, y_down, z_back),
            Vec3::new(x_right, y_down, z_back),
            back_face_uv[0],
            back_face_uv[1],
        );

        // Top is facing towards the positive Y axis (Up).
        let top_quad = Quad::new(
            Vec3::new(x_right, y_up, z_back),
            Vec3::new(x_left, y_up, z_back),
            Vec3::new(x_right, y_up, z_front),
            Vec3::new(x_left, y_up, z_front),
            top_face_uv[0],
            top_face_uv[1],
        );

        // Bottom is facing towards the negative Y axis (Down).
        let bottom_quad = Quad::new(
            Vec3::new(x_right, y_down, z_back),
            Vec3::new(x_left, y_down, z_back),
            Vec3::new(x_right, y_down, z_front),
            Vec3::new(x_left, y_down, z_front),
            bottom_face_uv[0],
            bottom_face_uv[1],
        );

        // Left is facing towards the negative X axis (West).
        let left_quad = Quad::new(
            Vec3::new(x_left, y_up, z_front),
            Vec3::new(x_left, y_up, z_back),
            Vec3::new(x_left, y_down, z_front),
            Vec3::new(x_left, y_down, z_back),
            left_face_uv[0],
            left_face_uv[1],
        );

        // Right is facing towards the positive X axis (East).
        let right_quad = Quad::new(
            Vec3::new(x_right, y_up, z_back),
            Vec3::new(x_right, y_up, z_front),
            Vec3::new(x_right, y_down, z_back),
            Vec3::new(x_right, y_down, z_front),
            right_face_uv[0],
            right_face_uv[1],
        );

        Cube {
            mesh: Mesh::new(vec![
                Box::from(front_quad),
                Box::from(back_quad),
                Box::from(top_quad),
                Box::from(bottom_quad),
                Box::from(left_quad),
                Box::from(right_quad),
            ]),
        }
    }
}
