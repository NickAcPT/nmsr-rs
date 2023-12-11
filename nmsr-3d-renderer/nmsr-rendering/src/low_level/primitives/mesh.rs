use glam::Affine3A;

use crate::low_level::primitives::part_primitive::PartPrimitive;
use crate::low_level::primitives::vertex::Vertex;

pub struct Mesh {
    primitives: Vec<PrimitiveDispatch>,
    model_transform: Affine3A,
}

impl Mesh {
    pub fn new(primitives: Vec<PrimitiveDispatch>) -> Self {
        Mesh {
            primitives,
            model_transform: Affine3A::IDENTITY,
        }
    }
    pub fn new_with_transform(primitives: Vec<PrimitiveDispatch>, model_transform: Affine3A) -> Self {
        Mesh {
            primitives,
            model_transform,
        }
    }
}

impl PartPrimitive for Mesh {
    #[inline(always)]
    fn get_vertices(&self) -> Vec<Vertex> {
        self.primitives
            .iter()
            .flat_map(|quad| quad.get_vertices())
            .map(|v| v.transform(self.model_transform))
            .collect()
    }

    #[inline(always)]
    fn get_indices(&self) -> Vec<u16> {
        // Go through all primitives, get their indices, and add them to the list
        // Be sure to offset the indices by the number of vertices we've already added
        let mut indices = Vec::with_capacity(&self.primitives.len() * /* quad index count */ 6 * /* number of faces per cube */ 6);
        let mut offset = 0;

        for quad in &self.primitives {
            let quad_indices = quad.get_indices();
            indices.extend(quad_indices.iter().map(|index| index + offset));
            offset += quad.get_vertices().len() as u16;
        }

        indices
    }
    
    #[inline(always)]
    fn get_vertices_grouped(&self) -> Vec<[Vertex; 3]> {
        self.primitives
            .iter()
            .flat_map(|quad| quad.get_vertices_grouped())
            .map(|v| [v[0].transform(self.model_transform), v[1].transform(self.model_transform), v[2].transform(self.model_transform)])
            .collect()
    }
}

pub enum PrimitiveDispatch {
    Cube(super::cube::Cube),
    Quad(super::quad::Quad),
    Mesh(Mesh),
}

impl From<super::cube::Cube> for PrimitiveDispatch {
    fn from(cube: super::cube::Cube) -> Self {
        PrimitiveDispatch::Cube(cube)
    }
}

impl From<super::quad::Quad> for PrimitiveDispatch {
    fn from(quad: super::quad::Quad) -> Self {
        PrimitiveDispatch::Quad(quad)
    }
}

impl From<Mesh> for PrimitiveDispatch {
    fn from(mesh: Mesh) -> Self {
        PrimitiveDispatch::Mesh(mesh)
    }
}

impl PartPrimitive for PrimitiveDispatch {
    fn get_vertices(&self) -> Vec<Vertex> {
        match self {
            PrimitiveDispatch::Cube(cube) => cube.get_vertices(),
            PrimitiveDispatch::Quad(quad) => quad.get_vertices(),
            PrimitiveDispatch::Mesh(mesh) => mesh.get_vertices(),
        }
    }

    fn get_indices(&self) -> Vec<u16> {
        match self {
            PrimitiveDispatch::Cube(cube) => cube.get_indices(),
            PrimitiveDispatch::Quad(quad) => quad.get_indices(),
            PrimitiveDispatch::Mesh(mesh) => mesh.get_indices(),
        }
    }
    
    fn get_vertices_grouped(&self) -> Vec<[Vertex; 3]> {
        match self {
            PrimitiveDispatch::Cube(cube) => cube.get_vertices_grouped(),
            PrimitiveDispatch::Quad(quad) => quad.get_vertices_grouped(),
            PrimitiveDispatch::Mesh(mesh) => mesh.get_vertices_grouped(),
        }
    }
}
