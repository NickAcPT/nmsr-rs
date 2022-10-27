use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::uv::uv_magic::UvImage;
use crate::uv::Rgba16Image;
use anyhow::{Context, Result};
use image::{GenericImageView, ImageBuffer, Pixel};
use rayon::prelude::*;
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

        for (x, y, pixel) in applied_uv.enumerate_pixels_mut() {
            let alpha = pixel.0[3];
            if alpha > 0 {
                if let Some(overlay) = overlay {
                    let pixel_channels = pixel.channels_mut();
                    let overlay_channels = overlay.uv_image.get_pixel(x, y).channels();

                    for i in 0..3 {
                        pixel_channels[i] = (((pixel_channels[i] as f32 / u16::MAX as f32)
                            * (overlay_channels[i] as f32 / u16::MAX as f32))
                            * u16::MAX as f32) as u16;
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
            .par_iter()
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
        let (_, first_uv) = applied_uvs
            .first()
            .with_context(|| "There needs to be at least 1 UV image part")?;
        let (width, height) = (first_uv.width(), first_uv.height());

        let mut pixels = applied_uvs
            .iter()
            .flat_map(|(uv, applied)| {
                applied.enumerate_pixels().map(move |(x, y, pixel)| {
                    (
                        unsafe { uv.uv_image.unsafe_get_pixel(x, y) }.0[2], // Depth stored in B channel
                        x,
                        y,
                        pixel,
                    )
                })
            })
            .collect::<Vec<_>>();

        pixels.sort_by_key(|(depth, _, _, _)| *depth);

        // Merge final image
        let mut final_image: Rgba16Image = ImageBuffer::new(width, height);
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
