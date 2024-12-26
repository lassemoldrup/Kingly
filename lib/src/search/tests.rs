use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Instant;

use crate::eval::MaterialEval;
use crate::mv;
use crate::position::Position;
use crate::search::thread::SearchInfo;
use crate::search::ThreadPool;
use crate::types::Value;

use super::{SearchJob, SearchResult, TranspositionTable};

fn search(position: Position, depth: i8) -> SearchResult {
    let kill_switch = Arc::new(AtomicBool::new(false));
    let t_table = Arc::new(TranspositionTable::with_hash_size(1));
    SearchJob::default_builder()
        .position(position)
        .build()
        .search(depth, Instant::now(), kill_switch, t_table)
        .unwrap()
}

fn search_material(position: Position, depth: i8) -> SearchResult {
    let kill_switch = Arc::new(AtomicBool::new(false));
    let t_table = Arc::new(TranspositionTable::with_hash_size(1));
    SearchJob::builder(MaterialEval)
        .position(position)
        .build()
        .search(depth, Instant::now(), kill_switch, t_table)
        .unwrap()
}

fn search_threaded(position: Position, depth: i8) -> SearchResult {
    let mut thread_pool = ThreadPool::new();
    let job = SearchJob::default_builder()
        .position(position)
        .depth(depth)
        .build();
    let rx = thread_pool.spawn(job).unwrap();
    let SearchInfo::Finished(best_mv) = rx.iter().last().unwrap() else {
        panic!("Last search info was not Finished");
    };
    let result = thread_pool.wait().unwrap();
    assert_eq!(result.pv[0], best_mv);
    result
}

#[test]
fn queen_standoff_should_give_advantage_to_player_to_move() {
    let w_to_move_fen = "4k3/8/8/3q4/3Q4/8/8/4K3 w - - 0 1";
    let b_to_move_fen = "4k3/8/8/3q4/3Q4/8/8/4K3 b - - 0 1";
    let w_to_move_pos = Position::from_fen(w_to_move_fen).unwrap();
    let b_to_move_pos = Position::from_fen(b_to_move_fen).unwrap();

    let w_res = search_material(w_to_move_pos, 1);
    let b_res = search_material(b_to_move_pos, 1);

    // The player to move should have a value of 900 (1 queen) at depth 1
    assert_eq!(w_res.score, Value::centipawn(900));
    assert_eq!(b_res.score, Value::centipawn(900));
}

#[test]
fn finds_mate_in_two() {
    let fen = "3r2k1/5ppp/8/8/8/8/4R3/K3R3 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();

    let res = search(position, 4);
    assert_eq!(res.score, Value::mate_in_ply(3));
}

#[test]
fn finds_threefold_repetition() {
    let fen = "6kq/6p1/6Q1/8/8/8/1q6/6K1 w - - 0 1";
    let mut position = Position::from_fen(fen).unwrap();
    position.make_move(mv!(G6 -> E8));
    position.make_move(mv!(G8 -> H7));
    position.make_move(mv!(E8 -> H5));
    position.make_move(mv!(H7 -> G8));
    position.make_move(mv!(H5 -> E8));
    position.make_move(mv!(G8 -> H7));

    let res = search(position, 4);
    assert_eq!(res.score, Value::centipawn(0));
}

#[test]
fn finds_fifty_move_draw() {
    let fen = "6kq/8/8/8/5K2/8/8/8 b - - 98 4";
    let position = Position::from_fen(fen).unwrap();

    let res = search(position, 2);
    assert_eq!(res.score, Value::centipawn(0));
}

#[test]
fn no_fifty_move_draw_on_checkmate() {
    let fen = "7q/5kp1/8/8/8/8/1q6/6K1 w - - 98 2";
    let position = Position::from_fen(fen).unwrap();

    let res = search(position, 4);
    assert_eq!(res.score, Value::neg_mate_in_ply(2));
}

#[test]
fn find_mate_in_eight() {
    let fen = "3k4/8/8/8/3K4/5R2/8/8 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();

    let res = search_threaded(position, 15);
    assert_eq!(res.score, Value::mate_in_ply(15));
    assert_eq!(res.pv[0], mv!(F3 -> F7));
}

#[test]
fn zugzwang_test_position_1() {
    let fen = "1q1k4/2Rr4/8/2Q3K1/8/8/8/8 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();

    let res = search_threaded(position, 8);
    assert_eq!(res.pv[0], mv!(G5 -> H6));
}

#[test]
fn zugzwang_test_position_2() {
    let fen = "8/8/8/3p1K2/2kP4/8/8/8 w - - 1 1";
    let position = Position::from_fen(fen).unwrap();

    let res = search_threaded(position, 8);
    assert_eq!(res.pv[0], mv!(F5 -> E5));
}
