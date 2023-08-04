use image::RgbaImage;

#[cfg(feature = "uv")]
use nmsr_lib::rendering::entry::RenderingEntry;

use crate::manager::{NMSRaaSManager, RenderMode};
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
    let pipeline = parts_manager.get_pipeline();

    unimplemented!("wgpu rendering is not yet implemented")
}
