use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crusty::types::{Move};
use crusty::position::Position;
use arrayvec::ArrayVec;
//use crusty::move_gen::perft::perft;
use crusty::move_gen::{gentest, init_non_sliders, init_magics};

pub fn criterion_benchmark(c: &mut Criterion) {
    init_non_sliders();
    init_magics();
    let test = black_box(Position::new_default());
    let mut av = ArrayVec::<[Move; 256]>::new();

    c.bench_function("Gen all", |b| b.iter(|| {
        av = ArrayVec::<[Move; 256]>::new();
        unsafe { gentest(&test, &mut av) }
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);