#![allow(dead_code)]

// Re-export some types from the glam crate
pub use glam::{Vec2, Vec3, Mat4, Quat, EulerRot};

pub mod primitives;

pub(crate) mod utils;
