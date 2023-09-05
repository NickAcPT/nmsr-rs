use std::fmt::Debug;

use image::{Pixel, Rgba};
#[cfg(feature = "serializable_parts")]
use serde::{Deserialize, Serialize};

use crate::geometry::Point;
use crate::uv::part::UvImagePixel::{RawPixel, UvPixel};

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serializable_parts", derive(Serialize, Deserialize))]
pub(crate) enum UvImagePixel {
    RawPixel {
        position: Point<u8>,
        rgba: [u8; 4],
    },
    UvPixel {
        position: Point<u8>,
        uv: Point<u8>,
        depth: u8,
    },
}

impl UvImagePixel {
    const COORDINATE_RESOLVE_SMOOTHING_SCALE: u32 = 64;
    const TRANSPARENCY_CUTOFF: u8 = 250;
    const SKIN_SIZE: u32 = 64;

    fn resolve_coordinate(value: u8, is_u: bool, max_size: u32) -> u32 {
        let value_normalized = value as f32 / u8::MAX as f32;
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
        original_pixel: &Rgba<u8>,
        store_raw_pixels: bool,
    ) -> Option<Self> {
        compile_error!("Update this to use the new UV system");
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
                position: Point {
                    x: x as u8,
                    y: y as u8,
                },
                rgba: [
                    channels[0],
                    channels[1],
                    channels[2],
                    channels[3],
                ],
            })
        } else {
            Some(UvPixel {
                position: Point {
                    x: x as u8,
                    y: y as u8,
                },
                uv: Point {
                    x: u as u8,
                    y: v as u8,
                },
                depth,
            })
        }
    }
}
