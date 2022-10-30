use actix_web::{get, HttpResponse, Responder, web};
use crate::routes::model::PlayerRenderInput;
use crate::utils::Result;

#[get("/skin/{player}")]
pub(crate) async fn get_skin(path: web::Path<String>) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    Ok(HttpResponse::Ok().body(
        format!("Hello {:?}", player)
    ))
}