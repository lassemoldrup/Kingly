use std::sync::atomic::AtomicBool;

use criterion::{criterion_group, criterion_main, BatchSize, Criterion};
use crusty::eval::{Eval, StandardEval};
use crusty::search::TranspositionTable;
use crusty::{move_gen::MoveGen, position::Position, search::Search, tables::Tables};

fn bench_pos(c: &mut Criterion, id: &str, position: Position, depth: u8) {
    let move_gen = MoveGen::new(Tables::get());
    let eval = StandardEval::create();
    let stop_search = AtomicBool::new(false);

    c.bench_function(id, |b| {
        b.iter_batched(
            || (position.clone(), TranspositionTable::new()),
            |(pos, ref mut trans_table)| {
                Search::new(pos, move_gen, eval, trans_table)
                    .depth(depth)
                    .start(&stop_search);
            },
            BatchSize::SmallInput,
        )
    });
}

fn start_pos_depth_6(c: &mut Criterion) {
    let position = Position::new();
    bench_pos(c, "search startpos d 6", position, 6);
}

fn kaufman_pos_1_depth_4(c: &mut Criterion) {
    let position =
        Position::from_fen("1rbq1rk1/p1b1nppp/1p2p3/8/1B1pN3/P2B4/1P3PPP/2RQ1R1K w - - 0 1")
            .unwrap();
    bench_pos(c, "search kaufman 1 d 4", position, 4);
}

fn kaufman_pos_2_depth_4(c: &mut Criterion) {
    let position =
        Position::from_fen("3r2k1/p2r1p1p/1p2p1p1/q4n2/3P4/PQ5P/1P1RNPP1/3R2K1 b - - 0 1").unwrap();
    bench_pos(c, "search kaufman 2 d 4", position, 4);
}

fn kaufman_pos_3_depth_4(c: &mut Criterion) {
    let position =
        Position::from_fen("3r2k1/1p3ppp/2pq4/p1n5/P6P/1P6/1PB2QP1/1K2R3 w - - 0 1").unwrap();
    bench_pos(c, "search kaufman 3 d 4", position, 4);
}

fn kaufman_pos_4_depth_3(c: &mut Criterion) {
    let position =
        Position::from_fen("r1b1r1k1/1ppn1p1p/3pnqp1/8/p1P1P3/5P2/PbNQNBPP/1R2RB1K w - - 0 1")
            .unwrap();
    bench_pos(c, "search kaufman 4 d 3", position, 3);
}

criterion_group!(
    name = fast;
    config = Criterion::default().sample_size(20);
    targets =
        start_pos_depth_6,
        kaufman_pos_1_depth_4,
        kaufman_pos_2_depth_4,
        kaufman_pos_3_depth_4,
        kaufman_pos_4_depth_3
);

criterion_group!(
    name = slow;
    config = Criterion::default().sample_size(100);
    targets =
        start_pos_depth_6,
        kaufman_pos_1_depth_4,
        kaufman_pos_2_depth_4,
        kaufman_pos_3_depth_4,
        kaufman_pos_4_depth_3
);

criterion_main!(fast);
