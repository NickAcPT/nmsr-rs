use axum::{
    extract::State,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use hyper::{
    header::{CACHE_CONTROL, CONTENT_TYPE},
    Method,
};
use mtpng::{
    encoder::{Encoder, Options},
    ColorType, Header,
};
use tracing::{instrument, trace_span};
use xxhash_rust::xxh3::xxh3_64;

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
    state: State<NMSRState>,
    method: Method,
    request: RenderRequest,
) -> Result<Response> {
    let resolved = state.resolver.resolve(&request).await?;

    if method == Method::HEAD {
        return Ok(([(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME))]).into_response());
    }

    let result = match request.mode {
        RenderRequestMode::Skin => internal_render_skin(&request, &state, resolved).await,
        _ => internal_render_model(&request, &state, &resolved).await,
    }?;

    let mut res = create_image_response(result, &state, &request)?;
    let hash = xxh3_64(format!("{:?}", request).as_bytes());

    if let Ok(etag_value) = HeaderValue::from_str(&format!("{hash:x}")) {
        res.headers_mut().insert(
            "Etag",
            etag_value,
        );
    }

    Ok(res)
}

fn create_image_response<T>(skin: T, State(state): &State<NMSRState>, request: &RenderRequest) -> Result<Response>
where
    T: IntoResponse,
{
    let mut response = skin.into_response();
    let cache_ctrl = state.get_cache_control_for_request(request);
    
    if let Ok(cache_ctrl) = HeaderValue::from_str(&cache_ctrl) {
        response.headers_mut().insert(
            CACHE_CONTROL,
            cache_ctrl,
        );
    }
    
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
