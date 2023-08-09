use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Mat4};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    /// The position of the vertex
    pub position: Vec3,
    /// The uv coordinates of the vertex
    uv: Vec2,
}

impl Vertex {
    pub fn new(position: Vec3, uv: Vec2) -> Self {
        Vertex { position, uv }
    }
    
    pub(crate) fn transform(&self, model_trasform: Mat4) -> Self {
        if model_trasform == Mat4::IDENTITY {
            return *self;
        }
        
        Vertex {
            position: model_trasform.transform_point3(self.position),
            uv: self.uv,
        }
    }
}
