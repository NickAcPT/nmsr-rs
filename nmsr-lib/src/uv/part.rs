use std::fmt::Debug;

use image::{Pixel, Rgba};
#[cfg(feature = "serializable_parts")]
use serde::{Deserialize, Serialize};

use crate::geometry::Point;
use crate::uv::part::UvImagePixel::{RawPixel, UvPixel};

#[derive(Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "serializable_parts", derive(Serialize, Deserialize))]
pub(crate) enum UvImagePixel {
    RawPixel {
        position: Point<u16>,
        rgba: [u8; 4],
    },
    UvPixel {
        position: Point<u16>,
        uv: Point<u8>,
        depth: u16,
        shading: u8,
    },
}

impl UvImagePixel {
    pub(crate) fn new(
        x: u32,
        y: u32,
        original_pixel: &Rgba<u8>,
        store_raw_pixels: bool,
    ) -> Option<Self> {
        let channels = original_pixel.channels();

        let (r, g, b, a) = (
            channels[0] as u32,
            channels[1] as u32,
            channels[2] as u32,
            channels[3] as u32,
        );

        if a == 0 {
            return None;
        }

        let rgba: u32 = (r | (g << 8) | (b << 16) | (a << 24)) as u32;

        // Our Red channel is composed of the 6 bits of the u coordinate + 2 bits from the v coordinate
        // U is used as-is because our coordinates are 0-63
        // 1   2   3   4   5   6   7   8
        // [    ---- u ----    ]   [ v ]
        // Our Green channel is composed of the 4 remaining bits of the v coordinate + 4 bits from the shading
        // V is used as-is because our coordinates are 0-63
        // 1   2   3   4   5   6   7   8
        // [  -- v --  ]   [  -- s --  ]
        // Our Blue channel is composed of the 4 remaining bits of the shading + 4 bits from the depth
        // 1   2   3   4   5   6   7   8
        // [  -- s --  ]   [  -- d --  ]
        // Our Alpha channel is composed of the 8 remaining bits of the depth
        // 1   2   3   4   5   6   7   8
        // [          -- d --          ]
        // let final_number = ((final_depth & 0x1FFF) << 19) | ((shading & 0xFF) << 11) | ((v & 0x3F) << 5) | (u & 0x3F);

        println!("{:#034b}", rgba);
        println!("{:#034b}", ((rgba >> 20) & 0x1FFF));
        
        let u = (rgba & 0x3F) as u8;
        let v = ((rgba >> 6) & 0x3F) as u8;
        let shading = ((rgba >> 12) & 0xFF) as u8;
        let depth = ((rgba >> 20) & 0x1FFF) as u16;

        if store_raw_pixels {
            Some(RawPixel {
                position: Point {
                    x: x as u16,
                    y: y as u16,
                },
                rgba: [channels[0], channels[1], channels[2], channels[3]],
            })
        } else {
            Some(UvPixel {
                position: Point {
                    x: x as u16,
                    y: y as u16,
                },
                uv: Point {
                    x: u as u8,
                    y: v as u8,
                },
                depth,
                shading,
            })
        }
    }
}

#[test]
fn test_uv_pixel() {
    let u = 32u32;
    let v = 23u32;
    let shading = 255u32;

    let final_depth = 3621u32;

    // Our Red channel is composed of the 6 bits of the u coordinate + 2 bits from the v coordinate
    // U is used as-is because our coordinates are 0-63
    // 1   2   3   4   5   6   7   8
    // [    ---- u ----    ]   [ v ]
    // Our Green channel is composed of the 4 remaining bits of the v coordinate + 4 bits from the shading
    // V is used as-is because our coordinates are 0-63
    // 1   2   3   4   5   6   7   8
    // [  -- v --  ]   [  -- s --  ]
    // Our Blue channel is composed of the 4 remaining bits of the shading + 4 bits from the depth
    // 1   2   3   4   5   6   7   8
    // [  -- s --  ]   [  -- d --  ]
    // Our Alpha channel is composed of the 8 remaining bits of the depth
    // 1   2   3   4   5   6   7   8
    // [          -- d --          ]

    let final_number: u32 = ((final_depth & 0x1FFF) << 20) | ((shading & 0xFF) << 12) | ((v & 0x3F) << 6) | (u & 0x3F);

    union U {
        rgba: u32,
        channels: [u8; 4],
    }
    
    let pixels = U { rgba: 0xe0f55fe2 };
    let mut channels = unsafe { pixels.channels };
    channels.reverse();
    
    println!("{:#10x}", final_number);

    let pixel = UvImagePixel::new(1, 1, &Rgba([36, 173, 63, 145]), false);

    assert_eq!(
        pixel,
        Some(UvPixel {
            position: Point { x: 1, y: 1 },
            uv: Point { x: u as u8, y: v as u8 },
            depth: final_depth as u16,
            shading: shading as u8,
        })
    );
}
