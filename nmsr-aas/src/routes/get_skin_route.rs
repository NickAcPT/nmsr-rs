use actix_web::{get, HttpResponse, Responder, web};
use actix_web::web::Buf;
use nmsr_lib::parts::manager::PartsManager;
use crate::routes::model::PlayerRenderInput;
use crate::utils::Result;
use nmsr_lib::rendering::entry::RenderingEntry;
use crate::utils::errors::NMSRaaSError;
use std::io::{BufWriter, Cursor};
use image::ImageFormat::Png;
use serde::Deserialize;

#[derive(Deserialize, Default)]
pub(crate) struct GetSkinInfo {
    alex: bool,
}

#[get("/skin/{player}")]
pub(crate) async fn get_skin(path: web::Path<String>, skin_info: web::Query<GetSkinInfo>, parts_manager: web::Data<PartsManager>) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let skin = player.get_skin_bytes().await?;
    let skin_image = image::load_from_memory(skin.chunk()).map_err(NMSRaaSError::InvalidImageError)?;

    let entry = RenderingEntry::new(skin_image.into_rgba8(), skin_info.alex);

    let render = entry.render(parts_manager.as_ref()).map_err(NMSRaaSError::NMSRError)?;

    let mut render_bytes = Vec::new();

    // Write the image to a byte array
    {
        let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
        render.write_to(&mut writer, Png)?;
    }

    Ok(HttpResponse::Ok().body(
        render_bytes
    ))
}