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

use crate::{
    error::Result,
    model::{
        armor::{VanillaMinecraftArmorMaterial, VanillaMinecraftArmorMaterialData, VanillaMinecraftArmorTrim, VanillaMinecraftArmorTrimMaterial},
        request::{RenderRequest, RenderRequestFeatures},
        resolver::{ResolvedRenderEntryTextureType, ResolvedRenderRequest},
    },
};

use super::{render::create_png_from_bytes, NMSRState};

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
    let has_cape = request.features.contains(RenderRequestFeatures::Cape)
        && resolved
            .textures
            .contains_key(&ResolvedRenderEntryTextureType::Cape);

    let shadow_y_pos = request.get_shadow_y_pos();

    let mut player_armor_slots = PlayerArmorSlots::default();

    player_armor_slots.helmet.replace(
        VanillaMinecraftArmorMaterialData::new(VanillaMinecraftArmorMaterial::Iron).with_trim(
            VanillaMinecraftArmorTrim::Silence,
            VanillaMinecraftArmorTrimMaterial::Redstone,
        ),
    );

    player_armor_slots
        .chestplate
        .replace(VanillaMinecraftArmorMaterialData::new(
            VanillaMinecraftArmorMaterial::Diamond,
        ));

    player_armor_slots
        .leggings
        .replace(VanillaMinecraftArmorMaterialData::new(
            VanillaMinecraftArmorMaterial::Gold,
        ));

    player_armor_slots
        .boots
        .replace(VanillaMinecraftArmorMaterialData::new(
            VanillaMinecraftArmorMaterial::Netherite,
        ));

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
        parts,
    );

    load_textures(resolved, &state, &request, &part_context, &mut scene).await?;

    scene.render(&state.graphics_context)?;

    let render = scene.copy_output_texture(&state.graphics_context).await?;
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
    let armor_slots = part_provider.armor_slots.as_ref().expect("msg");

    for (&texture_type, texture_bytes) in &resolved.textures {
        let mut image_buffer = load_image(&texture_bytes)?;

        if texture_type == ResolvedRenderEntryTextureType::Skin {
            image_buffer = state.process_skin(image_buffer, request.features)?;
        }

        scene.set_texture(&state.graphics_context, texture_type.into(), &image_buffer);
    }

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

    Ok(())
}

fn load_image(texture: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory_with_format(&texture, ImageFormat::Png)
        .map_err(|_| NMSRRenderingError::ImageFromRawError)?;
    Ok(img.into_rgba8())
}
