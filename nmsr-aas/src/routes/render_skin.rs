use nmsr_rendering::errors::NMSRRenderingError;

use super::NMSRState;
use crate::{
    error::{RenderRequestError, Result},
    model::{
        request::{RenderRequest, RenderRequestFeatures},
        resolver::{ResolvedRenderEntryTextureType, ResolvedRenderRequest},
    },
    utils::png::create_png_from_bytes,
};

pub(crate) async fn internal_render_skin_or_cape(
    request: &RenderRequest,
    mut resolved: ResolvedRenderRequest,
) -> Result<Vec<u8>> {
    let texture_bytes = if request.mode.is_skin() {
        resolved
            .textures
            .remove(&ResolvedRenderEntryTextureType::Skin)
    } else {
        #[cfg(feature = "ears")]
        let ears_cape = {
            resolved
                .textures
                .remove(&ResolvedRenderEntryTextureType::Ears(
                    crate::model::resolver::ResolvedRenderEntryEarsTextureType::Cape,
                ))
        };
        #[cfg(not(feature = "ears"))]
        let ears_cape = { None };

        ears_cape.or_else(|| {
            resolved
                .textures
                .remove(&ResolvedRenderEntryTextureType::Cape)
        })
    }
    .ok_or(RenderRequestError::MissingTexture(request.mode.to_string()))?;

    if request
        .features
        .contains(RenderRequestFeatures::UnProcessedSkin)
    {
        return Ok(texture_bytes);
    }

    let processed_png_bytes = if request.mode.is_skin() {
        let skin_image = image::load_from_memory(&texture_bytes)
            .map_err(NMSRRenderingError::ImageFromRawError)?
            .into_rgba8();

        let processed = NMSRState::process_skin(skin_image, request.features)?;

        create_png_from_bytes((processed.width(), processed.height()), &processed)?
    } else {
        texture_bytes
    };

    Ok(processed_png_bytes)
}
