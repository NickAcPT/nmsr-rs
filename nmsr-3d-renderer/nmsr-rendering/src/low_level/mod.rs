#![allow(dead_code)]

// Re-export the Vec3 type from the glam crate
pub use glam::Vec3;
pub use glam::Vec2;

pub mod primitives;
pub mod vertex;
pub mod quad;
pub mod mesh;
pub mod cube;
pub mod utils;
