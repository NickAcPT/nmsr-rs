use criterion::{black_box, criterion_group, criterion_main, Criterion};
use nmsr_lib::parts::manager::PartsManager;
use rust_embed::RustEmbed;
use vfs::{EmbeddedFS, VfsPath};

#[derive(RustEmbed, Debug)]
#[folder = "benches/renders/"]
struct FullBodyParts;

fn bench(c: &mut Criterion) {
    let fs: VfsPath = EmbeddedFS::<FullBodyParts>::new().into();
    
    let mut group = c.benchmark_group("nmsr-rs");
    group.sampling_mode(criterion::SamplingMode::Flat);
    group.bench_function("uv_loading", |b| b.iter(|| black_box(load_parts(&fs))));
    group.finish();
}

fn load_parts(root: &VfsPath) -> PartsManager {
    PartsManager::new(root).unwrap()
}

criterion_group!(benches, bench);
criterion_main!(benches);
