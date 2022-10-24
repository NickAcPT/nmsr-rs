use crate::uv::utils::apply_uv_map;
use crate::uv::utils::get_uv_max_depth;
use crate::uv::Rgba16Image;
use image::RgbaImage;

pub struct UvImage {
    pub uv_image: Rgba16Image,
    pub max_depth: u16,
}

impl UvImage {
    pub fn new(uv_image: Rgba16Image) -> UvImage {
        let max_depth = get_uv_max_depth(&uv_image);

        UvImage {
            uv_image,
            max_depth,
        }
    }

    pub fn apply(&self, original_image: &RgbaImage) -> RgbaImage {
        apply_uv_map(original_image, &self.uv_image)
    }
}
