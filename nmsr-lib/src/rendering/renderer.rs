use image::{GenericImage, ImageBuffer, Pixel, Rgba, RgbaImage};
#[cfg(feature = "parallel_iters")]
use rayon::prelude::*;

use crate::{errors::NMSRError, parts::{manager::SpeedyPartsManager, player_model::PlayerModel, speedy_uv::apply_uv_map}};
use crate::errors::Result;
use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::utils::par_iterator_if_enabled;
use crate::uv::part::UvImagePixel;

impl RenderingEntry {
    pub fn render_speedy(&self, parts_manager: &SpeedyPartsManager) -> Result<RgbaImage> {
           let image = match (&self.model, self.render_layers) {
            (&PlayerModel::Steve, true) => &parts_manager.with_layers.steve,
            (&PlayerModel::Steve, false) => &parts_manager.no_layers.steve,
            (&PlayerModel::Alex, true) => &parts_manager.with_layers.alex,
            (&PlayerModel::Alex, false) => &parts_manager.no_layers.alex,
        };
        
        apply_uv_map(&self.skin, image, self.render_shading)
    }
        
    pub fn render(&self, parts_manager: &PartsManager) -> Result<RgbaImage> {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let applied_uvs: Vec<_> = {
            par_iterator_if_enabled!(all_parts)
                .map(|&p| (p, p.apply(&self.skin, self.render_shading)))
                .collect()
        };

        // Get the image size
        let (_, first_uv) = applied_uvs.first().ok_or(NMSRError::NoPartsFound)?;
        let first_uv = first_uv.as_ref()?;

        let pixels = applied_uvs
            .iter()
            .flat_map(|(uv, applied)| {
                uv.uv_pixels
                    .iter()
                    .filter(|p| matches!(p, UvImagePixel::UvPixel { .. }))
                    .filter_map(move |pixel| match pixel {
                        UvImagePixel::UvPixel {
                            /* depth, */ position, ..
                        } => {
                            applied
                                .as_ref()
                                .map(|a| {
                                    (
                                        /* *depth, */
                                        position.x,
                                        position.y,
                                        a.get_pixel(position.x as u32, position.y as u32),
                                    )
                                })
                                .ok()
                                .filter(|(/* _, */ _, _, pixel)| /* alpha > 0 */ pixel.0[3] > 0)
                        }
                        // SAFETY: This is never hit since it's being guarded by the filter call before the filter_map
                        UvImagePixel::RawPixel { .. } => unsafe {
                            std::hint::unreachable_unchecked();
                        },
                    })
            })
            .collect::<Vec<_>>();

        // Merge final image
        let (width, height) = (first_uv.width(), first_uv.height());
        let mut final_image: RgbaImage = ImageBuffer::new(width, height);

        if let Some(environment) = &parts_manager.environment_background {
            for uv_pixel in &environment.uv_pixels {
                if let UvImagePixel::RawPixel { position, rgba } = uv_pixel {
                    unsafe {
                        final_image.unsafe_put_pixel(
                            position.x as u32,
                            position.y as u32,
                            Rgba(*rgba),
                        )
                    }
                }
            }
        }

        {
            for (x, y, pixel) in pixels {
                final_image.get_pixel_mut(x as u32, y as u32).blend(pixel);
            }
        }

        // Return it
        Ok(final_image)
    }
}
