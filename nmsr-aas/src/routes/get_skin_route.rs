use crate::{routes::model::PlayerRenderInput, utils::Result};
use actix_web::{get, web, HttpResponse, Responder};

#[get("/skin/{player}")]
pub(crate) async fn get_skin(
    path: web::Path<String>,
    mojang_requests_client: web::Data<reqwest::Client>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let skin_bytes = player
        .get_skin_bytes(mojang_requests_client.as_ref())
        .await?;

    Ok(HttpResponse::Ok().content_type("image/png").body(skin_bytes))
}
