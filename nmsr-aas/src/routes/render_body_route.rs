use std::borrow::Borrow;
use std::io::{BufWriter, Cursor};

use actix_web::http::header::{CacheControl, CacheDirective, ETag, EntityTag, CONTENT_TYPE};
use actix_web::web::Path;
use actix_web::{get, head, web, web::Buf, HttpResponse, Responder};
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
    pub(crate) alex: Option<String>,
    pub(crate) steve: Option<String>,
    pub(crate) noshading: Option<String>,
    pub(crate) nolayers: Option<String>,
}

#[derive(Debug)]
pub(crate) struct RenderDataCacheKey {
    pub(crate) slim_arms: bool,
    pub(crate) include_shading: bool,
    pub(crate) include_layers: bool,
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
    let (mode, player) = get_render_data(path)?;

    let include_shading = skin_info.noshading.is_none();
    let include_layers = skin_info.nolayers.is_none();

    let parts_manager = parts_manager.as_ref().get_manager(&mode)?;

    // Fetch the skin hash, model and skin bytes
    let (hash, skin_bytes) = player
        .fetch_skin_bytes(cache_manager.as_ref(), mojang_requests_client.as_ref())
        .await?;

    // Separate the skin hash from the model
    let skin_hash = hash.get_hash();

    // Whether we need to use the Alex model
    // Logic is as follows:
    // 1. If Mojang says the model is Alex, use Alex (this means the player set their model to Alex)
    // 2. If the user specified an alex model, use that
    // 3. If the user specified a steve model, use that, overriding the 1. and 2. rules
    let slim_arms = hash.is_slim_arms() || skin_info.alex.is_some();
    let slim_arms = slim_arms && skin_info.steve.is_none();

    let cache_key = RenderDataCacheKey {
        slim_arms,
        include_shading,
        include_layers,
    };

    let cached_render = cache_manager
        .read()
        .get_cached_render(&mode, skin_hash, &cache_key)?;

    let render_bytes = if let Some(bytes) = cached_render {
        bytes
    } else {
        let skin_image = image::load_from_memory(skin_bytes.chunk())?;

        let entry = RenderingEntry::new(
            skin_image.into_rgba8(),
            slim_arms,
            include_shading,
            include_layers,
        )?;

        let render = entry.render(parts_manager.borrow())?;

        let mut render_bytes = Vec::new();

        // Write the image to a byte array
        {
            let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
            render.write_to(&mut writer, Png)?;
        }

        {
            cache_manager.write().cache_render(
                &mode,
                skin_hash,
                &cache_key,
                render_bytes.as_slice(),
            )?;
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

#[head("/{type}/{player}")]
pub(crate) async fn render_head(
    path: web::Path<(String, String)>,
    mojang_requests_client: web::Data<reqwest::Client>,
    cache_manager: web::Data<RwLock<MojangCacheManager>>,
) -> Result<impl Responder> {
    let (_, player) = get_render_data(path)?;

    drop(
        player
            .fetch_skin_bytes(cache_manager.as_ref(), mojang_requests_client.as_ref())
            .await?,
    );

    Ok(HttpResponse::Ok()
        .append_header((CONTENT_TYPE, "image/png"))
        .finish())
}

fn get_render_data(path: Path<(String, String)>) -> Result<(RenderMode, PlayerRenderInput)> {
    let (mode, player) = path.into_inner();
    let mode: RenderMode =
        RenderMode::try_from(mode.as_str()).map_err(|_| NMSRaaSError::InvalidRenderMode(mode))?;
    let player: PlayerRenderInput = player.try_into()?;

    Ok((mode, player))
}
