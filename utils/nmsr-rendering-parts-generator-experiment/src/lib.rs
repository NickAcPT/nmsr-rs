use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{anyhow, Ok, Result};
use image::{GenericImage, ImageBuffer, Rgba, RgbaImage};
use itertools::Itertools;
use nmsr_rendering::high_level::{
    camera::{Camera, ProjectionParameters},
    model::PlayerModel,
    parts::provider::PlayerPartProviderContext,
    pipeline::{
        scene::{Scene, Size, SunInformation},
        Backends, BlendState, Features, GraphicsContext, GraphicsContextDescriptor, SceneContext,
        SceneContextWrapper, ShaderSource,
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
};

pub use nmsr_rendering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PartOutputFormat {
    Qoi,
    Png,
}

impl PartOutputFormat {
    pub(crate) fn get_extension(&self) -> &'static str {
        match self {
            PartOutputFormat::Qoi => "qoi",
            PartOutputFormat::Png => "png",
        }
    }
}

pub enum PartsGroupLogic {
    SplitArmsFromBody,
    MergeArmsWithBody,
    MergeEverything,
}

struct PartGroupSpec {
    pub(crate) parts: Vec<PlayerBodyPartType>,
    pub(crate) toggle_slim: bool,
    name: &'static str,
}

impl PartGroupSpec {
    fn new(parts: Vec<PlayerBodyPartType>, toggle_slim: bool, name: &'static str) -> Self {
        Self {
            parts,
            toggle_slim,
            name,
        }
    }
}

impl PartsGroupLogic {
    pub(crate) fn get_groups(&self) -> Vec<PartGroupSpec> {
        match self {
            PartsGroupLogic::SplitArmsFromBody => {
                vec![
                    PartGroupSpec::new(
                        vec![
                            PlayerBodyPartType::Head,
                            PlayerBodyPartType::Body,
                            PlayerBodyPartType::LeftLeg,
                            PlayerBodyPartType::RightLeg,
                        ],
                        /* toggle slim */ false,
                        /* name */ "Body.qoi",
                    ),
                    PartGroupSpec::new(
                        vec![
                            PlayerBodyPartType::HeadLayer,
                            PlayerBodyPartType::BodyLayer,
                            PlayerBodyPartType::LeftLegLayer,
                            PlayerBodyPartType::RightLegLayer,
                        ],
                        /* toggle slim */ false,
                        /* name */ "Body Layer.qoi",
                    ),
                    PartGroupSpec::new(
                        vec![PlayerBodyPartType::LeftArm, PlayerBodyPartType::RightArm],
                        /* toggle slim */ true,
                        /* name */ "{model}/Arms.qoi",
                    ),
                    PartGroupSpec::new(
                        vec![
                            PlayerBodyPartType::LeftArmLayer,
                            PlayerBodyPartType::RightArmLayer,
                        ],
                        /* toggle slim */ true,
                        /* name */ "{model}/Arms Layer.qoi",
                    ),
                ]
            }
            PartsGroupLogic::MergeArmsWithBody => {
                vec![
                    PartGroupSpec::new(
                        vec![
                            PlayerBodyPartType::Head,
                            PlayerBodyPartType::Body,
                            PlayerBodyPartType::LeftLeg,
                            PlayerBodyPartType::RightLeg,
                            PlayerBodyPartType::LeftArm,
                            PlayerBodyPartType::RightArm,
                        ],
                        /* toggle slim */ true,
                        /* name */ "{model}/Body.qoi",
                    ),
                    PartGroupSpec::new(
                        vec![
                            PlayerBodyPartType::HeadLayer,
                            PlayerBodyPartType::BodyLayer,
                            PlayerBodyPartType::LeftLegLayer,
                            PlayerBodyPartType::RightLegLayer,
                            PlayerBodyPartType::LeftArmLayer,
                            PlayerBodyPartType::RightArmLayer,
                        ],
                        /* toggle slim */ true,
                        /* name */ "{model}/Body Layer.qoi",
                    ),
                ]
            }
            PartsGroupLogic::MergeEverything => vec![
                PartGroupSpec::new(
                    vec![
                        PlayerBodyPartType::Head,
                        PlayerBodyPartType::Body,
                        PlayerBodyPartType::LeftLeg,
                        PlayerBodyPartType::RightLeg,
                        PlayerBodyPartType::LeftArm,
                        PlayerBodyPartType::RightArm,
                    ],
                    /* toggle slim */ true,
                    /* name */ "{model}/Body.qoi",
                ),
                PartGroupSpec::new(
                    vec![
                        PlayerBodyPartType::Head,
                        PlayerBodyPartType::Body,
                        PlayerBodyPartType::LeftLeg,
                        PlayerBodyPartType::RightLeg,
                        PlayerBodyPartType::LeftArm,
                        PlayerBodyPartType::RightArm,
                        PlayerBodyPartType::HeadLayer,
                        PlayerBodyPartType::BodyLayer,
                        PlayerBodyPartType::LeftLegLayer,
                        PlayerBodyPartType::RightLegLayer,
                        PlayerBodyPartType::LeftArmLayer,
                        PlayerBodyPartType::RightArmLayer,
                    ],
                    /* toggle slim */ true,
                    /* name */ "{model}/Body Layer.qoi",
                ),
            ],
        }
    }
}

