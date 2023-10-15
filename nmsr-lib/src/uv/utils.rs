use crate::errors::{NMSRError, Result};
use crate::uv::part::UvImagePixel;
use crate::uv::uv_magic::UvImage;
use image::RgbaImage;
use std::borrow::BorrowMut;

#[inline(always)]
pub fn apply_uv_map(input: &RgbaImage, uv: &UvImage, render_shading: bool) -> Result<RgbaImage> {
    // Generate a new image
    let mut image = image::ImageBuffer::new(uv.size.0, uv.size.1);

    for uv_pixel in &uv.uv_pixels {
        if let UvImagePixel::UvPixel {
            position,
            uv,
            shading,
            ..
        } = uv_pixel
        {
            let x = position.x;
            let y = position.y;

            let mut pixel = *input
                .get_pixel_checked(uv.x as u32, uv.y as u32)
                .ok_or_else(|| NMSRError::InvalidUvPoint(*uv))?;
            
            // Skip transparent pixels from the skin
            if pixel.0[3] == 0 {
                continue;
            }

            if render_shading {
                let overlay_percent = (*shading as f32) / u8::MAX as f32;

                for channel_index in 0..3 {
                    let original_percent = (pixel[channel_index] as f32) / u8::MAX as f32;

                    pixel[channel_index] =
                        ((original_percent * overlay_percent) * (u8::MAX as f32)) as u8;
                }
            }

            image.borrow_mut().put_pixel(x as u32, y as u32, pixel);
        }
    }

    Ok(image)
}
