use image::{codecs::png::PngEncoder, ImageEncoder};
use tracing::trace_span;

use crate::error::{NMSRaaSError, Result};

pub(crate) fn create_png_from_bytes((width, height): (u32, u32), bytes: &[u8]) -> Result<Vec<u8>> {
    let mut out = Vec::new();

    let _guard = trace_span!("write_image_bytes").entered();
    
    let encoder = PngEncoder::new(&mut out);

    encoder
        .write_image(&bytes, width, height, image::ExtendedColorType::Rgba8)
        .map_err(|e| NMSRaaSError::ClonedError(e.to_string()))?;

    Ok(out)
}
