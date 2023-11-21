use crate::low_level::primitives::vertex::Vertex;

pub trait PartPrimitive {
    /// Returns the vertices of the primitive
    fn get_vertices(&self) -> Vec<Vertex>;

    /// Returns the indices of the vertices of the primitive
    /// in the order they should be drawn
    fn get_indices(&self) -> Vec<u16>;

    /// Returns the vertices of the primitive
    fn get_vertices_grouped(&self) -> Vec<[Vertex; 3]>;
}
