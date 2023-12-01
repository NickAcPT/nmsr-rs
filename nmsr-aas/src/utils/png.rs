use image::{RgbaImage, codecs::png::PngEncoder};
use mtpng::{
    encoder::{Encoder, Options},
    ColorType, Header,
};
use tracing::trace_span;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::console;

use crate::error::{ExplainableExt, Result};

pub(crate) fn create_png_from_bytes(size: (u32, u32), bytes: &[u8]) -> Result<Vec<u8>> {
    crate::log("create_png_from_bytes");
    
    let result = {
        
        let image = RgbaImage::from_raw(size.0, size.1, bytes.to_vec())
        .expect_throw("Failed to create image from raw bytes");
        
        crate::log("create_png_from_bytes 2");
    
        let output = Vec::new();
        
        let mut buf_cursor = std::io::Cursor::new(output);
        let encoder = PngEncoder::new_with_quality(&mut buf_cursor, image::codecs::png::CompressionType::Fast, image::codecs::png::FilterType::NoFilter);
        
        image.write_with_encoder(encoder)
            .expect_throw("Failed to write image to buffer");
        
        buf_cursor.into_inner()
    };
    
    crate::log("create_png_from_bytes 3");
    
    Ok(result)
}
