use std::borrow::BorrowMut;
use std::sync::Mutex;
use crate::mojang::caching::MojangCacheManager;
use crate::{routes::model::PlayerRenderInput, utils::Result};
use actix_web::{get, web, HttpResponse, Responder};

#[get("/skin/{player}")]
pub(crate) async fn get_skin(
    path: web::Path<String>,
    mojang_requests_client: web::Data<reqwest::Client>,
    cache_manager: web::Data<Mutex<MojangCacheManager>>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let (_, skin_bytes) = player
        .fetch_skin_bytes(cache_manager.lock()?.borrow_mut(), mojang_requests_client.as_ref())
        .await?;

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(skin_bytes))
}
