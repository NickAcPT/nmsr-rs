use std::borrow::BorrowMut;
use crate::mojang::caching::MojangCacheManager;
use crate::{routes::model::PlayerRenderInput, utils::Result};
use actix_web::{get, web, web::Buf, HttpResponse, Responder};
use image::ImageFormat::Png;
use nmsr_lib::{parts::manager::PartsManager, rendering::entry::RenderingEntry};
use serde::Deserialize;
use std::io::{BufWriter, Cursor};
use std::sync::Mutex;

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
    cache_manager: web::Data<Mutex<MojangCacheManager>>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    let (hash, skin_bytes) = player
        .fetch_skin_bytes(cache_manager.lock()?.borrow_mut(), mojang_requests_client.as_ref())
        .await?;

    let cached_render = cache_manager.lock()?.get_cached_full_body_render(&hash)?;
    if let Some(bytes) = cached_render {
        return Ok(HttpResponse::Ok().content_type("image/png").body(bytes));
    }

    let skin_image = image::load_from_memory(skin_bytes.chunk())?;

    let entry = RenderingEntry::new(skin_image.into_rgba8(), skin_info.alex.is_some());

    let render = entry.render(parts_manager.as_ref())?;

    let mut render_bytes = Vec::new();

    // Write the image to a byte array
    {
        let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
        render.write_to(&mut writer, Png)?;
    }

    cache_manager.lock()?.cache_full_body_render(&hash, render_bytes.as_slice())?;

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .body(render_bytes))
}
