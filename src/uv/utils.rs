use crate::uv::Rgba16Image;
use image::{Pixel, Rgba, RgbaImage};
use rayon::prelude::*;

const COORDINATE_RESOLVE_SMOOTHING_SCALE: u32 = 32;
const TRANSPARENCY_CUTOFF: u16 = 250;

pub fn apply_uv_map(input: &RgbaImage, uv_map: &Rgba16Image) -> RgbaImage {
    // Generate a new image
    image::ImageBuffer::from_fn(uv_map.width(), uv_map.height(), |x, y| {
        // First we have to read the pixel
        let original_pixel = uv_map.get_pixel(x, y);
        let channels = original_pixel.channels();
        // The coordinates are stored in the following format
        // - R - U coordinate (Horizontal, X)
        // - G - 100% - V coordinate (Vertical, Y)
        // - B - Depth - unused here
        // - A - Normal alpha
        let (u_coord, v_coord, alpha) = (channels[0], channels[1], channels[3]);

        if alpha > TRANSPARENCY_CUTOFF {
            let (u, v) = (
                resolve_coordinate(
                    u_coord,
                    true,
                    input.width() * COORDINATE_RESOLVE_SMOOTHING_SCALE,
                ) / COORDINATE_RESOLVE_SMOOTHING_SCALE,
                resolve_coordinate(
                    v_coord,
                    false,
                    input.height() * COORDINATE_RESOLVE_SMOOTHING_SCALE,
                ) / COORDINATE_RESOLVE_SMOOTHING_SCALE,
            );
            *input.get_pixel(u, v)
        } else {
            Rgba([0u8, 0u8, 0u8, 0u8])
        }
    })
}

fn resolve_coordinate(value: u16, is_u: bool, max_size: u32) -> u32 {
    let value_normalized = value as f32 / u16::MAX as f32;
    let new_coord = (value_normalized) * (max_size as f32 - 1.0);
    let new_coord = new_coord.round();

    if is_u {
        new_coord as u32
    } else {
        (max_size - 1) - (new_coord as u32)
    }
}

pub fn get_uv_max_depth(image: &Rgba16Image) -> u16 {
    let points = image
        .pixels()
        .par_bridge()
        .map(|&p| p.0[2])
        .collect::<Vec<_>>();
    *points.iter().max().unwrap_or(&0)
}
