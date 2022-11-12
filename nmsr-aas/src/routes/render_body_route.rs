use std::io::{BufWriter, Cursor};

use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag};
use actix_web::{get, web, web::Buf, HttpResponse, Responder};
use image::ImageFormat::Png;
use parking_lot::RwLock;
use serde::Deserialize;
use xxhash_rust::xxh3::xxh3_64;

use nmsr_lib::rendering::entry::RenderingEntry;

use crate::config::CacheConfiguration;
use crate::manager::{NMSRaaSManager, RenderMode};
use crate::mojang::caching::MojangCacheManager;
use crate::utils::errors::NMSRaaSError;
use crate::{routes::model::PlayerRenderInput, utils::Result};

#[derive(Deserialize, Default)]
pub(crate) struct RenderData {
    alex: Option<String>,
}

#[get("/{type}/{player}")]
pub(crate) async fn render(
    path: web::Path<(String, String)>,
    skin_info: web::Query<RenderData>,
    cache_config: web::Data<CacheConfiguration>,
    parts_manager: web::Data<NMSRaaSManager>,
    mojang_requests_client: web::Data<reqwest::Client>,
    cache_manager: web::Data<RwLock<MojangCacheManager>>,
) -> Result<impl Responder> {
    let (mode, player) = path.into_inner();
    let mode: RenderMode =
        RenderMode::try_from(mode.as_str()).map_err(|_| NMSRaaSError::InvalidRenderMode(mode))?;
    let player: PlayerRenderInput = player.try_into()?;
    let slim_arms = skin_info.alex.is_some();

    let parts_manager = parts_manager.as_ref().get_manager(&mode)?;

    let (hash, skin_bytes) = player
        .fetch_skin_bytes(cache_manager.as_ref(), mojang_requests_client.as_ref())
        .await?;

    let cached_render = cache_manager
        .read()
        .get_cached_render(&mode, &hash, slim_arms)?;

    let render_bytes = if let Some(bytes) = cached_render {
        bytes
    } else {
        let skin_image = image::load_from_memory(skin_bytes.chunk())?;

        let entry = RenderingEntry::new(skin_image.into_rgba8(), slim_arms)?;

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

        render_bytes
    };

    let hash = xxh3_64(render_bytes.as_slice());

    let response = HttpResponse::Ok()
        .content_type("image/png")
        .append_header(CacheControl(vec![
            CacheDirective::Public,
            CacheDirective::MaxAge(cache_config.mojang_profile_request_expiry),
        ]))
        .append_header(ETag(EntityTag::new_strong(format!("{:x}", hash))))
        .body(render_bytes);

    Ok(response)
}
