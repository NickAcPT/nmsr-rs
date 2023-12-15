use std::sync::Arc;

use criterion::{criterion_group, criterion_main, Criterion, SamplingMode};

use ears_rs::parser::EarsParser;
use glam::Vec3;
pub use nmsr_rasterizer_test::{camera::CameraRotation, model::{RenderEntry, Size}, shader::ShaderState};
use nmsr_rasterizer_test::shader;
use nmsr_rendering::high_level::{parts::provider::PlayerPartProviderContext, types::PlayerBodyPartType, IntoEnumIterator};


fn bench(c: &mut Criterion) {
    let mut camera = nmsr_rasterizer_test::camera::Camera::new_orbital(
        Vec3::new(0.0, 16.5 + 2.5, 0.0),
        5.0,
        CameraRotation {
            yaw: 20f32,
            pitch: 10f32,
            roll: 0f32,
        },
        nmsr_rasterizer_test::camera::ProjectionParameters::Perspective { fov: 45f32 },
        Some(Size {
            width: 512,
            height: 869
        }),
    );


    let mut texture = image::open("NickAc.png").unwrap().into_rgba8();

    let context: PlayerPartProviderContext<()> = PlayerPartProviderContext {
        model: nmsr_rendering::high_level::model::PlayerModel::Alex,
        has_hat_layer: true,
        has_layers: true,
        has_cape: false,
        arm_rotation: 10.0,
        shadow_y_pos: None,
        shadow_is_square: false,
        armor_slots: None,
        #[cfg(feature = "ears")]
        ears_features: EarsParser::parse(&texture).expect("Yes"),
    };

    ears_rs::utils::process_erase_regions(&mut texture).expect("Failed to process erase regions");
    ears_rs::utils::strip_alpha(&mut texture);

    let state = ShaderState::new(camera, Arc::new(texture), shader::SunInformation {
            direction: glam::Vec3A::new(0.0, -1.0, 1.0),
            intensity: 2.0,
            ambient: 0.621,
        }, &context, &PlayerBodyPartType::iter().collect::<Vec<_>>());


    let mut entry = RenderEntry::new(camera.get_size().unwrap());
    let mut group = c.benchmark_group("nmsr-rs");
    group.sampling_mode(SamplingMode::Flat);
    group.bench_function("render_entry", |b| {
        b.iter(|| {
            entry.textures.clear_depth();
            entry.draw(&state);
        })
    });
    group.finish();
    entry.dump();
}

criterion_group!(benches, bench);
criterion_main!(benches);