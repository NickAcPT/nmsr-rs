use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nmsr_lib::parts::manager::PartsManager;
use rust_embed::RustEmbed;
use vfs::{EmbeddedFS, VfsPath};

#[derive(RustEmbed, Debug)]
#[folder = "benches/renders-qoi/"]
struct FullBodyParts;

fn bench(c: &mut Criterion) {
    let fs: VfsPath = EmbeddedFS::<FullBodyParts>::new().into();
    c.bench_function("uv_parts", |b| b.iter(|| black_box(load_parts(&fs))));
}

fn load_parts(root: &VfsPath) -> PartsManager {
    PartsManager::new(root).unwrap()
}

criterion_group!(
    name = benches;
    config = Criterion::default().measurement_time(Duration::from_secs(100));
    targets = bench
);
criterion_main!(benches);
