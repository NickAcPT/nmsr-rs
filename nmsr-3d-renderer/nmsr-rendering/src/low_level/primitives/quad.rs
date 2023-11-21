use glam::Vec3;

use crate::low_level::primitives::part_primitive::PartPrimitive;
use crate::low_level::primitives::vertex::Vertex;

use super::vertex::VertexUvCoordinates;

pub struct Quad {
    pub top_left: Vertex,
    pub top_right: Vertex,
    pub bottom_left: Vertex,
    pub bottom_right: Vertex,
}

impl Quad {
    /// Create a new quad with the given vertices
    pub fn new_from_vec(
        top_left: Vertex,
        top_right: Vertex,
        bottom_left: Vertex,
        bottom_right: Vertex,
    ) -> Self {
        Quad {
            top_left,
            top_right,
            bottom_left,
            bottom_right,
        }
    }
    
    #[allow(clippy::too_many_arguments)]
    pub fn new_with_normal(
        top_left: Vec3,
        top_right: Vec3,
        bottom_left: Vec3,
        bottom_right: Vec3,
        top_left_uv: VertexUvCoordinates,
        top_right_uv: VertexUvCoordinates,
        bottom_left_uv: VertexUvCoordinates,
        bottom_right_uv: VertexUvCoordinates,
        normal: Vec3,
    ) -> Self {
        Quad {
            top_left: Vertex::new(top_left, top_left_uv, normal),
            top_right: Vertex::new(top_right, top_right_uv, normal),
            bottom_left: Vertex::new(bottom_left, bottom_left_uv, normal),
            bottom_right: Vertex::new(bottom_right, bottom_right_uv, normal),
        }
    }
}

impl PartPrimitive for Quad {
    fn get_vertices(&self) -> Vec<Vertex> {
        vec![
            self.top_left,
            self.top_right,
            self.bottom_left,
            self.bottom_right,
        ]
    }

    fn get_indices(&self) -> Vec<u16> {
        // We're going in clockwise order
        vec![
            // First triangle (bottom left, top left, bottom right)
            2, 0, 3, // Second triangle (top left, top right, bottom right)
            0, 1, 3,
        ]
    }

    fn get_vertices_grouped(&self) -> Vec<[Vertex; 3]> {
        vec![
            [self.bottom_left, self.top_left, self.bottom_right],
            [self.top_left, self.top_right, self.bottom_right],
        ]
    }
}
