use std::borrow::BorrowMut;
use crate::errors::{NMSRError, Result};
use crate::uv::part::UvImagePixel;
use crate::uv::uv_magic::UvImage;
use image::RgbaImage;

pub fn apply_uv_map(input: &RgbaImage, uv: &UvImage) -> Result<RgbaImage> {
    // Generate a new image
    let mut image = image::ImageBuffer::new(uv.size.0, uv.size.1);

    for uv_pixel in &uv.uv_pixels {
        if let UvImagePixel::UvPixel { position, uv, .. } = uv_pixel {
            let x = position.x;
            let y = position.y;

            let pixel = input
                .get_pixel_checked(uv.x as u32, uv.y as u32)
                .ok_or_else(|| NMSRError::InvalidUvPoint(uv.clone()))?;
            image.borrow_mut().put_pixel(x as u32, y as u32, *pixel);
        }
    }

    Ok(image)
}