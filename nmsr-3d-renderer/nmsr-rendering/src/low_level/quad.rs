use glam::{Vec2, Vec3};

use crate::low_level::primitives::PartPrimitive;
use crate::low_level::vertex::Vertex;

pub struct Quad {
    top_left: Vertex,
    top_right: Vertex,
    bottom_left: Vertex,
    bottom_right: Vertex,
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

    /// Create a new quad with the given vertex positions and uv coordinates
    pub fn new(
        top_left: Vec3,
        top_right: Vec3,
        bottom_left: Vec3,
        bottom_right: Vec3,
        top_left_uv: Vec2,
        bottom_right_uv: Vec2,
    ) -> Self {
        Quad {
            top_left: Vertex::new(top_left, top_left_uv),
            top_right: Vertex::new(top_right, Vec2::new(bottom_right_uv.x, top_left_uv.y)),
            bottom_left: Vertex::new(bottom_left, Vec2::new(top_left_uv.x, bottom_right_uv.y)),
            bottom_right: Vertex::new(bottom_right, bottom_right_uv),
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
}
