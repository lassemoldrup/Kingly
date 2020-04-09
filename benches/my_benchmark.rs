use criterion::{black_box, criterion_group, criterion_main, Criterion};
use crusty::types::Bitboard;

pub fn criterion_benchmark(c: &mut Criterion) {
    let test = black_box(Bitboard::new(2547243619602955089));
    c.bench_function("Count", |b| b.iter(|| test.iter().count()));
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);