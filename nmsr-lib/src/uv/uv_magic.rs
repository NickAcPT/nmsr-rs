use crate::errors::Result;
use crate::uv::part::UvImagePixel;
use crate::uv::utils::apply_uv_map;
use image::RgbaImage;

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serializable_parts", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serializable_parts_rkyv", derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize))]
pub struct UvImage {
    pub name: String,
    pub size: (u32, u32),
    pub(crate) uv_pixels: Vec<UvImagePixel>,
}

impl UvImage {
    pub fn new(name: String, uv_image: RgbaImage, store_raw_pixels: bool) -> UvImage {
        let uv_pixels: Vec<_> = uv_image
            .enumerate_pixels()
            .filter(|(_, _, p)| p.0[3] != 0)
            .map(|(x, y, p)| UvImagePixel::new(x, y, p, store_raw_pixels))
            .collect();

        UvImage {
            name,
            size: (uv_image.width(), uv_image.height()),
            uv_pixels,
        }
    }

    pub fn apply(&self, original_image: &RgbaImage, render_shading: bool) -> Result<RgbaImage> {
        apply_uv_map(original_image, self, render_shading)
    }
}
