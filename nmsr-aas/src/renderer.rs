use image::RgbaImage;

#[cfg(feature = "uv")]
use nmsr_lib::rendering::entry::RenderingEntry;

use crate::model::resolver::ResolvedRenderRequest;

use crate::manager::{NMSRaaSManager, RenderMode};
use crate::utils::errors::NMSRaaSError;
use crate::utils::Result;

#[cfg(feature = "tracing")]
use tracing::instrument;

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
#[cfg_attr(feature = "tracing", instrument(level = "trace", skip(parts_manager)))]
pub(crate) async fn render_skin(
    parts_manager: &NMSRaaSManager,
    mode: &RenderMode,
    mut resolved: ResolvedRenderRequest,
    include_shading: bool,
    include_layers: bool,
) -> Result<Vec<u8>> {
    #[cfg(feature = "renderdoc")]
    parts_manager.start_frame_capture();
    
    use std::io::{BufWriter, Cursor};

    use nmsr_rendering::high_level::{
        parts::provider::PlayerPartProviderContext,
        pipeline::{
            scene::{Scene, Size},
            SceneContext,
        },
        types::PlayerPartTextureType,
    };
    use tracing::{trace_span, Span};

    use crate::model::resolver::RenderEntryTextureType;

    let setup_guard = trace_span!(parent: Span::current(), "setup").entered();

    let skin_image = trace_span!("process_skin").in_scope(|| {
        resolved
            .textures
            .remove(&RenderEntryTextureType::Skin)
            .ok_or(NMSRaaSError::GameProfileError(
                "Missing skin texture".to_owned(),
            ))
    })?;
    
    let cape_image = resolved.textures.get(&RenderEntryTextureType::Cape);

    let skin_image = process_skin(skin_image)?;

    let graphics_context = &parts_manager.graphics_context;
    let scene_context =
        trace_span!("build_scene_context").in_scope(|| SceneContext::new(graphics_context));
    let camera = mode.get_camera();
    let sun = mode.get_lighting(!include_shading);
    let arm_rotation = mode.get_arm_rotation();
    let mut body_parts = mode.get_body_parts();
    
    body_parts.retain(|p| include_layers || !p.is_layer());

    let model = resolved.model.into();

    let ctx = PlayerPartProviderContext {
        model,
        has_layers: include_layers,
        arm_rotation,
        has_cape: cape_image.is_some(),
    };

    const WIDTH: u32 = 512/*  * 4 */;
    const HEIGHT: u32 = 832/*  * 4 */;
    
    let mut scene = trace_span!("build_scene").in_scope(|| {
        Scene::new(
            graphics_context,
            scene_context,
            camera,
            sun,
            Size {
                width: WIDTH,
                height: HEIGHT,
            },
            &ctx,
            body_parts,
        )
    });

    trace_span!("set_textures").in_scope(|| {
        scene.set_texture(graphics_context, PlayerPartTextureType::Skin, &skin_image);
        
        if let Some(cape) = cape_image {
            scene.set_texture(graphics_context, PlayerPartTextureType::Cape, cape);
        }
    });

    drop(setup_guard);

    trace_span!("render").in_scope(|| scene.render(graphics_context))?;

    // Raw pixel bytes
    let render = {
        let _guard = trace_span!(parent: Span::current(), "copy_output_texture").entered();

        scene.copy_output_texture(graphics_context).await?
    };
    
    // Write the image to a byte array
    let render_bytes = {
        let render_bytes = Vec::new();
        
        let _guard = trace_span!("write_image_bytes").entered();

        let mut header = mtpng::Header::new();
        header.set_size(WIDTH, HEIGHT).expect("Buddy, I expected this to work 1");
        header.set_color(mtpng::ColorType::TruecolorAlpha, 8).expect("Buddy, I expected this to work 2");

        let mut options = mtpng::encoder::Options::new();

        let mut encoder = mtpng::encoder::Encoder::new(render_bytes, &options);

        encoder.write_header(&header).expect("Buddy, I expected this to work 3");
        encoder.write_image_rows(&render).expect("Buddy, I expected this to work 4");
        encoder.finish().expect("Buddy, I expected this to work 5")
    };

    #[cfg(feature = "renderdoc")]
    parts_manager.end_frame_capture();

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
