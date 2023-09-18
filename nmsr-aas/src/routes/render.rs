use super::NMSRState;
use crate::{
    error::Result,
    model::request::{RenderRequest, RenderRequestMode},
    routes::render_model::internal_render_model,
    routes::render_skin::internal_render_skin,
};
use axum::{
    extract::State,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use hyper::{
    header::{CACHE_CONTROL, CONTENT_TYPE},
    Method,
};
use tracing::instrument;
use xxhash_rust::xxh3::xxh3_64;

const IMAGE_PNG_MIME: &str = "image/png";

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
        RenderRequestMode::Skin => internal_render_skin(&request, resolved).await,
        _ => internal_render_model(&request, &state, &resolved).await,
    }?;

    let mut res = create_image_response(result, &state, &request);
    let hash = xxh3_64(format!("{request:?}").as_bytes());

    if let Ok(etag_value) = HeaderValue::from_str(&format!("{hash:x}")) {
        res.headers_mut().insert("Etag", etag_value);
    }

    Ok(res)
}

fn create_image_response<T>(
    skin: T,
    State(state): &State<NMSRState>,
    request: &RenderRequest,
) -> Response
where
    T: IntoResponse,
{
    let mut response = skin.into_response();
    let cache_ctrl = state.get_cache_control_for_request(request);

    if let Ok(cache_ctrl) = HeaderValue::from_str(&cache_ctrl) {
        response.headers_mut().insert(CACHE_CONTROL, cache_ctrl);
    }

    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME));

    response
}
