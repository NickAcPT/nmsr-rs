use image::RgbaImage;

#[cfg(feature = "uv")]
use nmsr_lib::rendering::entry::RenderingEntry;

use crate::manager::{NMSRaaSManager, RenderMode};
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;

#[cfg(feature = "uv")]
pub(crate) async fn render_skin(
    parts_manager: &NMSRaaSManager,
    mode: &RenderMode,
    skin_image: RgbaImage,
    slim_arms: bool,
    include_shading: bool,
    include_layers: bool,
) -> Result<Vec<u8>> {
    let parts_manager = parts_manager.get_manager(mode)?;

    let mut render_bytes = Vec::new();

    let entry = RenderingEntry::new(skin_image, slim_arms, include_shading, include_layers)?;

    let render = entry.render(&parts_manager)?;

    // Write the image to a byte array
    {
        let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
        render.write_to(&mut writer, Png)?;
    }

    Ok(render_bytes)
}

#[cfg(feature = "wgpu")]
pub(crate) async fn render_skin(
    parts_manager: &NMSRaaSManager,
    mode: &RenderMode,
    skin_image: RgbaImage,
    slim_arms: bool,
    include_shading: bool,
    include_layers: bool,
) -> Result<Vec<u8>> {
    use std::io::{BufWriter, Cursor};

    use image::ImageOutputFormat;
    use nmsr_rendering::high_level::{
        parts::provider::PlayerPartProviderContext,
        pipeline::{
            scene::{Scene, Size},
            SceneContext,
        },
        player_model::PlayerModel, types::PlayerPartTextureType,
    };
    
    let skin_image = process_skin(skin_image)?;

    let graphics_context = &parts_manager.graphics_context;
    let scene_context = SceneContext::new(graphics_context);
    let camera = mode.get_camera();
    let body_parts = mode.get_body_parts();

    let model = if slim_arms {
        PlayerModel::Alex
    } else {
        PlayerModel::Steve
    };

    let ctx = PlayerPartProviderContext { model };

    const WIDTH: u32 = 512;
    const HEIGHT: u32 = 832;

    let mut scene = Scene::new(
        graphics_context,
        scene_context,
        camera,
        Size {
            width: WIDTH,
            height: HEIGHT,
        },
        &ctx,
        body_parts,
    );

    scene.set_texture(
        graphics_context,
        PlayerPartTextureType::Skin,
        &skin_image,
    );

    scene.render(graphics_context)?;

    let render = scene
        .copy_output_texture(graphics_context, WIDTH, HEIGHT)
        .await?;

    let mut render_bytes = Vec::new();
    // Write the image to a byte array
    {
        let mut writer = BufWriter::new(Cursor::new(&mut render_bytes));
        render.write_to(&mut writer, ImageOutputFormat::Png)?;
    }

    Ok(render_bytes)
}

pub(crate) fn process_skin(skin: RgbaImage) -> Result<RgbaImage> {
    // Make sure the skin is 64x64
    let mut skin = ears_rs::utils::legacy_upgrader::upgrade_skin_if_needed(skin)
        .ok_or(NMSRaaSError::LegacySkinUpgradeError)?;

    #[cfg(feature = "ears")]
    {
        // If using Ears, process the erase sections specified in the Alfalfa data
        ears_rs::utils::eraser::process_erase_regions(&mut skin)?;
    }

    // Strip the alpha data from the skin
    ears_rs::utils::alpha::strip_alpha(&mut skin);

    Ok(skin)
}
