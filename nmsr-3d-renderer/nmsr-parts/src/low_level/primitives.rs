use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct Vertex {
    /// The position of the vertex
    position: Vec3,
    /// The uv coordinates of the vertex
    uv: Vec2,
}

impl Vertex {
    pub(crate) fn new(position: Vec3, uv: Vec2) -> Self {
        Vertex {
            position,
            uv,
        }
    }
}

pub(crate) trait PartPrimitive {
    /// Returns the vertices of the primitive
    fn get_vertices(&self) -> Vec<Vertex>;

    /// Returns the indices of the vertices of the primitive
    /// in the order they should be drawn
    fn get_indices(&self) -> Vec<u16>;
}
