use std::{
    fmt::{Debug, Formatter},
    mem::transmute,
    simd::{u32x4, u8x16, u8x4, SimdPartialEq, SimdPartialOrd, SimdUint},
};

use image::{imageops, GenericImage, ImageBuffer, Pixel, Rgba, RgbaImage};
use itertools::Itertools;
use rust_embed::RustEmbed;

use crate::errors::Result;

pub struct SpeedyUvImage {
    pixels: Vec<u32>,
    width: u32,
    height: u32,
    layers: usize,
}

pub fn apply_uv_map(
    input: &RgbaImage,
    uv: &SpeedyUvImage,
    render_shading: bool,
) -> Result<RgbaImage> {
    // Generate a new image
    fn owo(rgba: u32) -> (u8, u8, u8) {
        let u = (rgba & 0x3F) as u8;
        let v = ((rgba >> 6) & 0x3F) as u8;
        let shading = ((rgba >> 12) & 0xFF) as u8;

        (u, v, shading)
    }

    let mut pixels = uv
        .pixels
        .iter()
        .flat_map(|p| {
            if p == &0 {
                return [0, 0, 0, 0];
            }
            
            let (u, v, _) = owo(*p);

            input.get_pixel_checked(u as u32, v as u32).unwrap_or(&Rgba([0, 0, 0, 0])).0
        })
        .collect_vec();

    let pixels = pixels
        .chunks_mut((uv.width * 4 * uv.height) as usize)
        .reduce(|a: &mut [u8], b: &mut [u8]| -> &mut [u8] {
            let mut a_img: ImageBuffer<Rgba<u8>, &mut [u8]> =
                ImageBuffer::<Rgba<_>, _>::from_raw(uv.width, uv.height, a)
                    .expect("Failed to create image a");
                
            let b_img = ImageBuffer::<Rgba<_>, _>::from_raw(uv.width, uv.height, b)
                .expect("Failed to create image b");

            imageops::overlay(&mut a_img, &b_img, 0, 0);

            a_img.into_raw()
        })
        .expect("Failed to reduce");

    let image =
        RgbaImage::from_raw(uv.width, uv.height, pixels.to_vec()).expect("Failed to create image");

    Ok(image)
}

fn blend(dst: &mut u32x4, other: &u32x4) {
    if other.simd_eq(u32x4::splat(0)).all() {
        return;
    }

    // If other's alpha is 255, just copy it over
    if other.simd_gt(u32x4::splat(0x000000FF)).all() {
        *dst = *other;
        return;
    }
}

