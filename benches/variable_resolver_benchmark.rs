use criterion::{criterion_group, criterion_main, Criterion, black_box};
use std::fs;
use sox::environment::StoreMode;

pub fn benchmark_resolver(c: &mut Criterion) {
    let mut group = c.benchmark_group("Resolver");
    let source = fs::read_to_string("/Users/ocj/projects/sox/resources/main.sox")
        .expect("Failed to read source code");
    group.bench_function("WithoutResolver",
                         |b| b.iter(|| sox::init::run(black_box(source.clone()), black_box(false), StoreMode::Map)));
    group.bench_function("WithResolver",
                         |b| b.iter(|| sox::init::run(black_box(source.clone()), black_box(true), StoreMode::Vec)));
    group.finish();
}

criterion_group!(benches, benchmark_resolver);
criterion_main!(benches);
