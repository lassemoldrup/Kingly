use std::sync::atomic::AtomicBool;

use move_gen::MoveGen;

use crate::{standard::{Position, eval::MaterialEval, move_gen, tables::Tables, search::Search}, framework::{search::Search as SearchT, value::Value}};

#[test]
fn queen_standoff_should_give_advantage_to_player_to_move() {
    let eval = MaterialEval;
    let move_gen = MoveGen::new(Tables::get());

    let w_to_move_pos = Position::from_fen("4k3/8/8/3q4/3Q4/8/8/4K3 w - - 0 1").unwrap();
    let b_to_move_pos = Position::from_fen("4k3/8/8/3q4/3Q4/8/8/4K3 b - - 0 1").unwrap();

    let mut w_value = Value::CentiPawn(0);
    let mut b_value = Value::CentiPawn(0);

    Search::new(w_to_move_pos, &move_gen, &eval)
        .depth(1)
        .on_info(|sr| w_value = sr.value())
        .start(&AtomicBool::new(false));

    Search::new(b_to_move_pos, &move_gen, &eval)
        .depth(1)
        .on_info(|sr| b_value = sr.value())
        .start(&AtomicBool::new(false));

    // The player to move should have a value of 900 (1 queen) at depth 1
    assert_eq!(w_value, Value::CentiPawn(900));
    assert_eq!(b_value, Value::CentiPawn(900));
}

#[test]
fn finds_mate_in_two() {
    let eval = MaterialEval;
    let move_gen = MoveGen::new(Tables::get());

    let position = Position::from_fen("3r2k1/5ppp/8/8/8/8/4R3/K3R3 w - - 0 1").unwrap();
    let mut value = Value::CentiPawn(0);

    Search::new(position, &move_gen, &eval)
        .depth(4)
        .on_info(|sr| value = sr.value())
        .start(&AtomicBool::new(false));

    assert_eq!(value, Value::Inf(2));
}