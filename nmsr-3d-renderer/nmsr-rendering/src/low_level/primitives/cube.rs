use glam::{Vec3, Affine3A};

use crate::low_level::primitives::mesh::Mesh;
use crate::low_level::primitives::part_primitive::PartPrimitive;
use crate::low_level::primitives::quad::Quad;
use crate::low_level::primitives::vertex::Vertex;

use super::vertex::VertexUvCoordinates;

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
    
    fn get_vertices_grouped(&self) -> Vec<[Vertex; 3]> {
        self.mesh.get_vertices_grouped()
    }
}

impl Cube {
    //noinspection DuplicatedCode
    /// Create a new cube with the given parameters
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        center: Vec3,
        size: Vec3,
        model_transform: Affine3A,
        front_face_uv: [VertexUvCoordinates; 4],
        back_face_uv: [VertexUvCoordinates; 4],
        top_face_uv: [VertexUvCoordinates; 4],
        bottom_face_uv: [VertexUvCoordinates; 4],
        left_face_uv: [VertexUvCoordinates; 4],
        right_face_uv: [VertexUvCoordinates; 4],
    ) -> Self {
        let small = 0f32; //1.0 / 256.0;

        // Generate the quads for the cube. Y is up. Z is front. X is left.
        let x_left = center.x - size.x / 2.0;
        let x_right = center.x + size.x / 2.0;

        let y_up = center.y + size.y / 2.0;
        let y_down = center.y - size.y / 2.0;

        let z_front = center.z - size.z / 2.0;
        let z_back = center.z + size.z / 2.0;

        // Front is facing towards the negative Z axis (North).
        let front_quad = Quad::new_with_normal(
            Vec3::new(x_right, y_up, z_front + small),
            Vec3::new(x_left, y_up, z_front + small),
            Vec3::new(x_right, y_down, z_front + small),
            Vec3::new(x_left, y_down, z_front + small),
            front_face_uv[0],
            front_face_uv[1],
            front_face_uv[2],
            front_face_uv[3],
            [0.0, 0.0, -1.0].into(),
        );

        // Back is facing towards the positive Z axis (South).
        let back_quad = Quad::new_with_normal(
            Vec3::new(x_left, y_up, z_back - small),
            Vec3::new(x_right, y_up, z_back - small),
            Vec3::new(x_left, y_down, z_back - small),
            Vec3::new(x_right, y_down, z_back - small),
            back_face_uv[0],
            back_face_uv[1],
            back_face_uv[2],
            back_face_uv[3],
            [0.0, 0.0, 1.0].into(),
        );

        // Top is facing towards the positive Y axis (Up).
        let top_quad = Quad::new_with_normal(
            Vec3::new(x_right, y_up + small, z_back),
            Vec3::new(x_left, y_up + small, z_back),
            Vec3::new(x_right, y_up + small, z_front),
            Vec3::new(x_left, y_up + small, z_front),
            top_face_uv[0],
            top_face_uv[1],
            top_face_uv[2],
            top_face_uv[3],
            [0.0, 1.0, 0.0].into(),
        );

        // Bottom is facing towards the negative Y axis (Down).
        let bottom_quad = Quad::new_with_normal(
            Vec3::new(x_left, y_down - small, z_back),
            Vec3::new(x_right, y_down - small, z_back),
            Vec3::new(x_left, y_down - small, z_front),
            Vec3::new(x_right, y_down - small, z_front),
            bottom_face_uv[0],
            bottom_face_uv[1],
            bottom_face_uv[2],
            bottom_face_uv[3],
            [0.0, -1.0, 0.0].into(),
        );

        // Left is facing towards the negative X axis (West).
        let left_quad: Quad = Quad::new_with_normal(
            Vec3::new(x_left - small, y_up, z_front),
            Vec3::new(x_left - small, y_up, z_back),
            Vec3::new(x_left - small, y_down, z_front),
            Vec3::new(x_left - small, y_down, z_back),
            left_face_uv[0],
            left_face_uv[1],
            left_face_uv[2],
            left_face_uv[3],
            [-1.0, 0.0, 0.0].into(),
        );

        // Right is facing towards the positive X axis (East).
        let right_quad = Quad::new_with_normal(
            Vec3::new(x_right + small, y_up, z_back),
            Vec3::new(x_right + small, y_up, z_front),
            Vec3::new(x_right + small, y_down, z_back),
            Vec3::new(x_right + small, y_down, z_front),
            right_face_uv[0],
            right_face_uv[1],
            right_face_uv[2],
            right_face_uv[3],
            [1.0, 0.0, 0.0].into(),
        );

        Cube {
            mesh: Mesh::new_with_transform(vec![
                back_quad.into(),
                top_quad.into(),
                bottom_quad.into(),
                left_quad.into(),
                right_quad.into(),
                front_quad.into(),
            ], model_transform),
        }
    }
}
