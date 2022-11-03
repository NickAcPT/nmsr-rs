use crate::errors::Result;
use crate::uv::part::UvImagePixel;
use crate::{uv::utils::apply_uv_map, uv::utils::get_uv_max_depth, uv::Rgba16Image};

#[derive(Debug, Clone)]
pub struct UvImage {
    pub name: String,
    pub size: (u32, u32),
    pub(crate) uv_pixels: Vec<UvImagePixel>,
    pub max_depth: u16,
}

impl UvImage {
    pub fn new(name: String, uv_image: Rgba16Image, store_raw_pixels: bool) -> UvImage {
        let max_depth = get_uv_max_depth(&uv_image);
        let uv_pixels = uv_image
            .enumerate_pixels()
            .flat_map(|(x, y, p)| UvImagePixel::new(x, y, p, store_raw_pixels))
            .collect();

        UvImage {
            name,
            size: (uv_image.width(), uv_image.height()),
            uv_pixels,
            max_depth,
        }
    }

    pub fn apply(&self, original_image: &Rgba16Image) -> Result<Rgba16Image> {
        apply_uv_map(original_image, self)
    }
}
