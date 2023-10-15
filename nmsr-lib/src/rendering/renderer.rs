use image::{GenericImage, ImageBuffer, Pixel, Rgba, RgbaImage};
#[cfg(feature = "parallel_iters")]
use rayon::prelude::*;
use tracing::{instrument, trace_span};

use crate::errors::NMSRError;
use crate::errors::Result;
use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::utils::par_iterator_if_enabled;
use crate::uv::part::UvImagePixel;
use crate::uv::uv_magic::UvImage;

impl RenderingEntry {
    #[instrument(level = "trace", skip(self, uv_image, skin, _span), parent = _span, fields(part = uv_image.name.as_str()))]
    fn apply_uv_and_overlay(
        &self,
        uv_image: &UvImage,
        skin: &RgbaImage,
        _span: &tracing::Span,
    ) -> Result<RgbaImage> {
        let applied_uv =
            trace_span!("apply_uv").in_scope(|| uv_image.apply(skin, self.render_shading))?;

        applied_uv.save(format!("{}-applied.png", uv_image.name)).unwrap();
            
        Ok(applied_uv)
    }

    #[instrument(level = "trace", skip(parts_manager))]
    pub fn render(&self, parts_manager: &PartsManager) -> Result<RgbaImage> {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let applied_uvs: Vec<_> = trace_span!("apply_uvs").in_scope(|| {
            let current = tracing::Span::current();

            par_iterator_if_enabled!(all_parts)
                .map(|&p| (p, self.apply_uv_and_overlay(p, &self.skin, &current)))
                .collect()
        });

        // Get the image size
        let (_, first_uv) = applied_uvs.first().ok_or(NMSRError::NoPartsFound)?;
        let first_uv = first_uv.as_ref()?;

        let _span = trace_span!("collect_pixels").entered();

        let pixels = applied_uvs
            .iter()
            .flat_map(|(uv, applied)| {
                uv.uv_pixels
                    .iter()
                    .filter(|p| matches!(p, UvImagePixel::UvPixel { .. }))
                    .filter_map(move |pixel| match pixel {
                        UvImagePixel::UvPixel {
                            depth,
                            position,
                            ..
                        } => {
                            applied
                                .as_ref()
                                .map(|a| {
                                    (
                                        *depth,
                                        position.x,
                                        position.y,
                                        a.get_pixel(position.x as u32, position.y as u32),
                                    )
                                })
                                .ok()
                                .filter(|(_, _, _, pixel)| /* alpha > 0 */ pixel.0[3] > 0)
                        }
                        // SAFETY: This is never hit since it's being guarded by the filter call before the filter_map
                        UvImagePixel::RawPixel { .. } => unsafe {
                            std::hint::unreachable_unchecked();
                        },
                    })
            })
            .collect::<Vec<_>>();

        drop(_span);

        /* #[cfg(feature = "parallel_iters")]
        {
            let _guard = trace_span!("parallel_sort_pixels").entered();
            pixels.par_sort_by_key(|(depth, _, _, _)| *depth);
        }
        #[cfg(not(feature = "parallel_iters"))]
        {
            let _guard = trace_span!("sort_pixels").entered();
            pixels.sort_by_key(|(depth, _, _, _)| *depth);
        } */

        // Merge final image
        let (width, height) = (first_uv.width(), first_uv.height());
        let mut final_image: RgbaImage = ImageBuffer::new(width, height);

        if let Some(environment) = &parts_manager.environment_background {
            let _span = trace_span!("set_environment_background").entered();

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
            let _span = trace_span!("blend_pixels").entered();

            for (_, x, y, pixel) in pixels {
                final_image.get_pixel_mut(x as u32, y as u32).blend(pixel);
            }
        }

        // Return it
        Ok(final_image)
    }
}
