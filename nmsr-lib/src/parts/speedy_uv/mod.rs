use std::{
    fmt::{Debug, Formatter},
    simd::{u32x4, u8x16, u8x4, SimdUint, SimdPartialEq},
};

use image::{GenericImage, RgbaImage, Pixel};
use rust_embed::RustEmbed;

use crate::errors::Result;

pub struct SpeedyUvImage {
    pixels: Vec<u8x4>,
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
    let mut image: RgbaImage = image::ImageBuffer::new(uv.width, uv.height);

    let data_in_layer_count = (uv.width) as usize;

    for y in (0..uv.pixels.len()).step_by(data_in_layer_count * uv.layers) {
        for layer_count in (0..uv.layers).step_by(2) {
            let row_start = y + (layer_count * data_in_layer_count);
            let pixels = &uv.pixels[row_start..(row_start + data_in_layer_count * 2)];

            //let pixels: &[u32] = unsafe { std::mem::transmute(pixels) };
            //println!("{:?}", pixels.len());
            
            fn owo(pixel: u8x4) -> u8x4 {
                let rgba = u32::from_le_bytes(pixel.to_array());
                
                let u = (rgba & 0x3F) as u8;
                let v = ((rgba >> 6) & 0x3F) as u8;
                let shading = ((rgba >> 12) & 0xFF) as u8;
                
                u8x4::from_array([
                    u,
                    v,
                    shading,
                    0xFF,
                ])
            }
            
            for pixel_pos in (0..pixels.len()).step_by(2) {
                let pixel_layer_1 = pixels[pixel_pos];
                let pixel_layer_2 = pixels[pixel_pos + 1];
                
                if pixel_layer_1.simd_eq(u8x4::splat(0)).all() || pixel_layer_2.simd_eq(u8x4::splat(0)).all() {
                    continue;
                }
                
                let actual_pixel_y = y / (data_in_layer_count * uv.layers);
                let actual_pixel_x = pixel_pos % data_in_layer_count;
                
                //println!("{:?} -> {:?}", owo(pixel_layer_1), owo(pixel_layer_2));
                
                let (pixel_uv, _) = owo(pixel_layer_1).interleave(owo(pixel_layer_2));
                
                let [pixel_u_1, pixel_u_2, pixel_v_1, pixel_v_2] = pixel_uv.to_array();
                
                let p_1 = input.get_pixel(pixel_u_1 as u32, pixel_v_1 as u32);
                let p_2 = input.get_pixel(pixel_u_2 as u32, pixel_v_2 as u32);
                
                image.get_pixel_mut(actual_pixel_x as u32, actual_pixel_y as u32).blend(p_1);
                image.get_pixel_mut(((pixel_pos + 1) % data_in_layer_count) as u32, actual_pixel_y as u32).blend(p_2);
            }

            //let layer_1 = u32x4::from_slice(&pixels[0..data_in_layer_count]);
            //let layer_2 = u32x4::from_slice(&pixels[data_in_layer_count..]);
//
            //if layer_1.simd_eq(u32x4::splat(0)).all() || layer_2.simd_eq(u32x4::splat(0)).all() {
            //    continue;
            //}
            
            //println!("{:?} & {:?}", layer_1, layer_2);
        }
    }

    Ok(image)
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
        let layer_count_aligned = layers.len().next_multiple_of(2);

        let real_layer_count = layers.len();
        let width = layers[0].width() as usize;
        let height = layers[0].height() as usize;

        let mut pixels: Vec<u8x4> = Vec::with_capacity(layer_count_aligned * (height * (width * 4)));
        pixels.resize(layer_count_aligned * (height * (width * 4)), u8x4::splat(0));

        for y in 0..height {
            for l in 0..real_layer_count {
                let data_in_layer_count: usize = (width * 4) as usize;
                let layer = &layers[l];
                let (_, row, _) = layer.as_flat_samples().samples[y * data_in_layer_count..(y + 1) * data_in_layer_count].as_simd::<4>();

                // Pixels should be in the following layout:
                // [Layer 0 - Row 0][Layer 1 - Row 0][Layer 2 - Row 0]...
                // [Layer 0 - Row 1][Layer 1 - Row 1][Layer 2 - Row 1]...

                let data_in_layer_count: usize = (width) as usize;
                
                let dst_row_start =
                    (y * data_in_layer_count * real_layer_count) + (l * data_in_layer_count);
                let dst_row_end = dst_row_start + data_in_layer_count;

                pixels[dst_row_start..dst_row_end].copy_from_slice(row);
            }
        }

        Self {
            pixels,
            width: width as u32,
            height: height as u32,
            layers: layer_count_aligned,
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
        let manager = PartsManager::new(&EmbeddedFS::<FullBodyParts>::new().into()).unwrap();
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
