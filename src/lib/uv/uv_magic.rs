use crate::{uv::utils::apply_uv_map, uv::utils::get_uv_max_depth, uv::Rgba16Image};

#[derive(Debug)]
pub struct UvImage {
    pub name: String,
    pub uv_image: Rgba16Image,
    pub max_depth: u16,
}

impl UvImage {
    pub fn new(name: String, uv_image: Rgba16Image) -> UvImage {
        let max_depth = get_uv_max_depth(&uv_image);

        UvImage {
            name,
            uv_image,
            max_depth,
        }
    }

    pub fn apply(&self, original_image: &Rgba16Image) -> Rgba16Image {
        apply_uv_map(original_image, &self.uv_image)
    }
}
