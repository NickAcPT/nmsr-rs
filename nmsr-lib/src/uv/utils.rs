use crate::errors::{NMSRError, Result};
use crate::uv::part::UvImagePixel;
use crate::uv::uv_magic::UvImage;
use crate::uv::Rgba16Image;
use std::borrow::BorrowMut;

pub fn apply_uv_map(input: &Rgba16Image, uv: &UvImage) -> Result<Rgba16Image> {
    // Generate a new image
    let mut image = image::ImageBuffer::new(uv.size.0, uv.size.1);

    for uv_pixel in &uv.uv_pixels {
        if let UvImagePixel::UvPixel { position, uv, .. } = uv_pixel {
            let u = position.0;
            let v = position.1;

            let pixel = input.get_pixel_checked(uv.0 as u32, uv.1 as u32).ok_or_else(|| NMSRError::InvalidUvPoint(uv.clone()))?;
            image.borrow_mut().put_pixel(u as u32, v as u32, *pixel);
        }
    }

    Ok(image)
}

pub fn get_uv_max_depth(image: &Rgba16Image) -> u16 {
    let points = image.pixels().map(|&p| p.0[2]).collect::<Vec<_>>();
    *points.iter().max().unwrap_or(&0)
}

pub(crate) const U16_TO_U8_PIXEL_RATIO: f32 = u8::MAX as f32 / u16::MAX as f32;
pub(crate) const U8_TO_U16_PIXEL_RATIO: f32 = u16::MAX as f32 / u8::MAX as f32;

macro_rules! u16_to_u8 {
    ($x:expr) => {
        ($x as f32 * crate::uv::utils::U16_TO_U8_PIXEL_RATIO) as u8
    };
}

macro_rules! u8_to_u16 {
    ($x:expr) => {
        ($x as f32 * crate::uv::utils::U8_TO_U16_PIXEL_RATIO) as u16
    };
}

pub(crate) use u16_to_u8;
pub(crate) use u8_to_u16;
