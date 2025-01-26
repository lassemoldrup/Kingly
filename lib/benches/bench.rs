use criterion::{black_box, criterion_group, criterion_main, Bencher, Criterion};
use kingly_lib::{bb, MoveGen, Position};

pub fn move_gen_some_pos_benchmark(c: &mut Criterion) {
    let move_gen = MoveGen::init();
    let pos = Position::from_fen("rn1qk2r/pp3ppp/2pb1p2/8/2BP2b1/5N2/PPP2PPP/R1BQK2R w KQkq - 4 8")
        .unwrap();
    c.bench_function("gen moves some pos", move |b: &mut Bencher| {
        b.iter(|| move_gen.gen_all_moves_and_check(black_box(&pos)))
    });
}

pub fn iter_bench(c: &mut Criterion) {
    let bitboard = bb!(A5, B4, E8, H4, H5, H6);

    c.bench_function("bb iter", move |b: &mut Bencher| {
        b.iter(|| {
            for sq in black_box(bitboard) {
                black_box(sq);
            }
        })
    });
}

criterion_group!(benches, move_gen_some_pos_benchmark, iter_bench);

criterion_main!(benches);
