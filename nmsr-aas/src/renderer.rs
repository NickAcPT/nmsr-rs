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
    use nmsr_rendering::high_level::pipeline::scene::{Scene, Size};

    let scene_context = parts_manager.get_scence_context();
    let camera = mode.get_camera();
    
    let scene = Scene::new(scene_context, camera, Size {width: 832, height: 512});

    Ok(vec![])
    // unimplemented!("wgpu rendering is not yet implemented")
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