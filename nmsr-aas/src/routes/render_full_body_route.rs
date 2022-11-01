use crate::{routes::model::PlayerRenderInput, utils::errors::NMSRaaSError, utils::Result};
use actix_web::{get, web, web::Buf, HttpResponse, Responder};
use image::ImageFormat::Png;
use nmsr_lib::{parts::manager::PartsManager, rendering::entry::RenderingEntry};
use serde::Deserialize;
use std::io::{BufWriter, Cursor};

#[derive(Deserialize, Default)]
pub(crate) struct RenderFullBodyData {
    alex: Option<String>,
}

#[get("/full/{player}")]
pub(crate) async fn render_full_body(
    path: web::Path<String>,
    skin_info: web::Query<RenderFullBodyData>,
    parts_manager: web::Data<PartsManager>,
    mojang_requests_client: web::Data<reqwest::Client>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let skin = player
        .get_skin_bytes(mojang_requests_client.as_ref())
        .await?;
    
    let skin_image =
        image::load_from_memory(skin.chunk()).map_err(NMSRaaSError::InvalidImageError)?;

    let entry = RenderingEntry::new(skin_image.into_rgba8(), skin_info.alex.is_some());

    let render = entry
        .render(parts_manager.as_ref())?;

    let mut render_bytes = Vec::new();

    // Write the image to a byte array
    {
        let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
        render.write_to(&mut writer, Png)?;
    }

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(render_bytes))
}
