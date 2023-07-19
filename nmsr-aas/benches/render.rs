#![allow(unused)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

use std::time::Duration;

use nmsr_lib::parts::manager::PartsManager;
use nmsr_lib::rendering::entry::RenderingEntry;
use nmsr_lib::vfs::PhysicalFS;

pub fn criterion_benchmark(c: &mut Criterion) {
    let manager = black_box(PartsManager::new(
        &PhysicalFS::new(".\\run\\parts\\fullbody").into(),
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
        b.iter(|| {
            black_box(
                black_box(&entry)
                    .render(black_box(&manager))
                    .expect("Bruh"),
            )
        })
    });
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(30));
    targets = criterion_benchmark
);
criterion_main!(benches);
