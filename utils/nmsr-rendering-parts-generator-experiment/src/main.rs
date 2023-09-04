use std::{collections::HashMap, ops::Deref};

use anyhow::{anyhow, Ok, Result};
use image::{GenericImage, ImageBuffer, Rgba, RgbaImage};
use itertools::Itertools;
use nmsr_rendering::high_level::{
    camera::{Camera, CameraRotation, ProjectionParameters},
    model::PlayerModel,
    parts::provider::PlayerPartProviderContext,
    pipeline::{
        scene::{Scene, Size, SunInformation},
        Backends, GraphicsContext, GraphicsContextDescriptor, SceneContext, SceneContextWrapper,
        ShaderSource,
    },
    types::{PlayerBodyPartType, PlayerPartTextureType},
    IntoEnumIterator,
};
use tokio::fs;

#[tokio::main]
async fn main() -> Result<()> {
    let rotation = CameraRotation {
        yaw: 20.0,
        pitch: 10.0,
        roll: 0.0,
    };

    let camera = Camera::new_orbital(
        [0.0, 16.5, 0.0].into(),
        45.0,
        rotation,
        ProjectionParameters::Perspective { fov: 45.0 },
        None,
    );

    let sun = SunInformation::new([0.0, -1.0, 5.0].into(), 1.0, 0.621);

    let viewport_size = Size {
        width: 512,
        height: 832,
    };

    let part_provider: PlayerPartProviderContext<()> = PlayerPartProviderContext {
        model: PlayerModel::Alex,
        has_hat_layer: true,
        has_layers: true,
        has_cape: false,
        arm_rotation: 10.0,
        shadow_y_pos: None,
        shadow_is_square: false,
        armor_slots: None,
    };

    let mut to_process: Vec<PartRenderOutput> = vec![];

    for back_face in vec![false, true] {
        let mut shader: String = include_str!("nmsr-old-uvmap-shader.wgsl").into();
        if back_face {
            shader = shader.replace("//backingface:", "")
        }

        let graphics_context = GraphicsContext::new_with_shader(
            GraphicsContextDescriptor {
                backends: Some(Backends::all()),
                surface_provider: Box::new(|_| None),
                default_size: (0, 0),
                texture_format: None,
            },
            ShaderSource::Wgsl(shader.into()),
        )
        .await?;
    
        println!("Created graphics context {:?}", graphics_context.multisampling_strategy);

        let scene_context = SceneContext::new(&graphics_context);

        let mut scene: Scene<SceneContextWrapper> = Scene::new(
            &graphics_context,
            scene_context.into(),
            camera,
            sun,
            viewport_size,
            &part_provider,
            vec![],
        );

        scene.set_texture(
            &graphics_context,
            PlayerPartTextureType::Skin,
            &RgbaImage::new(64, 64),
        );

        for part in PlayerBodyPartType::iter() {
            if back_face && !part.is_layer() {
                continue;
            }

            scene.rebuild_parts(&part_provider, vec![part]);

            scene.render(&graphics_context)?;

            let render = scene.copy_output_texture(&graphics_context).await?;

            let render_image: RgbaImage =
                ImageBuffer::from_raw(viewport_size.width, viewport_size.height, render)
                    .ok_or(anyhow!("Unable to convert render to image"))?;

            //render_image.save(format!("renders/render-{:?}-{}.png", part, back_face))?;

            to_process.push(PartRenderOutput {
                part,
                image: render_image,
            });
        }
    }

    let processed = process_render_outputs(to_process);

    let layer_count = processed
        .values()
        .max_by_key(|layers| layers.len())
        .map(|layers| layers.len())
        .unwrap_or_default();

    let mut layers: HashMap<usize, RgbaImage> = HashMap::new();

    for (point, pixels) in processed {
        // Go through each element in pixels, and remove elements on it until we find a pixel that is not transparent
        //let bad_pixels = pixels
        //    .iter()
        //    .take_while(|(_, part)| part.is_layer())
        //    .collect::<Vec<_>>();

        for (index, (pixel, _)) in pixels.iter()/* .skip(bad_pixels.len()) */.enumerate() {
            let img = layers
                .entry(index)
                .or_insert_with(|| RgbaImage::new(viewport_size.width, viewport_size.height));

            unsafe {
                img.unsafe_put_pixel(point.x, point.y, *pixel);
            }
        }
    }

    fs::create_dir_all("renders").await?;

    for (index, img) in layers {
        img.save(format!("renders/render-layer-{}.png", index))?;
    }

    println!("Layer count: {:?}", layer_count);

    Ok(())
}

fn process_render_outputs(
    to_process: Vec<PartRenderOutput>,
) -> HashMap<Point, Vec<(Rgba<u8>, PlayerBodyPartType)>> {
    let pixels: HashMap<_, Vec<_>> = to_process
        .into_iter()
        .flat_map(|PartRenderOutput { part, image }| {
            image
                .enumerate_pixels()
                .map(move |(x, y, pixel)| (x, y, *pixel, part))
                .filter(|(_, _, pixel, _)| pixel[3] != 0)
                .collect::<Vec<_>>()
        })
        .sorted_by_cached_key(|(x, y, _, _)| (*x, *y))
        .group_by(|(x, y, _, _)| (*x, *y))
        .into_iter()
        .flat_map(|(_, group)| {
            let pixels = group
                .map(|(x, y, pixel, part)| (Point::from((x, y)), (pixel, part)))
                .sorted_by_key(|(_, (pixel, _))| pixel[2])
                .collect::<Vec<_>>();

            pixels
        })
        .into_group_map();

    pixels
}

struct PartRenderOutput {
    part: PlayerBodyPartType,
    image: RgbaImage,
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