/*
fn blend(&mut self, other: &Rgba<T>) {
    // http://stackoverflow.com/questions/7438263/alpha-compositing-algorithm-blend-modes#answer-11163848

    if other.0[3].is_zero() {
        return;
    }
    if other.0[3] == T::DEFAULT_MAX_VALUE {
        *self = *other;
        return;
    }

    // First, as we don't know what type our pixel is, we have to convert to floats between 0.0 and 1.0
    let max_t = T::DEFAULT_MAX_VALUE;
    let max_t = max_t.to_f32().unwrap();
    let (bg_r, bg_g, bg_b, bg_a) = (self.0[0], self.0[1], self.0[2], self.0[3]);
    let (fg_r, fg_g, fg_b, fg_a) = (other.0[0], other.0[1], other.0[2], other.0[3]);
    let (bg_r, bg_g, bg_b, bg_a) = (
        bg_r.to_f32().unwrap() / max_t,
        bg_g.to_f32().unwrap() / max_t,
        bg_b.to_f32().unwrap() / max_t,
        bg_a.to_f32().unwrap() / max_t,
    );
    let (fg_r, fg_g, fg_b, fg_a) = (
        fg_r.to_f32().unwrap() / max_t,
        fg_g.to_f32().unwrap() / max_t,
        fg_b.to_f32().unwrap() / max_t,
        fg_a.to_f32().unwrap() / max_t,
    );

    // Work out what the final alpha level will be
    let alpha_final = bg_a + fg_a - bg_a * fg_a;
    if alpha_final == 0.0 {
        return;
    };

    // We premultiply our channels by their alpha, as this makes it easier to calculate
    let (bg_r_a, bg_g_a, bg_b_a) = (bg_r * bg_a, bg_g * bg_a, bg_b * bg_a);
    let (fg_r_a, fg_g_a, fg_b_a) = (fg_r * fg_a, fg_g * fg_a, fg_b * fg_a);

    // Standard formula for src-over alpha compositing
    let (out_r_a, out_g_a, out_b_a) = (
        fg_r_a + bg_r_a * (1.0 - fg_a),
        fg_g_a + bg_g_a * (1.0 - fg_a),
        fg_b_a + bg_b_a * (1.0 - fg_a),
    );

    // Unmultiply the channels by our resultant alpha channel
    let (out_r, out_g, out_b) = (
        out_r_a / alpha_final,
        out_g_a / alpha_final,
        out_b_a / alpha_final,
    );

    // Cast back to our initial type on return
    *self = Rgba([
        NumCast::from(max_t * out_r).unwrap(),
        NumCast::from(max_t * out_g).unwrap(),
        NumCast::from(max_t * out_b).unwrap(),
        NumCast::from(max_t * alpha_final).unwrap(),
    ])
} */

impl Debug for SpeedyUvImage {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpeedyUvImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .field("layers", &self.layers)
            .finish()
    }
}

impl SpeedyUvImage {
    pub fn new(layers: &[RgbaImage]) -> Self {
        let layer_count = layers.len();

        let width = layers[0].width() as usize;
        let height = layers[0].height() as usize;

        let pixels = layers
            .into_iter()
            .flat_map(|layer| {
                let samples = layer.as_flat_samples();

                samples.samples.chunks_exact(4)
            })
            .map(|f: &[u8]| -> [u8; 4] { f.try_into().unwrap() })
            .map(|f| u32::from_le_bytes(f))
            .collect::<Vec<_>>();

        // Make sure we have enough pixels to do 4 pixels at a time
        //pixels.resize((layer_count * (height * (width * 4))).next_multiple_of(4), 0);

        Self {
            pixels,
            width: width as u32,
            height: height as u32,
            layers: layer_count,
        }
    }
}

#[cfg(test)]
mod test {
    use image::RgbaImage;
    use rust_embed::RustEmbed;

    use crate::{
        parts::{manager::PartsManager, speedy_uv::apply_uv_map},
        vfs::EmbeddedFS,
    };

    #[derive(RustEmbed, Debug)]
    #[folder = "benches/renders-simd/"]
    struct FullBodyParts;

    #[test]
    fn owo() {
        let manager = PartsManager::new_speedy(&EmbeddedFS::<FullBodyParts>::new().into()).unwrap();
        println!("{:?}", manager);

        let image = manager.with_layers.alex;

        /* let mut out_img = image::RgbaImage::new(image.width, image.height);
        out_img.fill(0);

        let layer_count = image.layers;

        for layer in 0..image.layers {
            for y in 0..image.height {
                let row_size = image.width * 4;

                let row_start = (y * row_size * layer_count) + (layer * row_size);
                let row_end = row_start + row_size;

                let row = &image.pixels[row_start as usize..row_end as usize];

                out_img.as_flat_samples_mut().samples[y as usize * row_size as usize..(y + 1) as usize * row_size as usize].copy_from_slice(row);
            }

            out_img.save(format!("layer-{}.png", layer)).unwrap();
        } */

        let input = image::load_from_memory(include_bytes!("../../../../aaaa.png"))
            .unwrap()
            .to_rgba8();

        apply_uv_map(&input, &image, false)
            .unwrap()
            .save("owo.png")
            .unwrap();
    }
}
