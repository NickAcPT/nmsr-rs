use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use image::{ImageBuffer, Pixel, RgbaImage};
use rayon::prelude::*;
use std::ops::{Deref, DerefMut};
use crate::uv::Rgba16Image;
use crate::uv::uv_magic::UvImage;

impl RenderingEntry {
    fn apply_uv_and_overlays(&self, parts_manager: &PartsManager, uv_image: &UvImage, skin: &Rgba16Image) -> Rgba16Image {
        let mut applied_uv = uv_image.apply(skin);

        let overlays = parts_manager.get_overlays(self, uv_image);

        for (x, y, pixel) in applied_uv.enumerate_pixels_mut() {
            let alpha = pixel.0[3] / u16::MAX;
            if alpha > 0 {
                for overlay in &overlays {
                    let overlay_pixel = overlay.uv_image.get_pixel(x, y);
                    pixel.blend(overlay_pixel);
                }
            }
        }

        applied_uv
    }

    pub fn render(&self, parts_manager: &PartsManager) -> Rgba16Image {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let mut applied_uvs: Vec<_> = all_parts
            .par_iter()
            .map(|p| (p.deref(), self.apply_uv_and_overlays(parts_manager, p, &self.skin)))
            .collect();

        // Get the image size
        let (_, first_uv) = applied_uvs
            .first()
            .expect("There needs to be at least 1 image");
        let (width, height) = (first_uv.width(), first_uv.height());

        // Order them by distance to the camera
        applied_uvs.sort_by_key(|(uv, _)| uv.max_depth);

        // Merge final image
        let mut final_image = ImageBuffer::new(width, height);
        for (_, image) in applied_uvs {
            image::imageops::overlay(&mut final_image, &image, 0, 0);
        }

        // Return it
        final_image
    }
}
