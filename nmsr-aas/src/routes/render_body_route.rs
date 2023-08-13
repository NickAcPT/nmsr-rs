use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag, CONTENT_TYPE};
use actix_web::web::Path;
use actix_web::{get, head, web, web::Buf, HttpResponse, Responder};
use enumset::{enum_set, EnumSet};
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use xxhash_rust::xxh3::xxh3_64;

use crate::config::{CacheConfiguration, MojankConfiguration};
use crate::manager::{NMSRaaSManager, RenderMode};
use crate::model::resolver::RenderRequestResolver;
use crate::model::{RenderRequest, RenderRequestEntry};
use crate::mojang::caching::MojangCacheManager;
use crate::renderer::render_skin;
use crate::utils::errors::NMSRaaSError;
use crate::{routes::model::PlayerRenderInput, utils::Result};

#[derive(Deserialize, Default, Debug)]
pub(crate) struct RenderData {
    pub(crate) alex: Option<String>,
    pub(crate) steve: Option<String>,
    pub(crate) noshading: Option<String>,
    pub(crate) nolayers: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RenderDataCacheKey {
    pub(crate) slim_arms: bool,
    pub(crate) include_shading: bool,
    pub(crate) include_layers: bool,
}

#[get("/{type}/{player}")]
#[cfg_attr(
    feature = "tracing",
    tracing::instrument(skip(parts_manager, render_request_resolver))
)]
pub(crate) async fn render(
    path: Path<(String, String)>,
    skin_info: web::Query<RenderData>,
    parts_manager: web::Data<NMSRaaSManager>,
    render_request_resolver: web::Data<RenderRequestResolver>,
) -> Result<impl Responder> {
    let (mode, entry) = get_render_data(path)?;

    let include_shading = skin_info.noshading.is_none();
    let include_layers = skin_info.nolayers.is_none();

    let parts_manager = parts_manager.as_ref();

    let render_request = RenderRequest::new_from_excluded_features(entry, None, EnumSet::EMPTY);
    let resolved = render_request_resolver.resolve(render_request).await?;

    let render_bytes = render_skin(
        parts_manager,
        &mode,
        resolved,
        include_shading,
        include_layers,
    )
    .await?;

    let hash = xxh3_64(render_bytes.as_slice());

    let response = HttpResponse::Ok()
        .content_type("image/png")
        .append_header(CacheControl(vec![
            CacheDirective::Public,
            CacheDirective::MaxAge(render_request_resolver.cache_config().mojang_profile_request_expiry),
        ]))
        .append_header(ETag(EntityTag::new_strong(format!("{hash:x}"))))
        .body(render_bytes);

    Ok(response)
}

#[head("/{type}/{player}")]
pub(crate) async fn render_head(
    path: Path<(String, String)>,
    render_request_resolver: web::Data<RenderRequestResolver>,
) -> Result<impl Responder> {
    let (_, entry) = get_render_data(path)?;
    let render_request = RenderRequest::new_from_excluded_features(entry, None, EnumSet::EMPTY);
    
    drop(render_request_resolver.resolve(render_request).await?);

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "image/png"))
        .finish())
}

fn get_render_data(path: Path<(String, String)>) -> Result<(RenderMode, RenderRequestEntry)> {
    let (mode, player) = path.into_inner();
    let mode: RenderMode =
        RenderMode::try_from(mode.as_str()).map_err(|_| NMSRaaSError::InvalidRenderMode(mode))?;
    let player: RenderRequestEntry = player.try_into()?;

    Ok((mode, player))
}
