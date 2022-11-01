use crate::mojang::caching::MojangCacheManager;
use crate::{routes::model::PlayerRenderInput, utils::Result};
use actix_web::{get, web, web::Buf, HttpResponse, Responder};
use image::ImageFormat::Png;
use nmsr_lib::{rendering::entry::RenderingEntry};
use parking_lot::RwLock;
use serde::Deserialize;
use std::io::{BufWriter, Cursor};
use crate::manager::{NMSRaaSManager, RenderMode};
use crate::utils::errors::NMSRaaSError;

#[derive(Deserialize, Default)]
pub(crate) struct RenderData {
    alex: Option<String>,
}

#[get("/{type}/{player}")]
pub(crate) async fn render(
    path: web::Path<(String, String)>,
    skin_info: web::Query<RenderData>,
    parts_manager: web::Data<NMSRaaSManager>,
    mojang_requests_client: web::Data<reqwest::Client>,
    cache_manager: web::Data<RwLock<MojangCacheManager>>,
) -> Result<impl Responder> {
    let (mode, player) = path.into_inner();
    let mode: RenderMode = RenderMode::try_from(mode.as_str()).map_err(|_| NMSRaaSError::InvalidRenderMode(mode))?;
    let player: PlayerRenderInput = player.try_into()?;
    let slim_arms = skin_info.alex.is_some();

    let parts_manager = parts_manager.as_ref().get_manager(&mode)?;

    let (hash, skin_bytes) = player
    .fetch_skin_bytes(cache_manager.as_ref(), mojang_requests_client.as_ref())
    .await?;

    let cached_render = cache_manager.read().get_cached_render(&mode, &hash, slim_arms)?;
    if let Some(bytes) = cached_render {
        return Ok(HttpResponse::Ok().content_type("image/png").body(bytes));
    }

    let skin_image = image::load_from_memory(skin_bytes.chunk())?;

    let entry = RenderingEntry::new(skin_image.into_rgba8(), slim_arms);

    let render = entry.render(parts_manager)?;

    let mut render_bytes = Vec::new();

    // Write the image to a byte array
    {
        let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
        render.write_to(&mut writer, Png)?;
    }

    {
        cache_manager
            .write()
            .cache_render(&mode, &hash, slim_arms, render_bytes.as_slice())?;
    }

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(render_bytes))
}
