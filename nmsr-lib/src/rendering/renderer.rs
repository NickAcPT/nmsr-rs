use std::ops::Deref;

use image::imageops::crop;
use image::{GenericImage, GenericImageView, ImageBuffer, Pixel, Rgba};
#[cfg(feature = "parallel_iters")]
use rayon::prelude::*;
use tracing::{instrument, trace_span};

use crate::errors::NMSRError;
use crate::errors::Result;
use crate::parts::manager::PartsManager;
use crate::rendering::entry::RenderingEntry;
use crate::utils::{par_iterator_if_enabled};
use crate::uv::part::UvImagePixel;
use crate::uv::utils::u8_to_u16;
use crate::uv::uv_magic::UvImage;
use crate::uv::Rgba16Image;

impl RenderingEntry {
    #[instrument(level = "trace", skip(parts_manager, uv_image, skin, _span), parent = _span, fields(part = uv_image.name.as_str()))]
    fn apply_uv_and_overlay(
        &self,
        parts_manager: &PartsManager,
        uv_image: &UvImage,
        skin: &Rgba16Image,
        _span: &tracing::Span,
    ) -> Result<Rgba16Image> {

        let mut applied_uv = trace_span!("apply_uv", part = uv_image.name.as_str()).in_scope(|| uv_image.apply(skin))?;

        if !self.render_shading {
            return Ok(applied_uv);
        }

        let overlay = parts_manager.get_overlay(uv_image);

        if let Some(overlay) = overlay {
            let _span_guard = trace_span!("apply_overlay").entered();
            for uv_pixel in &overlay.uv_pixels {
                if let UvImagePixel::RawPixel {
                    position,
                    rgba: overlay_channels,
                } = uv_pixel
                {
                    let pixel_channels = applied_uv
                        .get_pixel_mut(position.x as u32, position.y as u32)
                        .channels_mut();

                    for channel_index in 0..4 {
                        let original_percent =
                            (pixel_channels[channel_index] as f32) / u16::MAX as f32;
                        let overlay_percent =
                            (u8_to_u16!(overlay_channels[channel_index]) as f32) / u16::MAX as f32;

                        pixel_channels[channel_index] =
                            ((original_percent * overlay_percent) * (u16::MAX as f32)) as u16;
                    }
                }
            }
        }

        Ok(applied_uv)
    }

    #[instrument(level="trace", skip(parts_manager))]
    pub fn render(&self, parts_manager: &PartsManager) -> Result<Rgba16Image> {
        // Compute all the parts needed to be rendered
        let all_parts = parts_manager.get_parts(self);

        // Apply all the UVs
        let mut applied_uvs: Vec<_> = trace_span!("apply_uvs").in_scope(|| {
            let current = tracing::Span::current();

            par_iterator_if_enabled!(all_parts)
                .map(|p| {
                    (
                        p.deref(),
                        self.apply_uv_and_overlay(parts_manager, p, &self.skin, &current),
                    )
                })
                .collect()
        });

        // Sort by UV name first to make sure it's deterministic
        if cfg!(feature = "parallel_iters") {
            applied_uvs.par_sort_by_key(|(uv, _)| &uv.name);
        } else {
            applied_uvs.sort_by_key(|(uv, _)| &uv.name)
        }

        // Get the image size
        let (_, first_uv) = applied_uvs.first().ok_or(NMSRError::NoPartsFound)?;
        let first_uv = first_uv.as_ref()?;

        let _span = trace_span!("collect_pixels").entered();

        let mut pixels = applied_uvs.iter()
            .flat_map(|(uv, applied)| {
               uv.uv_pixels.iter().flat_map(move |pixel| match pixel {
                    UvImagePixel::RawPixel { .. } => None,
                    UvImagePixel::UvPixel {
                        depth, position, ..
                    } => Some((
                        depth,
                        position.x,
                        position.y,
                        applied
                            .as_ref()
                            .map(|a| a.get_pixel(position.x as u32, position.y as u32)),
                    )),
                })
            })
            .collect::<Vec<_>>();

        drop(_span);

        if cfg!(feature = "parallel_iters") {
            let _guard = trace_span!("parallel_sort_pixels").entered();
            pixels.par_sort_by_key(|(depth, _, _, _)| *depth);
        } else {
            let _guard = trace_span!("sort_pixels").entered();
            pixels.sort_by_key(|(depth, _, _, _)| *depth);
        }

        // Merge final image
        let (width, height) = (first_uv.width(), first_uv.height());
        let mut final_image: Rgba16Image = ImageBuffer::new(width, height);


        if let Some(environment) = &parts_manager.environment_background {
            let _span = trace_span!("set_environment_background").entered();

            for uv_pixel in &environment.uv_pixels {
                if let UvImagePixel::RawPixel { position, rgba } = uv_pixel {
                    unsafe {
                        let rgba = [
                            u8_to_u16!(rgba[0]),
                            u8_to_u16!(rgba[1]),
                            u8_to_u16!(rgba[2]),
                            u8_to_u16!(rgba[3]),
                        ];

                        final_image.unsafe_put_pixel(
                            position.x as u32,
                            position.y as u32,
                            Rgba(rgba),
                        )
                    }
                }
            }
        }

        {
            let _span = trace_span!("blend_pixels").entered();

            for (_, x, y, pixel) in pixels {
                let pixel = pixel?;
                let alpha = pixel.0[3];
                if alpha > 0 {
                    final_image.get_pixel_mut(x as u32, y as u32).blend(pixel);
                }
            }
        }

        final_image = crop_image(final_image);

        // Return it
        Ok(final_image)
    }
}

const CROP_MARGIN: u32 = 15;

#[instrument(level="trace", skip(image))]
fn crop_image(mut image: Rgba16Image) -> Rgba16Image {
    let mut min_x: u32 = image.width();
    let mut min_y: u32 = image.height();
    let mut max_x: u32 = 0;
    let mut max_y: u32 = 0;

    for (x, y, pixel) in image.enumerate_pixels() {
        let pixel_alpha = pixel.0[3];
        if pixel_alpha > 0 {
            // We found a pixel with alpha, so we need to crop
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }

    let min_x = min_x.saturating_sub(CROP_MARGIN);
    let max_x = max_x.saturating_add(CROP_MARGIN);
    crop(&mut image, min_x, min_y, max_x - min_x, max_y - min_y).to_image()
}
