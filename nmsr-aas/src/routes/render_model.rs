use deadpool::managed::Object;
use image::{ImageFormat, RgbaImage};
use nmsr_rendering::{
    errors::NMSRRenderingError,
    high_level::{
        model::{PlayerArmorSlots, PlayerModel},
        parts::provider::PlayerPartProviderContext,
        pipeline::{pools::SceneContextPoolManager, scene::Scene},
    },
};
use tracing::instrument;

use super::NMSRState;
#[cfg(feature = "ears")]
use crate::model::resolver::ResolvedRenderEntryEarsTextureType;
use crate::{
    error::Result,
    model::{
        armor::VanillaMinecraftArmorMaterialData,
        request::{RenderRequest, RenderRequestFeatures},
        resolver::{ResolvedRenderEntryTextureType, ResolvedRenderRequest},
    },
    utils::png::create_png_from_bytes,
};

pub(crate) async fn internal_render_model(
    request: &RenderRequest,
    state: &NMSRState,
    resolved: &ResolvedRenderRequest,
) -> Result<Vec<u8>> {
    let scene_context = state.create_scene_context().await?;

    let mode = &request.mode;
    let camera = request.get_camera();

    let size = request.get_size();

    let arm_rotation = request.get_arm_rotation();
    let lighting = request.get_lighting();

    let parts = mode.get_body_parts();

    let final_model = request.model.unwrap_or(resolved.model);

    let has_layers = request.features.contains(RenderRequestFeatures::BodyLayers);
    let has_hat_layer = request.features.contains(RenderRequestFeatures::HatLayer);

    #[allow(unused_variables)]
    let has_cape = {
        let has_cape_feature = request.features.contains(RenderRequestFeatures::Cape);
        let has_cape = resolved
            .textures
            .contains_key(&ResolvedRenderEntryTextureType::Cape);

        let has_ears_feature = false;
        let has_ears_cape = false;

        #[cfg(feature = "ears")]
        let has_ears_feature = request.features.contains(RenderRequestFeatures::Ears);

        #[cfg(feature = "ears")]
        let has_ears_cape = resolved
            .textures
            .contains_key(&ResolvedRenderEntryTextureType::Ears(
                ResolvedRenderEntryEarsTextureType::Cape,
            ));

        has_cape_feature && (has_cape || (has_ears_feature && has_ears_cape))
    };

    let shadow_y_pos = request.get_shadow_y_pos();

    let mut player_armor_slots = PlayerArmorSlots::default();

    player_armor_slots.helmet = request
        .extra_settings
        .as_ref()
        .and_then(|x| x.helmet.clone());
    player_armor_slots.chestplate = request
        .extra_settings
        .as_ref()
        .and_then(|x| x.chestplate.clone());
    player_armor_slots.leggings = request
        .extra_settings
        .as_ref()
        .and_then(|x| x.leggings.clone());
    player_armor_slots.boots = request
        .extra_settings
        .as_ref()
        .and_then(|x| x.boots.clone());

    let part_context = PlayerPartProviderContext::<VanillaMinecraftArmorMaterialData> {
        model: PlayerModel::from(final_model),
        has_layers,
        has_hat_layer,
        has_cape,
        arm_rotation,
        shadow_y_pos,
        shadow_is_square: mode.is_head(),
        armor_slots: Some(player_armor_slots),
    };

    let mut scene = Scene::new(
        &state.graphics_context,
        scene_context,
        camera,
        lighting,
        size,
        &part_context,
        &parts,
    );

    load_textures(resolved, &state, &request, &part_context, &mut scene).await?;

    scene.render(&state.graphics_context)?;

    let render = scene
        .copy_output_texture(&state.graphics_context, true)
        .await?;
    let render_bytes = create_png_from_bytes((size.width, size.height), &render)?;

    Ok(render_bytes)
}

#[instrument(skip_all)]
async fn load_textures(
    resolved: &ResolvedRenderRequest,
    state: &NMSRState,
    request: &RenderRequest,
    part_provider: &PlayerPartProviderContext<VanillaMinecraftArmorMaterialData>,
    scene: &mut Scene<Object<SceneContextPoolManager>>,
) -> Result<()> {
    for (&texture_type, texture_bytes) in &resolved.textures {
        let mut image_buffer = load_image(&texture_bytes)?;

        if texture_type == ResolvedRenderEntryTextureType::Skin {
            image_buffer = state.process_skin(image_buffer, request.features)?;
        }

        scene.set_texture(&state.graphics_context, texture_type.into(), &image_buffer);
    }

    if let Some(armor_slots) = part_provider.armor_slots.as_ref() {
        let (main_layer, second_armor_layer) = state
            .armor_manager
            .create_armor_texture(armor_slots)
            .await?;

        scene.set_texture(
            &state.graphics_context,
            VanillaMinecraftArmorMaterialData::ARMOR_TEXTURE_ONE,
            &main_layer,
        );

        if let Some(second_armor_layer) = second_armor_layer {
            scene.set_texture(
                &state.graphics_context,
                VanillaMinecraftArmorMaterialData::ARMOR_TEXTURE_TWO,
                &second_armor_layer,
            );
        }
    }

    Ok(())
}

fn load_image(texture: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory_with_format(&texture, ImageFormat::Png)
        .map_err(NMSRRenderingError::ImageFromRawError)?;
    Ok(img.into_rgba8())
}
