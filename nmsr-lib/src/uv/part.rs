use crate::uv::part::UvImagePixel::{RawPixel, UvPixel};
use image::{Pixel, Rgba};

#[derive(Debug, Clone)]
pub(crate) struct Point(pub(crate) u32, pub(crate) u32);

#[derive(Debug, Clone)]
pub(crate) enum UvImagePixel {
    RawPixel {
        position: Point,
        rgba: [u16; 4],
    },
    UvPixel {
        position: Point,
        uv: Point,
        depth: u16,
    },
}

impl UvImagePixel {
    const COORDINATE_RESOLVE_SMOOTHING_SCALE: u32 = 64;
    const TRANSPARENCY_CUTOFF: u16 = 250;
    const SKIN_SIZE: u32 = 64;

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

    pub(crate) fn new(
        x: u32,
        y: u32,
        original_pixel: &Rgba<u16>,
        store_raw_pixels: bool,
    ) -> Option<Self> {
        let channels = original_pixel.channels();
        // The coordinates are stored in the following format
        // - R - U coordinate (Horizontal, X)
        // - G - 100% - V coordinate (Vertical, Y)
        // - B - Depth - unused here
        // - A - Normal alpha
        let (u_coord, v_coord, depth, alpha) = (channels[0], channels[1], channels[2], channels[3]);

        if alpha <= Self::TRANSPARENCY_CUTOFF {
            return None;
        }
        let (u, v) = (
            Self::resolve_coordinate(
                u_coord,
                true,
                Self::SKIN_SIZE * Self::COORDINATE_RESOLVE_SMOOTHING_SCALE,
            ) / Self::COORDINATE_RESOLVE_SMOOTHING_SCALE,
            Self::resolve_coordinate(
                v_coord,
                false,
                Self::SKIN_SIZE * Self::COORDINATE_RESOLVE_SMOOTHING_SCALE,
            ) / Self::COORDINATE_RESOLVE_SMOOTHING_SCALE,
        );

        if store_raw_pixels {
            Some(RawPixel {
                position: Point(x, y),
                rgba: [channels[0], channels[1], channels[2], channels[3]],
            })
        } else {
            Some(UvPixel {
                position: Point(x, y),
                uv: Point(u, v),
                depth,
            })
        }
    }
}
