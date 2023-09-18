use mtpng::{
    encoder::{Encoder, Options},
    ColorType, Header,
};
use tracing::trace_span;

use crate::error::{ExplainableExt, Result};

pub(crate) fn create_png_from_bytes(size: (u32, u32), bytes: &[u8]) -> Result<Vec<u8>> {
    let render_bytes = Vec::new();

    let _guard = trace_span!("write_image_bytes").entered();

    let mut header = Header::new();
    header
        .set_size(size.0, size.1)
        .explain_closure(|| "Unable to set size for output PNG".to_string())?;
    header
        .set_color(ColorType::TruecolorAlpha, 8)
        .explain_closure(|| "Unable to set color type for output PNG".to_string())?;

    let options = Options::new();

    let mut encoder = Encoder::new(render_bytes, &options);

    encoder
        .write_header(&header)
        .explain_closure(|| "Unable to write header for output PNG".to_string())?;
    encoder
        .write_image_rows(bytes)
        .explain_closure(|| "Unable to write image rows for output PNG".to_string())?;

    encoder
        .finish()
        .explain_closure(|| "Unable to finish writing output PNG".to_string())
}
