use axum::{
    extract::State,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use hyper::{header::CONTENT_TYPE, Method};
use mtpng::{
    encoder::{Encoder, Options},
    ColorType, Header,
};
use tracing::{instrument, trace_span};

use crate::{
    error::{ExplainableExt, Result},
    model::request::{RenderRequest, RenderRequestMode},
    routes::render_skin::internal_render_skin,
};

use super::{render_model::internal_render_model, NMSRState};

const IMAGE_PNG_MIME: &'static str = "image/png";

#[axum::debug_handler]
#[instrument(skip(state, method))]
pub async fn render(
    request: RenderRequest,
    state: State<NMSRState>,
    method: Method,
) -> Result<Response> {
    let resolved = state.resolver.resolve(&request).await?;

    if method == Method::HEAD {
        return Ok(([(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME))]).into_response());
    }

    let result = match request.mode {
        RenderRequestMode::Skin => internal_render_skin(request, state, resolved).await,
        _ => internal_render_model(request, state, resolved).await,
    }?;

    create_image_response(result)
}

fn create_image_response<T>(skin: T) -> Result<Response>
where
    T: IntoResponse,
{
    let mut response = skin.into_response();

    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME));

    Ok(response)
}

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
        .write_image_rows(&bytes)
        .explain_closure(|| "Unable to write image rows for output PNG".to_string())?;

    encoder
        .finish()
        .explain_closure(|| "Unable to finish writing output PNG".to_string())
}
