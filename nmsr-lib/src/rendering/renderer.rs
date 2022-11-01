use crate::errors::{NMSRError, Result};
use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::uv::uv_magic::UvImage;
use crate::uv::Rgba16Image;
use image::{imageops, ImageBuffer, Pixel};
use std::ops::Deref;

impl RenderingEntry {
    fn apply_uv_and_overlay(
        &self,
        parts_manager: &PartsManager,
        uv_image: &UvImage,
        skin: &Rgba16Image,
    ) -> Rgba16Image {
        let mut applied_uv = uv_image.apply(skin);

        let overlay = parts_manager.get_overlay(uv_image);

        if let Some(overlay) = overlay {
            for uv_pixel in &overlay.uv_pixels {
                if let Some(overlay_channels) = uv_pixel.original_rgba {
                    let pixel_channels = applied_uv
                        .get_pixel_mut(uv_pixel.position.0, uv_pixel.position.1)
                        .channels_mut();

                    for channel_index in 0..4 {
                        let original_percent =
                            (pixel_channels[channel_index] as f32) / u16::MAX as f32;
                        let overlay_percent =
                            (overlay_channels[channel_index] as f32) / u16::MAX as f32;

                        pixel_channels[channel_index] =
                            ((original_percent * overlay_percent) * (u16::MAX as f32)) as u16;
                    }
                }
            }
        }

        applied_uv
    }

    pub fn render(&self, parts_manager: &PartsManager) -> Result<Rgba16Image> {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let mut applied_uvs: Vec<_> = all_parts
            .iter()
            .map(|p| {
                (
                    p.deref(),
                    self.apply_uv_and_overlay(parts_manager, p, &self.skin),
                )
            })
            .collect();

        // Sort by UV name first to make sure it's deterministic
        applied_uvs.sort_by_key(|(uv, _)| &uv.name);

        // Get the image size
        let (_, first_uv) = applied_uvs.first().ok_or(NMSRError::NoPartsFound)?;

        let mut pixels = applied_uvs
            .iter()
            .flat_map(|(uv, applied)| {
                uv.uv_pixels.iter().map(|pixel| {
                    (
                        pixel.depth,
                        pixel.position.0,
                        pixel.position.1,
                        applied.get_pixel(pixel.position.0, pixel.position.1),
                    )
                })
            })
            .collect::<Vec<_>>();

        pixels.sort_by_key(|(depth, _, _, _)| *depth);

        // Merge final image
        let (width, height) = (first_uv.width(), first_uv.height());
        let mut final_image: Rgba16Image = ImageBuffer::new(width, height);

        if let Some(environment) = &parts_manager.environment_background {
            imageops::replace(&mut final_image, environment, 0, 0);
        }

        for (_, x, y, pixel) in pixels {
            let alpha = pixel.0[3];
            if alpha > 0 {
                final_image.get_pixel_mut(x, y).blend(pixel);
            }
        }

        // Return it
        Ok(final_image)
    }
}
