use std::sync::atomic::AtomicBool;

use move_gen::MoveGen;

use crate::{standard::{Position, eval::MaterialEval, move_gen, tables::Tables, search::Search}, framework::{search::Search as SearchT, value::Value, moves::Move, square::Square}};

use super::transposition_table::TranspositionTable;

fn get_search(position: Position) -> Search<'static, MoveGen, MaterialEval> {
    let move_gen = Box::leak(Box::new(MoveGen::new(Tables::get())));
    let eval = Box::leak(Box::new(MaterialEval));
    let trans_table = Box::leak(Box::new(TranspositionTable::with_capacity(0)));

    Search::new(position, move_gen, eval, trans_table)
}

#[test]
fn queen_standoff_should_give_advantage_to_player_to_move() {
    let w_to_move_pos = Position::from_fen("4k3/8/8/3q4/3Q4/8/8/4K3 w - - 0 1").unwrap();
    let b_to_move_pos = Position::from_fen("4k3/8/8/3q4/3Q4/8/8/4K3 b - - 0 1").unwrap();

    let mut w_value = Value::CentiPawn(0);
    let mut b_value = Value::CentiPawn(0);

    get_search(w_to_move_pos)
        .depth(1)
        .on_info(|sr| w_value = sr.value)
        .start(&AtomicBool::new(false));

    get_search(b_to_move_pos)
        .depth(1)
        .on_info(|sr| b_value = sr.value)
        .start(&AtomicBool::new(false));

    // The player to move should have a value of 900 (1 queen) at depth 1
    assert_eq!(w_value, Value::CentiPawn(900));
    assert_eq!(b_value, Value::CentiPawn(900));
}

#[test]
fn finds_mate_in_two() {
    let position = Position::from_fen("3r2k1/5ppp/8/8/8/8/4R3/K3R3 w - - 0 1").unwrap();
    let mut value = Value::CentiPawn(0);

    get_search(position)
        .depth(4)
        .on_info(|sr| value = sr.value)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::Inf(2));
}

#[test]
fn finds_threefold_repetition() {
    let mut position = Position::from_fen("6kq/6p1/6Q1/8/8/8/1q6/6K1 w - - 0 1").unwrap();
    unsafe {
        use Square::*;
        position.make_move(Move::Regular(G6, E8));
        position.make_move(Move::Regular(G8, H7));
        position.make_move(Move::Regular(E8, H5));
        position.make_move(Move::Regular(H7, G8));
        position.make_move(Move::Regular(H5, E8));
        position.make_move(Move::Regular(G8, H7));
    }
    let mut value = Value::CentiPawn(-100);

    get_search(position)
        .depth(4)
        .on_info(|sr| value = sr.value)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::CentiPawn(0));
}

#[test]
fn finds_fifty_move_draw() {
    let position = Position::from_fen("6kq/8/8/8/5K2/8/8/8 b - - 98 4").unwrap();
    let mut value = Value::CentiPawn(-100);

    get_search(position)
        .depth(2)
        .on_info(|sr| value = sr.value)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::CentiPawn(0));
}

#[test]
fn no_fifty_move_draw_on_checkmate() {
    let position = Position::from_fen("7q/5kp1/8/8/8/8/1q6/6K1 w - - 98 2").unwrap();
    let mut value = Value::CentiPawn(0);

    get_search(position)
        .depth(4)
        .on_info(|sr| value = sr.value)
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::NegInf(1));
}
