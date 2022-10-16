use std::sync::atomic::AtomicBool;

use crate::eval::MaterialEval;
use crate::move_gen::MoveGen;
use crate::mv;
use crate::position::Position;
use crate::tables::Tables;
use crate::types::Value;

use super::transposition_table::TranspositionTable;
use super::Search;

fn get_search(position: Position, hash_size: usize) -> Search<'static, 'static, MaterialEval> {
    let move_gen = MoveGen::new(Tables::get());
    let eval = MaterialEval;
    let trans_table = Box::leak(Box::new(TranspositionTable::with_hash_size(hash_size)));

    Search::new(position, move_gen, eval, trans_table)
}

#[test]
fn queen_standoff_should_give_advantage_to_player_to_move() {
    let w_to_move_fen = "4k3/8/8/3q4/3Q4/8/8/4K3 w - - 0 1";
    let b_to_move_fen = "4k3/8/8/3q4/3Q4/8/8/4K3 b - - 0 1";
    let w_to_move_pos = Position::from_fen(w_to_move_fen).unwrap();
    let b_to_move_pos = Position::from_fen(b_to_move_fen).unwrap();

    let mut w_value = Value::centi_pawn(0);
    let mut b_value = Value::centi_pawn(0);

    get_search(w_to_move_pos, 1)
        .depth(1)
        .on_info(|info| w_value = info.score)
        .start(&AtomicBool::new(false));

    get_search(b_to_move_pos, 1)
        .depth(1)
        .on_info(|info| b_value = info.score)
        .start(&AtomicBool::new(false));

    // The player to move should have a value of 900 (1 queen) at depth 1
    assert_eq!(w_value, Value::centi_pawn(900));
    assert_eq!(b_value, Value::centi_pawn(900));
}

#[test]
fn finds_mate_in_two() {
    let fen = "3r2k1/5ppp/8/8/8/8/4R3/K3R3 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let mut value = Value::centi_pawn(0);

    get_search(position, 1)
        .depth(4)
        .on_info(|info| value = info.score)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::mate_in_ply(3));
}

#[test]
fn finds_threefold_repetition() {
    let fen = "6kq/6p1/6Q1/8/8/8/1q6/6K1 w - - 0 1";
    let mut position = Position::from_fen(fen).unwrap();
    unsafe {
        position.make_move(mv!(G6 -> E8));
        position.make_move(mv!(G8 -> H7));
        position.make_move(mv!(E8 -> H5));
        position.make_move(mv!(H7 -> G8));
        position.make_move(mv!(H5 -> E8));
        position.make_move(mv!(G8 -> H7));
    }
    let mut value = Value::centi_pawn(-100);

    get_search(position, 1)
        .depth(4)
        .on_info(|info| value = info.score)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::centi_pawn(0));
}

#[test]
fn finds_fifty_move_draw() {
    let fen = "6kq/8/8/8/5K2/8/8/8 b - - 98 4";
    let position = Position::from_fen(fen).unwrap();
    let mut value = Value::centi_pawn(-100);

    get_search(position, 1)
        .depth(2)
        .on_info(|info| value = info.score)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::centi_pawn(0));
}

#[test]
fn no_fifty_move_draw_on_checkmate() {
    let fen = "7q/5kp1/8/8/8/8/1q6/6K1 w - - 98 2";
    let position = Position::from_fen(fen).unwrap();
    let mut value = Value::centi_pawn(0);

    get_search(position, 1)
        .depth(4)
        .on_info(|info| value = info.score)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::mate_in_ply_neg(2));
}

#[test]
fn find_mate_in_eight() {
    let fen = "3k4/8/8/8/3K4/5R2/8/8 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let mut value = Value::centi_pawn(0);

    get_search(position, 16)
        .depth(15)
        .on_info(|info| value = info.score)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::mate_in_ply(15));
}

#[test]
fn zugzwang_test_position1() {
    let fen = "1q1k4/2Rr4/8/2Q3K1/8/8/8/8 w - - 0 1";
    let position = Position::from_fen(fen).unwrap();
    let mut best_move = mv!();

    get_search(position, 16)
        .depth(8)
        .on_info(|info| best_move = info.pv[0])
        .start(&AtomicBool::new(false));

    assert_eq!(best_move, mv!(G5 -> H6));
}

#[test]
fn zugzwang_test_position2() {
    let fen = "8/8/8/3p1K2/2kP4/8/8/8 w - - 1 1";
    let position = Position::from_fen(fen).unwrap();
    let mut best_move = mv!();

    get_search(position, 16)
        .depth(8)
        .on_info(|info| best_move = info.pv[0])
        .start(&AtomicBool::new(false));

    assert_eq!(best_move, mv!(F5 -> E5));
}
