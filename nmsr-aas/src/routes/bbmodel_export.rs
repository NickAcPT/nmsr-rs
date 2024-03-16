use std::collections::HashMap;

use axum::{
    extract::State,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use hyper::{
    header::{CONTENT_DISPOSITION, CONTENT_TYPE},
    Method,
};
use nmsr_rendering::high_level::{pipeline::{scene::Scene, SceneContextWrapper}, types::PlayerPartTextureType};
use nmsr_rendering_blockbench_model_generator_experiment::{
    blockbench::generate_project,
    error::BlockbenchGeneratorError,
    generator::{ModelGenerationProject, ModelProjectImageIO},
};
use tracing::instrument;

use crate::{
    error::Result,
    model::{
        armor::VanillaMinecraftArmorMaterialData,
        request::{RenderRequest, RenderRequestFeatures},
    },
    routes::render_model::create_part_context,
    utils::png::create_png_from_bytes,
};

use super::{render_model::load_image, NMSRState};

const APPLICATION_JSON_MIME: &str = "application/json";

struct NMSRaaSImageIO;

impl ModelProjectImageIO for NMSRaaSImageIO {
    fn read_png(
        &self,
        image: &[u8],
    ) -> std::result::Result<image::RgbaImage, BlockbenchGeneratorError> {
        load_image(image).map_err(|e| {
            BlockbenchGeneratorError::ExplainedError(format!("Failed to load png: {}", e))
        })
    }

    fn write_png(
        &self,
        image: &image::RgbaImage,
    ) -> std::result::Result<Vec<u8>, BlockbenchGeneratorError> {
        create_png_from_bytes((image.width(), image.height()), &image).map_err(|e| {
            BlockbenchGeneratorError::ExplainedError(format!("Failed to create png: {}", e))
        })
    }
}

#[axum::debug_handler]
#[instrument(skip(state, method))]
pub(crate) async fn internal_bbmodel_export(
    state: State<NMSRState<'static>>,
    method: Method,
    request: RenderRequest,
) -> Result<Response> {
    let resolved = state.resolver.resolve(&request).await?;

    if method == Method::HEAD {
        return Ok(([(
            CONTENT_TYPE,
            HeaderValue::from_static(APPLICATION_JSON_MIME),
        )])
        .into_response());
    }

    let mut part_context = create_part_context(&request, &resolved);
    
    if let Some(pos) = part_context.shadow_y_pos {
        part_context.shadow_y_pos = Some(pos - 0.01);
    }
    
    let mut textures = HashMap::new();

    for (texture_type, texture_bytes) in resolved.textures {
        textures.insert(texture_type.into(), load_image(&texture_bytes)?);
    }

    if request.features.contains(RenderRequestFeatures::Shadow) {
        textures.insert(
            PlayerPartTextureType::Shadow,
            load_image(Scene::<SceneContextWrapper>::get_shadow_bytes(part_context.shadow_is_square))?,
        );
    }

    if let Some(slots) = &part_context.armor_slots {
        let (armor_1, armor_2) = state.armor_manager.create_armor_texture(slots).await?;

        textures.insert(
            VanillaMinecraftArmorMaterialData::ARMOR_TEXTURE_ONE,
            armor_1,
        );

        if let Some(armor_2) = armor_2 {
            textures.insert(
                VanillaMinecraftArmorMaterialData::ARMOR_TEXTURE_TWO,
                armor_2,
            );
        }
    }

    let mut blockbench_project =
        ModelGenerationProject::new_with_part_context(NMSRaaSImageIO, part_context);

    for (texture_type, mut texture) in textures {
        if texture_type == PlayerPartTextureType::Skin {
            texture = NMSRState::process_skin(texture, request.features)?;
        }
        
        blockbench_project.add_texture(texture_type, texture, false)?;
    }

    let result = generate_project(blockbench_project)?;

    let mut res = result.into_response();

    res.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static(APPLICATION_JSON_MIME),
    );

    let entry_str = String::try_from(request.entry).unwrap_or("model".to_string());

    if let Ok(value) = HeaderValue::from_str(&format!(
        "attachment; filename={name}.bbmodel",
        name = entry_str
    )) {
        res.headers_mut().insert(CONTENT_DISPOSITION, value);
    }

    Ok(res)
}
