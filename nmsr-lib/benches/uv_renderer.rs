use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nmsr_lib::{parts::manager::PartsManager, rendering::entry::RenderingEntry};
use rust_embed::RustEmbed;
use vfs::{EmbeddedFS, VfsPath};

#[derive(RustEmbed, Debug)]
#[folder = "benches/renders-qoi/"]
struct FullBodyParts;

fn bench(c: &mut Criterion) {
    let fs: VfsPath = EmbeddedFS::<FullBodyParts>::new().into();
    let manager = PartsManager::new(&fs).unwrap();
    let skin = image::load_from_memory(include_bytes!("skin.png"))
        .unwrap()
        .into_rgba8();
    
    let request = RenderingEntry::new(skin, true, true, true).unwrap();

    c.bench_function("render_entry", |b| b.iter(|| {
        request.render(black_box(&manager))
    }));
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(100));
    targets = bench
);

criterion_main!(benches);