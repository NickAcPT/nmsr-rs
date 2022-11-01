use crate::{routes::model::PlayerRenderInput, utils::Result};
use actix_web::{get, web, HttpResponse, Responder};
use crate::mojang::caching::MojangCacheManager;

#[get("/skin/{player}")]
pub(crate) async fn get_skin(
    path: web::Path<String>,
    mojang_requests_client: web::Data<reqwest::Client>,
    cache_manager: web::Data<MojangCacheManager>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let (_, skin_bytes) = player
        .fetch_skin_bytes(cache_manager.as_ref(), mojang_requests_client.as_ref())
        .await?;

    Ok(HttpResponse::Ok().content_type("image/png").body(skin_bytes))
}