pub async fn generate_parts(
    camera: Camera,
    sun: SunInformation,
    viewport_size: Size,
    actual_parts: Vec<PlayerBodyPartType>,
    parts_group_logic: PartsGroupLogic,
    shadow_y_pos: Option<f32>,
    arm_rotation: f32,
    root: &Path,
    format: PartOutputFormat,
) -> Result<()> {
    fs::create_dir_all(&root)?;

    let groups = parts_group_logic.get_groups();

    for PartGroupSpec {
        parts,
        toggle_slim,
        name,
    } in groups
    {
        process_group(
            parts
                .into_iter()
                .filter(|p| actual_parts.contains(p))
                .collect_vec(),
            toggle_slim,
            camera,
            sun,
            arm_rotation,
            viewport_size,
            name,
            &root,
            format,
        )
        .await?;
    }

    let mut env_shadow = Vec::with_capacity(1);
    process_group_logic(
        vec![PlayerBodyPartType::Head],
        false,
        false,
        &mut env_shadow,
        camera,
        sun,
        viewport_size,
        arm_rotation,
        shadow_y_pos.or(Some(0.0)),
    )
    .await?;

    if let Some(PartRenderOutput { image, .. }) = env_shadow.first() {
        save(image, format, root.join("environment_background.qoi"))?;
    }

    Ok(())
}

async fn save_group(
    to_process: Vec<PartRenderOutput>,
    viewport_size: Size,
    name: String,
    renders_path: &Path,
    format: PartOutputFormat,
) -> Result<()> {
    let processed = process_render_outputs(to_process);

    let layer_count = processed
        .values()
        .max_by_key(|layers| layers.len())
        .map(|layers| layers.len())
        .unwrap_or_default();

    println!("Saving group {} with {} layers", name, layer_count);

    let mut layers: HashMap<usize, _> = HashMap::new();

    for (point, pixels) in processed {
        for (index, pixel) in pixels.iter().enumerate() {
            let img = layers
                .entry(index)
                .or_insert_with(|| RgbaImage::new(viewport_size.width, viewport_size.height));

            unsafe {
                img.unsafe_put_pixel(point.x, point.y, *pixel);
            }
        }
    }

    for (index, img) in &layers {
        let mut file = renders_path.join::<PathBuf>(name.clone().into());
        if layer_count > 1 {
            file = file
                .with_file_name(format!(
                    "{}-{}",
                    file.file_stem().unwrap().to_str().unwrap(),
                    index
                ))
                .with_extension("qoi");
        }

        if let Some(parent) = file.parent() {
            fs::create_dir_all(parent)?;
        }

        save(img, format, file)?;
    }

    Ok(())
}

