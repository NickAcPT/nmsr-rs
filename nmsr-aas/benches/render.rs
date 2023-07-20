#![allow(unused)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use std::time::Duration;
use rust_embed::RustEmbed;
use vfs::EmbeddedFS;

use nmsr_lib::parts::manager::PartsManager;
use nmsr_lib::rendering::entry::RenderingEntry;
use nmsr_lib::vfs::PhysicalFS;


#[derive(RustEmbed, Debug)]
#[folder = "../parts/"]
struct FullBodyBiggerParts;

#[derive(RustEmbed, Debug)]
#[folder = "../run/parts/fullbody_old_2023/"]
struct FullBodyParts;

pub fn criterion_benchmark(c: &mut Criterion) {
    let bigger_manager = black_box(PartsManager::new(
        &EmbeddedFS::<FullBodyBiggerParts>::new().into(),
    ))
    .expect("Failed to load parts");
    let normal_manager = black_box(PartsManager::new(
        &EmbeddedFS::<FullBodyParts>::new().into(),
    ))
    .expect("Failed to load parts");
    let entry = black_box(RenderingEntry::new(
        black_box(
            image::load_from_memory(include_bytes!("../../nickac-skin.png"))
                .expect("Failed to load skin")
                .into_rgba8(),
        ),
        false,
        true,
        true,
    ))
    .expect("Failed to create entry");

    c.bench_function("fullbody", |b| {
        b.iter(|| black_box(black_box(&entry).render(black_box(&normal_manager)).expect("Bruh")))
    });
    c.bench_function("fullbody (bigger)", |b| {
        b.iter(|| black_box(black_box(&entry).render(black_box(&bigger_manager)).expect("Bruh")))
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(35));
    targets = criterion_benchmark
);
criterion_main!(benches);
