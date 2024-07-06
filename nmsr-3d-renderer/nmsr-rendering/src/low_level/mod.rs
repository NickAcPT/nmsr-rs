#![allow(dead_code)]

// Re-export some types from the glam crate
pub use glam::{EulerRot, Mat4, Quat, Vec2, Vec3};

pub mod primitives;

pub(crate) mod utils;
