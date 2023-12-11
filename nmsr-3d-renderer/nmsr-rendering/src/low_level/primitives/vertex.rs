use bytemuck::{Pod, Zeroable};
use glam::{Vec2, Vec3, Affine3A};

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
    
    pub(crate) fn transform(&self, model_transform: Affine3A) -> Self {
        if model_transform == Affine3A::IDENTITY {
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
