use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crusty::types::{Move};
use crusty::position::Position;
use arrayvec::ArrayVec;
//use crusty::move_gen::perft::perft;
use crusty::move_gen::{gentest, init_non_sliders, init_magics};

pub fn criterion_benchmark(c: &mut Criterion) {
    let mut test = black_box(Position::new_default());
    test.set("4k3/8/8/2Pp4/8/8/8/4K3 w - d6 0 1").unwrap();
    let mut av = ArrayVec::<[Move; 256]>::new();
    init_non_sliders();
    init_magics();

    c.bench_function("En passant", |b| b.iter(|| {
        av = ArrayVec::<[Move; 256]>::new();
        unsafe { gentest(&test, &mut av) }
    }));

    let test = black_box(Position::new_default());
    c.bench_function("Initial pawns", |b| b.iter(|| {
        av = ArrayVec::<[Move; 256]>::new();
        unsafe { gentest(&test, &mut av) }
    }));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);