async fn process_group(
    parts: Vec<PlayerBodyPartType>,
    toggle_slim: bool,
    camera: Camera,
    sun: SunInformation,
    arm_rotation: f32,
    viewport_size: Size,
    name: &'static str,
    renders_path: &Path,
    format: PartOutputFormat,
) -> Result<()> {
    let toggle_backface = parts.iter().any(|p| p.is_hat_layer() || p.is_layer());

    let backface = if toggle_backface {
        vec![false, true]
    } else {
        vec![false]
    };

    let slim = if toggle_slim {
        vec![false, true]
    } else {
        vec![false]
    };

    for slim in slim {
        let mut result = Vec::new();

        for is_back_face in &backface {
            println!(
                "Processing group with parts {:?} (slim: {}, backface: {})",
                &parts, slim, is_back_face
            );

            if toggle_backface {
                let part_split = parts
                    .clone()
                    .into_iter()
                    .sorted_by_key(|part| part.is_layer() || part.is_hat_layer())
                    .into_group_map_by(|part| part.is_layer() || part.is_hat_layer());

                for (is_transparent, parts) in part_split {
                    if !is_transparent && *is_back_face {
                        continue;
                    }

                    if is_transparent {
                        for part in &parts {
                            process_group_logic(
                                vec![*part],
                                slim,
                                *is_back_face,
                                &mut result,
                                camera,
                                sun,
                                viewport_size,
                                arm_rotation,
                                None,
                            )
                            .await?;
                        }
                    } else {
                        process_group_logic(
                            parts,
                            slim,
                            *is_back_face,
                            &mut result,
                            camera,
                            sun,
                            viewport_size,
                            arm_rotation,
                            None,
                        )
                        .await?;
                    }
                }
            } else {
                process_group_logic(
                    parts.clone(),
                    slim,
                    *is_back_face,
                    &mut result,
                    camera,
                    sun,
                    viewport_size,
                    arm_rotation,
                    None,
                )
                .await?;
            }
        }

        let model_name = if slim { "Alex" } else { "Steve" };
        save_group(
            result,
            viewport_size,
            name.replace("{model}", model_name),
            &renders_path,
            format,
        )
        .await?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn process_group_logic(
    parts: Vec<PlayerBodyPartType>,
    slim: bool,
    back_face: bool,
    to_process: &mut Vec<PartRenderOutput>,
    camera: Camera,
    sun: SunInformation,
    viewport_size: Size,
    arm_rotation: f32,
    shadow_y_pos: Option<f32>,
) -> Result<()> {
    let opaque = parts.iter().all(|p| !(p.is_layer() || p.is_hat_layer()));

    println!(
        "  // Processing group logic with parts {:?} (slim: {}, backface: {})",
        &parts, slim, back_face
    );

    let part_provider: PlayerPartProviderContext<()> = PlayerPartProviderContext {
        model: if slim {
            PlayerModel::Alex
        } else {
            PlayerModel::Steve
        },
        has_hat_layer: parts.iter().any(|p| p.is_hat_layer()),
        has_layers: parts.iter().any(|p| p.is_layer()),
        has_cape: false,
        arm_rotation,
        shadow_y_pos,
        shadow_is_square: false,
        armor_slots: None,
        #[cfg(feature = "ears")]
        ears_features: None,
    };

    let mut shader: String = include_str!("nmsr-new-uvmap-shader.wgsl").into();
    if back_face {
        shader = shader.replace("//backingface:", "")
    } else {
        shader = shader.replace("//frontface:", "")
    }
    
    if let ProjectionParameters::Orthographic { .. } = camera.get_projection() {
        shader = shader.replace("//iso:", "")
    }
    
    let descriptor = GraphicsContextDescriptor {
        backends: Some(Backends::all()),
        surface_provider: Box::new(|_| None),
        default_size: (0, 0),
        texture_format: None,
        features: Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES,
        limits: None,
        blend_state: Some(BlendState::REPLACE),
        sample_count: Some(1),
        use_smaa: Some(false),
    };

    let graphics_context = if shadow_y_pos.is_none() {
        GraphicsContext::new_with_shader(descriptor, ShaderSource::Wgsl(shader.into())).await?
    } else {
        GraphicsContext::new(descriptor).await?
    };

    let scene_context = SceneContext::new(&graphics_context);

    let mut scene: Scene<SceneContextWrapper> = Scene::new(
        &graphics_context,
        scene_context.into(),
        camera,
        sun,
        viewport_size,
        &part_provider,
        &[],
    );

    scene.set_texture(
        &graphics_context,
        PlayerPartTextureType::Skin,
        &RgbaImage::new(64, 64),
    );

    scene.rebuild_parts(&part_provider, parts);

    scene.render(&graphics_context)?;

    let render = scene.copy_output_texture(&graphics_context, false).await?;

    let render_image: RgbaImage =
        ImageBuffer::from_raw(viewport_size.width, viewport_size.height, render)
            .ok_or(anyhow!("Unable to convert render to image"))?;

    to_process.push(PartRenderOutput {
        image: render_image,
        is_opaque: opaque,
    });

    Ok(())
}

fn process_render_outputs(to_process: Vec<PartRenderOutput>) -> HashMap<Point, Vec<Rgba<u8>>> {
    let pixels: HashMap<_, Vec<_>> = to_process
        .into_iter()
        .flat_map(|PartRenderOutput { image, is_opaque }| {
            image
                .enumerate_pixels()
                .map(move |(x, y, pixel)| (x, y, *pixel, is_opaque))
                .filter(|(_, _, pixel, _)| pixel[3] != 0)
                .collect::<Vec<_>>()
        })
        .sorted_by_cached_key(|(x, y, _, _)| (*x, *y))
        .group_by(|(x, y, _, _)| (*x, *y))
        .into_iter()
        .flat_map(|(_, group)| {
            let pixels = group
                .map(|(x, y, pixel, is_opaque)| (Point::from((x, y)), pixel, is_opaque))
                .sorted_by_key(|(_, pixel, _)| (get_depth(pixel) as i32))
                .collect::<Vec<_>>();

            let opaque_count = pixels.iter().filter(|(_, _, is_opaque)| *is_opaque).count();
            let has_opaque = opaque_count > 0;

            // Drop all transparent pixels before the first opaque one
            let mut pixels = if has_opaque {
                pixels
                    .into_iter()
                    .skip_while(|(_, _, is_opaque)| !*is_opaque)
                    .collect::<Vec<_>>()
            } else {
                pixels
            };

            if opaque_count > 1 {
                // Find groups of opaque pixels and drop all but the last one
                pixels = pixels
                    .into_iter()
                    .group_by(|(_, _, is_opaque)| *is_opaque)
                    .into_iter()
                    .flat_map(|(is_opaque, group)| {
                        let group_pixels = group.collect::<Vec<_>>();
                        if is_opaque {
                            group_pixels[group_pixels.len() - 1..].to_vec()
                        } else {
                            group_pixels
                        }
                    })
                    .collect_vec();
            }

            pixels.into_iter().map(|(point, pixel, _)| (point, pixel))
        })
        .into_group_map();

    pixels
}

fn get_depth(pixel: &Rgba<u8>) -> u16 {
    let r = pixel[0] as u32;
    let g = pixel[1] as u32;
    let b = pixel[2] as u32;
    let a = pixel[3] as u32;

    let rgba: u32 = r | (g << 8) | (b << 16) | (a << 24);
    // Our Blue channel is composed of the 4 remaining bits of the shading + 4 bits from the depth
    // 1   2   3   4   5   6   7   8
    // [  -- s --  ]   [  -- d --  ]
    // Our Alpha channel is composed of the 8 remaining bits of the depth
    // 1   2   3   4   5   6   7   8
    // [          -- d --          ]
    ((rgba >> 20) & 0x1FFF) as u16
}

fn save<P: AsRef<Path>>(img: &RgbaImage, format: PartOutputFormat, name: P) -> Result<()> {
    let fixed_name = name.as_ref().with_extension(format.get_extension());

    if format == PartOutputFormat::Png {
        img.save(fixed_name)?;
        return Ok(());
    }

    let encoded = qoi::encode_to_vec(&img.as_raw(), img.width(), img.height())?;
    fs::write(fixed_name, encoded)?;

    Ok(())
}

struct PartRenderOutput {
    image: RgbaImage,
    is_opaque: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Point {
    x: u32,
    y: u32,
}

impl From<(u32, u32)> for Point {
    fn from(value: (u32, u32)) -> Self {
        Self {
            x: value.0,
            y: value.1,
        }
    }
}
