use nmsr_rendering::errors::NMSRRenderingError;
use web_sys::console;

use super::NMSRState;
use crate::{
    error::{RenderRequestError, Result},
    model::{
        request::{RenderRequest, RenderRequestFeatures},
        resolver::{ResolvedRenderEntryTextureType, ResolvedRenderRequest},
    },
    utils::png::create_png_from_bytes,
};

pub(crate) async fn internal_render_skin(
    request: &RenderRequest,
    mut resolved: ResolvedRenderRequest,
) -> Result<Vec<u8>> {

    crate::log("internal_render_skin");
    
    let skin = resolved
        .textures
        .remove(&ResolvedRenderEntryTextureType::Skin)
        .ok_or(RenderRequestError::InvalidPlayerRequest(
            "Missing skin texture".to_string(),
        ))?;
        
    crate::log("internal_render_skin 2");

    if request
        .features
        .contains(RenderRequestFeatures::UnProcessedSkin)
    {
        return Ok(skin);
    }
    
    crate::log("internal_render_skin 3");

    let skin_image = image::load_from_memory(&skin)
        .map_err(NMSRRenderingError::ImageFromRawError)?
        .into_rgba8();
    
    crate::log("internal_render_skin 4");

    let processed = NMSRState::process_skin(skin_image, request.features)?;

    crate::log("internal_render_skin 5");
    
    let processed_png_bytes =
        create_png_from_bytes((processed.width(), processed.height()), &processed)?;

    crate::log("internal_render_skin 6");
        
    Ok(processed_png_bytes)
}
