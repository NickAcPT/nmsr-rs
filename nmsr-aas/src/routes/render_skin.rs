use nmsr_rendering::errors::NMSRRenderingError;

use crate::{
    error::{RenderRequestError, Result},
    model::{
        request::{RenderRequest, RenderRequestFeatures},
        resolver::{ResolvedRenderEntryTextureType, ResolvedRenderRequest},
    },
};

use super::{render::create_png_from_bytes, NMSRState};

pub(crate) async fn internal_render_skin(
    request: RenderRequest,
    state: &NMSRState,
    mut resolved: ResolvedRenderRequest,
) -> Result<Vec<u8>> {
    let skin = resolved
        .textures
        .remove(&ResolvedRenderEntryTextureType::Skin)
        .ok_or(RenderRequestError::InvalidPlayerRequest(
            "Missing skin texture".to_string(),
        ))?;

    if request
        .features
        .contains(RenderRequestFeatures::UnProcessedSkin)
    {
        return Ok(skin);
    }

    let skin_image = image::load_from_memory(&skin)
        .map_err(|_| NMSRRenderingError::ImageFromRawError)?
        .into_rgba8();

    let processed = state.process_skin(skin_image, request.features)?;

    let processed_png_bytes =
        create_png_from_bytes((processed.width(), processed.height()), &processed)?;

    Ok(processed_png_bytes)
}
