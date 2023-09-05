use image::{ImageBuffer, Rgba};

pub(crate) mod part;
pub(crate) mod utils;
pub mod uv_magic;

/// Sendable 16-bit Rgb + alpha channel image buffer
pub type Rgba16Image = image::RgbaImage;
