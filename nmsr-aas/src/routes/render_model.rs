use deadpool::managed::Object;
use image::{ImageFormat, RgbaImage};
use nmsr_rendering::{
    errors::NMSRRenderingError,
    high_level::{
        model::{PlayerArmorSlots, PlayerModel},
        parts::provider::{PlayerMovementContext, PlayerPartProviderContext},
        pipeline::{pools::SceneContextPoolManager, scene::Scene},
        types::PlayerPartTextureType,
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
    request: &mut RenderRequest,
    state: &NMSRState<'a>,
    resolved: &ResolvedRenderRequest,
) -> Result<Vec<u8>> {
    #[cfg(feature = "renderdoc")]
    let mut rd = if !request
        .features
        .contains(RenderRequestFeatures::SkipRenderDocFrameCapture)
    {
        Some(state.render_doc.lock().await)
    } else {
        None
    };

    #[cfg(feature = "renderdoc")]
    {
        if let Some(rd) = &mut rd {
            rd.start_frame_capture(std::ptr::null(), std::ptr::null());
        }
    }

    let scene_context = state.create_scene_context().await?;

    let mode = request.mode;
    #[allow(unused_mut)] // We use mut when we have ears feature enabled
    let mut camera = request.get_camera();

    let size = request.get_size();
    let lighting = request.get_lighting();

    let parts = mode.get_body_parts();

    let mut part_context = create_part_context(request, resolved);

    if request
        .features
        .contains(RenderRequestFeatures::FlipUpsideDown)
    {
        NMSRState::apply_upside_down_camera_settings(mode, &mut camera);
    }

    #[cfg(feature = "ears")]
    if request.features.contains(RenderRequestFeatures::Ears) {
        if let Some(features) = part_context.ears_features.as_ref() {
            NMSRState::apply_ears_camera_settings(features, mode, &request.features, &mut camera);
        }
    }

    if request
        .features
        .contains(RenderRequestFeatures::Deadmau5Ears)
    {
        NMSRState::apply_deadmau5ears_camera_settings(mode, &request.features, &mut camera);
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

    #[cfg(feature = "renderdoc")]
    {
        if let Some(rd) = &mut rd {
            rd.end_frame_capture(std::ptr::null(), std::ptr::null());
        }
    }

    Ok(render_bytes)
}

#[cfg(feature = "ears")]
fn load_ears_features(
    part_context: &mut PlayerPartProviderContext<VanillaMinecraftArmorMaterialData>,
    resolved: &ResolvedRenderRequest,
) {
    if let Some(skin_bytes) = resolved.textures.get(&ResolvedRenderEntryTextureType::Skin) {
        if let Ok(skin_image) = load_image_raw(skin_bytes) {
            if let Ok(features) = ears_rs::parser::EarsParser::parse(&skin_image) {
                part_context.ears_features = features;
                cleanup_invalid_ears_data(&skin_image, part_context);
            }
        }
    }
}

#[cfg(feature = "ears")]
fn cleanup_invalid_ears_data(
    skin_image: &RgbaImage,
    part_context: &mut PlayerPartProviderContext<VanillaMinecraftArmorMaterialData>,
) -> () {
    use ears_rs::alfalfa::read_alfalfa;

    if let Some(features) = part_context.ears_features.as_mut() {
        let alfalfa = read_alfalfa(&skin_image);

        if let Ok(alfalfa) = alfalfa {
            // If features has wings but the alfalfa data does not contain wings, remove the wings
            if features.wing.is_some()
                && alfalfa
                    .as_ref()
                    .and_then(|a| a.get_data(ears_rs::alfalfa::AlfalfaDataKey::Wings))
                    .is_none()
            {
                features.wing.take();
            }

            // If features has cape but the alfalfa data does not contain cape, remove the cape
            if features.cape_enabled
                && alfalfa
                    .as_ref()
                    .and_then(|a| a.get_data(ears_rs::alfalfa::AlfalfaDataKey::Cape))
                    .is_none()
            {
                features.cape_enabled = false;
                part_context.has_cape = false;
            }
        }

        // If features has emissives, but palette is empty, remove the emissives

        if features.emissive
            && ears_rs::utils::extract_emissive_palette(&skin_image)
                .ok()
                .flatten()
                .is_none()
        {
            features.emissive = false;
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
        let expected_size = PlayerPartTextureType::from(texture_type).get_texture_size();

        let is_skin_texture = texture_type == ResolvedRenderEntryTextureType::Skin;

        let mut image_buffer = load_image(texture_bytes, expected_size, !is_skin_texture)?;

        #[cfg(feature = "ears")]
        let is_skin_texture = is_skin_texture || texture_type == ResolvedRenderEntryTextureType::Ears(crate::model::resolver::ResolvedRenderEntryEarsTextureType::EmissiveProcessedSkin);

        if is_skin_texture {
            image_buffer = NMSRState::process_skin(image_buffer, request.features)?;
        }

        scene.set_texture(&state.graphics_context, texture_type.into(), &image_buffer);
    }

    if let (Some(armor_manager), Some(armor_slots)) =
        (&state.armor_manager, part_provider.armor_slots.as_ref())
    {
        let (main_layer, second_armor_layer) =
            armor_manager.create_armor_texture(armor_slots).await?;

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

pub(crate) fn load_image_raw(texture: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory_with_format(texture, ImageFormat::Png)
        .map_err(NMSRRenderingError::ImageFromRawError)?;
    Ok(img.into_rgba8())
}

pub(crate) fn load_image(
    texture: &[u8],
    expected_size: (u32, u32),
    swallow_errors: bool,
) -> Result<RgbaImage> {
    let result = load_image_raw(texture);

    if let (Err(_), true) = (&result, swallow_errors) {
        return Ok(RgbaImage::new(expected_size.0, expected_size.1));
    }

    return result;
}

pub(crate) fn create_part_context(
    request: &mut RenderRequest,
    resolved: &ResolvedRenderRequest,
) -> PlayerPartProviderContext<VanillaMinecraftArmorMaterialData> {
    let custom_arm_rotation_z = request.get_arm_rotation();

    let final_model = request.model.unwrap_or(resolved.model);

    let has_layers = request.features.contains(RenderRequestFeatures::BodyLayers);
    let has_hat_layer = request.features.contains(RenderRequestFeatures::HatLayer);

    let has_deadmau5_ears = request
        .features
        .contains(RenderRequestFeatures::Deadmau5Ears);

    let is_flipped_upside_down = request
        .features
        .contains(RenderRequestFeatures::FlipUpsideDown);

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

    let movement = PlayerMovementContext {
        time: request
            .extra_settings
            .as_ref()
            .and_then(|x| x.time)
            .unwrap_or(0f32),
        limb_swing: request
            .extra_settings
            .as_ref()
            .and_then(|x| x.limb_swing)
            .unwrap_or(0f32),
        ..Default::default()
    };

    #[cfg_attr(not(feature = "ears"), allow(unused_mut))]
    let mut context = PlayerPartProviderContext::<VanillaMinecraftArmorMaterialData> {
        model: PlayerModel::from(final_model),
        has_layers,
        has_hat_layer,
        has_deadmau5_ears,
        is_flipped_upside_down,
        has_cape,
        custom_arm_rotation_z,
        shadow_y_pos,
        shadow_is_square: request.mode.is_head() || request.mode.is_head_iso(),
        armor_slots: Some(player_armor_slots),
        movement,
        #[cfg(feature = "ears")]
        ears_features: None,
    };

    #[cfg(feature = "ears")]
    if request.features.contains(RenderRequestFeatures::Ears) {
        load_ears_features(&mut context, resolved);

        // Make sure that we don't mix Ears and Deadmau5Ears features
        if context.ears_features.is_some() {
            context.has_deadmau5_ears = false;
            request.features.remove(RenderRequestFeatures::Deadmau5Ears);
        }
    }

    context
}
