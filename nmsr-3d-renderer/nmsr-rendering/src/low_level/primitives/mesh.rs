use glam::Mat4;

use crate::low_level::primitives::part_primitive::PartPrimitive;
use crate::low_level::primitives::vertex::Vertex;

pub struct Mesh {
    primitives: Vec<Box<dyn PartPrimitive>>,
    model_transform: Mat4,
}

impl Mesh {
    pub fn new(primitives: Vec<Box<dyn PartPrimitive>>) -> Self {
        Mesh { primitives, model_transform: Mat4::IDENTITY }
    }
    pub fn new_with_transform(primitives: Vec<Box<dyn PartPrimitive>>, model_transform: Mat4) -> Self {
        Mesh { primitives, model_transform }
    }
}

impl PartPrimitive for Mesh {
    fn get_vertices(&self) -> Vec<Vertex> {
        self.primitives
            .iter()
            .flat_map(|quad| quad.get_vertices())
            .map(|v| v.transform(self.model_transform))
            .collect()
    }

    fn get_indices(&self) -> Vec<u16> {
        // Go through all primitives, get their indices, and add them to the list
        // Be sure to offset the indices by the number of vertices we've already added
        let mut indices = Vec::new();
        let mut offset = 0;

        for quad in &self.primitives {
            let quad_indices = quad.get_indices();
            indices.extend(quad_indices.iter().map(|index| index + offset));
            offset += quad.get_vertices().len() as u16;
        }

        indices
    }
}
