#![allow(dead_code)]

// Re-export the Vec types from the glam crate
pub use glam::Vec2;
pub use glam::Vec3;

pub mod primitives;

pub(crate) mod utils;
