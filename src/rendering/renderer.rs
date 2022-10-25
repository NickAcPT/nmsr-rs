use std::ops::Deref;
use image::{ImageBuffer, RgbaImage};
use crate::PartsManager;
use crate::rendering::entry::RenderingEntry;
use rayon::prelude::*;

impl RenderingEntry {
    pub(crate) fn render(&self, parts_manager: &PartsManager) -> RgbaImage {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let mut applied_uvs: Vec<_> = all_parts.par_iter()
            .map(|p| (p.deref(), p.apply(&self.skin)))
            .collect();

        // Get the image size
        let (_, first_uv) = applied_uvs.first().expect("There needs to be at least 1 image");
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