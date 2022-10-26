use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use image::GenericImageView;
use image::ImageBuffer;
use image::Pixel;

use rayon::prelude::*;
use std::ops::{Deref};
use crate::uv::Rgba16Image;
use crate::uv::uv_magic::UvImage;
use anyhow::{Context, Result};

impl RenderingEntry {
    fn apply_uv_and_overlays(&self, parts_manager: &PartsManager, uv_image: &UvImage, skin: &Rgba16Image) -> Rgba16Image {
        let mut applied_uv = uv_image.apply(skin);

        let overlays = parts_manager.get_overlays(uv_image);

        for (x, y, pixel) in applied_uv.enumerate_pixels_mut() {
            let alpha = pixel.0[3] as f32 / u16::MAX as f32;
            if alpha > 0.5 {
                for overlay in &overlays {
                    let overlay_pixel = overlay.uv_image.get_pixel(x, y);
                    pixel.blend(overlay_pixel);
                }
            }
        }

        applied_uv
    }

    pub fn render(&self, parts_manager: &PartsManager) -> Result<Rgba16Image> {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let applied_uvs: Vec<_> = all_parts
            .par_iter()
            .map(|p| (p.deref(), self.apply_uv_and_overlays(parts_manager, p, &self.skin)))
            .collect();

        // Get the image size
        let (_, first_uv) = applied_uvs
            .first()
            .with_context(|| "There needs to be at least 1 UV image part")?;
        let (width, height) = (first_uv.width(), first_uv.height());

        // Order them by distance to the camera
        let mut pixels = applied_uvs.iter()
            .flat_map(|(uv, applied)| applied.enumerate_pixels().map(move |(x, y, pixel)| (unsafe { uv.uv_image.unsafe_get_pixel(x, y) }.0[2], x, y, pixel)))
            .collect::<Vec<_>>();

        pixels.par_sort_by_key(|(depth, _, _, _)| *depth);

        // Merge final image
        let mut final_image: Rgba16Image = ImageBuffer::new(width, height);
        for (_, x, y, pixel) in pixels {
            final_image.get_pixel_mut(x, y).blend(pixel);
        }

        // Return it
        Ok(final_image)
    }
}
