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
use crate::{
    error::Result,
    model::{
        armor::VanillaMinecraftArmorMaterialData,
        request::{RenderRequest, RenderRequestFeatures},
        resolver::{ResolvedRenderEntryTextureType, ResolvedRenderRequest},
    },
    utils::png::create_png_from_bytes,
};

pub(crate) async fn internal_render_model<'a>(
    request: &RenderRequest,
    state: &NMSRState<'a>,
    resolved: &ResolvedRenderRequest,
) -> Result<Vec<u8>> {
    let scene_context = state.create_scene_context().await?;

    let mode = request.mode;
    #[allow(unused_mut)] // We use mut when we have ears feature enabled
    let mut camera = request.get_camera();

    let size = request.get_size();
    let lighting = request.get_lighting();

    let parts = mode.get_body_parts();

    let mut part_context = create_part_context(request, resolved);

    #[cfg(feature = "ears")]
    if request.features.contains(RenderRequestFeatures::Ears) {
        if let Some(features) = part_context.ears_features.as_ref() {
            NMSRState::apply_ears_camera_settings(features, mode, &mut camera);
        }
    }

    let mut scene = Scene::new(
        &state.graphics_context,
        scene_context,
        camera,
        lighting,
        size,
        &part_context,
        &parts,
    );

    load_textures(resolved, state, request, &mut part_context, &mut scene).await?;

    scene.render(&state.graphics_context)?;

    let render = scene
        .copy_output_texture(&state.graphics_context, true)
        .await?;
    let render_bytes = create_png_from_bytes((size.width, size.height), &render)?;

    Ok(render_bytes)
}

#[cfg(feature = "ears")]
fn load_ears_features(
    part_context: &mut PlayerPartProviderContext<VanillaMinecraftArmorMaterialData>,
    resolved: &ResolvedRenderRequest,
) {
    if let Some(skin_bytes) = resolved.textures.get(&ResolvedRenderEntryTextureType::Skin) {
        if let Ok(skin_image) = load_image(skin_bytes) {
            if let Ok(features) = ears_rs::parser::EarsParser::parse(&skin_image) {
                part_context.ears_features = features;
            }
        }
    }
}

#[instrument(skip_all)]
async fn load_textures<'a>(
    resolved: &ResolvedRenderRequest,
    state: &NMSRState<'a>,
    request: &RenderRequest,
    part_provider: &mut PlayerPartProviderContext<VanillaMinecraftArmorMaterialData>,
    scene: &mut Scene<Object<SceneContextPoolManager<'a>>>,
) -> Result<()> {
    for (&texture_type, texture_bytes) in &resolved.textures {
        let mut image_buffer = load_image(texture_bytes)?;

        if texture_type == ResolvedRenderEntryTextureType::Skin {
            image_buffer = NMSRState::process_skin(image_buffer, request.features)?;
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

pub(crate) fn load_image(texture: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory_with_format(texture, ImageFormat::Png)
        .map_err(NMSRRenderingError::ImageFromRawError)?;
    Ok(img.into_rgba8())
}

pub(crate) fn create_part_context(
    request: &RenderRequest,
    resolved: &ResolvedRenderRequest,
) -> PlayerPartProviderContext<VanillaMinecraftArmorMaterialData> {
    let arm_rotation = request.get_arm_rotation();

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
                crate::model::resolver::ResolvedRenderEntryEarsTextureType::Cape,
            ));

        has_cape_feature && (has_cape || (has_ears_feature && has_ears_cape))
    };

    let shadow_y_pos = request.get_shadow_y_pos();

    let player_armor_slots = PlayerArmorSlots::<VanillaMinecraftArmorMaterialData> {
        helmet: request
            .extra_settings
            .as_ref()
            .and_then(|x| x.helmet.clone()),
        chestplate: request
            .extra_settings
            .as_ref()
            .and_then(|x| x.chestplate.clone()),
        leggings: request
            .extra_settings
            .as_ref()
            .and_then(|x| x.leggings.clone()),
        boots: request
            .extra_settings
            .as_ref()
            .and_then(|x| x.boots.clone()),
    };

    #[cfg_attr(not(feature = "ears"), allow(unused_mut))]
    let mut context = PlayerPartProviderContext::<VanillaMinecraftArmorMaterialData> {
        model: PlayerModel::from(final_model),
        has_layers,
        has_hat_layer,
        has_cape,
        arm_rotation,
        shadow_y_pos,
        shadow_is_square: request.mode.is_head() || request.mode.is_head_iso(),
        armor_slots: Some(player_armor_slots),
        #[cfg(feature = "ears")]
        ears_features: None,
    };
    
    
    #[cfg(feature = "ears")]
    if request.features.contains(RenderRequestFeatures::Ears) {
        load_ears_features(&mut context, resolved);
    }
    
    context
}
