use std::io::{BufWriter, Cursor};

use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag, CONTENT_TYPE};
use actix_web::web::Buf;
use actix_web::{get, head, web, HttpResponse, Responder};
use image::ImageFormat::Png;
use parking_lot::RwLock;
use reqwest_middleware::ClientWithMiddleware;
use serde::Deserialize;
use xxhash_rust::xxh3::xxh3_64;

use nmsr_lib::rendering::entry::RenderingEntry;

use crate::config::CacheConfiguration;
use crate::mojang::caching::MojangCacheManager;
use crate::{routes::model::PlayerRenderInput, utils::Result};

#[derive(Deserialize, Default)]
pub(crate) struct SkinRequest {
    process: Option<String>,
}

#[get("/skin/{player}")]
pub(crate) async fn get_skin(
    path: web::Path<String>,
    skin_info: web::Query<SkinRequest>,
    cache_config: web::Data<CacheConfiguration>,
    mojang_requests_client: web::Data<ClientWithMiddleware>,
    cache_manager: web::Data<RwLock<MojangCacheManager>>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;
    let should_process = skin_info.process.is_some();

    let (hash, mut skin_bytes) = player
        .fetch_skin_bytes(
            cache_manager.as_ref(),
            mojang_requests_client.as_ref(),
            &tracing::Span::current(),
        )
        .await?;

    if should_process {
        let image = image::load_from_memory(skin_bytes.chunk())?.into_rgba8();
        let image = RenderingEntry::process_skin(image)?;

        let mut render_bytes = Vec::new();

        // Write the image to a byte array
        {
            let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
            image.write_to(&mut writer, Png)?;
        }

        skin_bytes = render_bytes.into();
    }

    Ok(HttpResponse::Ok()
        .content_type("image/png")
        .append_header(CacheControl(vec![CacheDirective::MaxAge(
            cache_config.mojang_profile_request_expiry,
        )]))
        .append_header(ETag(EntityTag::new_strong(format!(
            "{:x}",
            xxh3_64(hash.get_hash().as_bytes())
        ))))
        .body(skin_bytes))
}

#[head("/skin/{player}")]
pub(crate) async fn get_skin_head(
    path: web::Path<String>,
    mojang_requests_client: web::Data<ClientWithMiddleware>,
    cache_manager: web::Data<RwLock<MojangCacheManager>>,
) -> Result<impl Responder> {
    let player: PlayerRenderInput = path.into_inner().try_into()?;

    drop(
        player
            .fetch_skin_bytes(
                cache_manager.as_ref(),
                mojang_requests_client.as_ref(),
                &tracing::Span::current(),
            )
            .await?,
    );

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "image/png"))
        .finish())
}
