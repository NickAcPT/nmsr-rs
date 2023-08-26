use axum::{
    extract::State,
    http::HeaderValue,
    response::{IntoResponse, Response},
};
use hyper::{body::Bytes, header::CONTENT_TYPE, Method};
use image::{ImageFormat, RgbaImage};
use mtpng::{
    encoder::{Encoder, Options},
    ColorType, Header,
};
use nmsr_rendering::{
    errors::NMSRRenderingError,
    high_level::{
        parts::provider::PlayerPartProviderContext,
        pipeline::scene::{Scene, Size},
        player_model::PlayerModel, types::PlayerPartTextureType,
    },
};
use tracing::trace_span;

use crate::{
    error::{ExplainableExt, RenderRequestError, Result},
    model::{
        request::{RenderRequest, RenderRequestFeatures, RenderRequestMode},
        resolver::ResolvedRenderEntryTextureType,
    },
};

use super::NMSRState;

#[axum::debug_handler]
pub async fn render_model(
    request: RenderRequest,
    state: State<NMSRState>,
    method: Method,
) -> Result<Response> {
    match request.mode {
        RenderRequestMode::Skin => internal_render_skin(request, state, method).await,
        _ => internal_render_model(request, state, method).await,
    }
}

async fn internal_render_skin(
    request: RenderRequest,
    State(state): State<NMSRState>,
    method: Method,
) -> Result<Response> {
    let mut resolved = state.resolver.resolve(&request).await?;

    if method == Method::HEAD {
        return Ok(([(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME))]).into_response());
    }

    let skin = resolved
        .textures
        .remove(&ResolvedRenderEntryTextureType::Skin)
        .ok_or(RenderRequestError::InvalidPlayerRequest(
            "Missing skin texture".to_string(),
        ))?;

    if request
        .features
        .contains(RenderRequestFeatures::UnProcessedSkin)
    {
        return create_image_response(skin);
    }

    let skin_image = image::load_from_memory(&skin)
        .map_err(|_| NMSRRenderingError::ImageFromRawError)?
        .into_rgba8();

    let processed = state.process_skin(skin_image, request.features)?;

    let processed_png_bytes =
        create_png_from_bytes((processed.width(), processed.height()), &processed)?;

    create_image_response(processed_png_bytes)
}

fn create_image_response<T>(skin: T) -> Result<Response>
where
    T: IntoResponse,
{
    let mut response = skin.into_response();

    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME));

    Ok(response)
}

const IMAGE_PNG_MIME: &'static str = "image/png";

async fn internal_render_model(
    request: RenderRequest,
    State(state): State<NMSRState>,
    method: Method,
) -> Result<Response> {
    let resolved = state.resolver.resolve(&request).await?;

    if method == Method::HEAD {
        return Ok(([(CONTENT_TYPE, HeaderValue::from_static(IMAGE_PNG_MIME))]).into_response());
    }

    let scene_context = state.create_scene_context().await?;

    let size = Size {
        width: 512,
        height: 896,
    };

    let mode = &request.mode;
    let camera = mode.get_camera();
    let arm_rotation = mode.get_arm_rotation();
    let lighting = mode.get_lighting(!request.features.contains(RenderRequestFeatures::Shading));
    let parts = mode.get_body_parts();

    let final_model = request.model.unwrap_or(resolved.model);

    let has_layers = request.features.contains(RenderRequestFeatures::BodyLayers);
    let has_cape = request.features.contains(RenderRequestFeatures::Cape)
        && resolved
            .textures
            .contains_key(&ResolvedRenderEntryTextureType::Cape);

    let part_context = PlayerPartProviderContext {
        model: PlayerModel::from(final_model),
        has_layers, // TODO - Hat layers
        has_cape,
        arm_rotation,
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

    for (texture_type, texture_bytes) in resolved.textures {
        let mut image_buffer = load_image(&texture_bytes)?;
        
        if texture_type == ResolvedRenderEntryTextureType::Skin {
            image_buffer = state.process_skin(image_buffer, request.features)?;
        }
        
        scene.set_texture(
            &state.graphics_context,
            texture_type.into(),
            &image_buffer,
        );
    }

    scene.render(&state.graphics_context)?;

    let render = scene.copy_output_texture(&state.graphics_context).await?;

    let render_bytes = create_png_from_bytes((size.width, size.height), &render)?;

    create_image_response(render_bytes)
}

fn create_png_from_bytes(size: (u32, u32), bytes: &[u8]) -> Result<Vec<u8>> {
    let render_bytes = Vec::new();

    let _guard = trace_span!("write_image_bytes").entered();

    let mut header = Header::new();
    header
        .set_size(size.0, size.1)
        .explain_closure(|| "Unable to set size for output PNG".to_string())?;
    header
        .set_color(ColorType::TruecolorAlpha, 8)
        .explain_closure(|| "Unable to set color type for output PNG".to_string())?;

    let options = Options::new();

    let mut encoder = Encoder::new(render_bytes, &options);

    encoder
        .write_header(&header)
        .explain_closure(|| "Unable to write header for output PNG".to_string())?;
    encoder
        .write_image_rows(&bytes)
        .explain_closure(|| "Unable to write image rows for output PNG".to_string())?;

    encoder
        .finish()
        .explain_closure(|| "Unable to finish writing output PNG".to_string())
}

fn load_image(texture: &[u8]) -> Result<RgbaImage> {
    let img = image::load_from_memory_with_format(&texture, ImageFormat::Png)
        .map_err(|_| NMSRRenderingError::ImageFromRawError)?;
    Ok(img.into_rgba8())
}
