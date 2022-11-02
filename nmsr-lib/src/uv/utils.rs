use crate::uv::part::UvImagePixel;
use crate::uv::uv_magic::UvImage;
use crate::uv::Rgba16Image;
use std::borrow::BorrowMut;

pub fn apply_uv_map(input: &Rgba16Image, uv: &UvImage) -> Rgba16Image {
    // Generate a new image
    let mut image = image::ImageBuffer::new(uv.size.0, uv.size.1);

    for uv_pixel in &uv.uv_pixels {
        if let UvImagePixel::UvPixel { position, uv, .. } = uv_pixel {
            let u = position.0;
            let v = position.1;

            image
                .borrow_mut()
                .put_pixel(u, v, *input.get_pixel(uv.0, uv.1));
        }
    }

    image
}

pub fn get_uv_max_depth(image: &Rgba16Image) -> u16 {
    let points = image.pixels().map(|&p| p.0[2]).collect::<Vec<_>>();
    *points.iter().max().unwrap_or(&0)
}
