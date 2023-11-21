use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Mat4};

pub type VertexUvCoordinates = Vec2;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vertex {
    /// The position of the vertex
    pub position: Vec3,
    /// The uv coordinates of the vertex
    pub uv: VertexUvCoordinates,
    pub normal: Vec3,
}

impl Vertex {
    pub fn new(position: Vec3, uv: VertexUvCoordinates, normal: Vec3) -> Self {
        Vertex { position, uv, normal }
    }
    
    pub(crate) fn transform(&self, model_transform: Mat4) -> Self {
        if model_transform == Mat4::IDENTITY {
            return *self;
        }
        
        let normal = model_transform.transform_vector3(self.normal).normalize();
        
        Vertex {
            position: model_transform.transform_point3(self.position),
            uv: self.uv,
            normal
        }
    }
}
