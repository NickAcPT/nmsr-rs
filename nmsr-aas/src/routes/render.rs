use super::{NMSRState, bbmodel_export::internal_bbmodel_export};
use crate::{
    error::{Result, RenderRequestError},
    model::request::{RenderRequest, RenderRequestMode},
    routes::render_model::internal_render_model,
    routes::render_skin::internal_render_skin_or_cape,
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
pub async fn render_post_warning() -> Result<Response> {
    return Err(RenderRequestError::WrongHttpMethodError("POST", "GET").into())
}

#[axum::debug_handler]
pub async fn render_get_warning() -> Result<Response> {
    return Err(RenderRequestError::WrongHttpMethodError("GET", "POST").into())
}

#[axum::debug_handler]
#[instrument(skip(state, method))]
pub async fn render(
    state: State<NMSRState<'static>>,
    method: Method,
    request: RenderRequest,
) -> Result<Response> {
    let resolved = state.resolver.resolve(&request).await?;
    
    if request.mode.is_blockbench_export() {
        // Blockbench export handles HEAD requests for itself, hence why it's before the HEAD method check
        return internal_bbmodel_export(state, method, request).await;
    }
    
    if method == Method::HEAD {
        return Ok(([(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME))]).into_response());
    }

    let result = match request.mode {
        RenderRequestMode::Skin | RenderRequestMode::Cape => internal_render_skin_or_cape(&request, resolved).await,
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
