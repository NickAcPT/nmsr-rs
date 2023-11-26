use image::RgbaImage;
use mtpng::{
    encoder::{Encoder, Options},
    ColorType, Header,
};
use tracing::trace_span;
use wasm_bindgen::UnwrapThrowExt;
use web_sys::console;

use crate::error::{ExplainableExt, Result};

pub(crate) fn create_png_from_bytes(size: (u32, u32), bytes: &[u8]) -> Result<Vec<u8>> {
//    console::log_1(&"create_png_from_bytes".into());
    
    let result = {
        let image = RgbaImage::from_raw(size.0, size.1, bytes.to_vec())
            .expect_throw("Failed to create image from raw bytes");
        
//        console::log_1(&"create_png_from_bytes 2".into());
        
        let output = Vec::new();
        
        let mut buf_cursor = std::io::Cursor::new(output);
        
        image.write_to(&mut buf_cursor, image::ImageOutputFormat::Png)
            .expect_throw("Failed to write image to buffer");
        
        buf_cursor.into_inner()
    };
    
//    console::log_1(&"create_png_from_bytes 3".into());
    
    Ok(result)
}
