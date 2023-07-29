#![allow(dead_code)]

// Re-export the Vec3 type from the glam crate
pub use glam::Vec2;
pub use glam::Vec3;

pub mod cube;
pub mod mesh;
pub mod primitives;
pub mod quad;
pub mod utils;
pub mod vertex;
