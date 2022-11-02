use crate::config::CacheConfiguration;
use crate::mojang::caching::MojangCacheManager;
use crate::{routes::model::PlayerRenderInput, utils::Result};
use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag};
use actix_web::{get, web, HttpResponse, Responder};
use parking_lot::RwLock;
use xxhash_rust::xxh3::xxh3_64;

#[get("/skin/{player}")]
pub(crate) async fn get_skin(
    path: web::Path<String>,
    cache_config: web::Data<CacheConfiguration>,
    mojang_requests_client: web::Data<reqwest::Client>,
    cache_manager: web::Data<RwLock<MojangCacheManager>>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let (hash, skin_bytes) = player
        .fetch_skin_bytes(cache_manager.as_ref(), mojang_requests_client.as_ref())
        .await?;

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .append_header(CacheControl(vec![CacheDirective::MaxAge(
            cache_config.mojang_profile_request_expiry,
        )]))
        .append_header(ETag(EntityTag::new_strong(format!(
            "{:x}",
            xxh3_64(hash.as_bytes())
        ))))
        .body(skin_bytes))
}
