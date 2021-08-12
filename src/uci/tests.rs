use crate::uci::{Uci, Command};
use crusty::test::client::ClientStub;
use std::io::{sink, empty};
use crusty::framework::io::{Output, Input};
use crusty::framework::moves::Move;
use crusty::framework::square::Square;

fn get_uci<I: Input, O: Output>(inp: I, out: O) -> Uci<ClientStub, I, O> {
    let client = ClientStub::new();
    Uci::new(client, inp, out)
}

#[test]
fn debug_cmd_sets_debug_mode() {
    let mut uci = get_uci(empty(), sink());

    assert!(!uci.debug);
    uci.execute(Command::Debug(true)).unwrap();
    assert!(uci.debug);
}

#[test]
fn is_ready_cmd_response_is_ready_ok() {
    let mut output = String::new();
    let mut uci = get_uci(empty(), &mut output);

    uci.execute(Command::IsReady).unwrap();
    assert_eq!(output, "readyok\n");
}

#[test]
fn position_cmd_updates_position_correctly() {
    let mut uci = get_uci(empty(), sink());

    let position = "rnbqkbnr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
    let moves = vec![Move::Regular(Square::E2, Square::E4),
                     Move::Regular(Square::E5, Square::D4)];
    uci.execute(Command::Position {
        fen: position.to_string(),
        moves: moves.clone(),
    }).unwrap();

    assert_eq!(uci.client.borrow().last_fen, position);
    assert_eq!(uci.client.borrow().moves_made, moves);

    let position = "rnbqkbnr/ppp2ppp/8/3pp3/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 0 3";
    uci.execute(Command::Position {
        fen: position.to_string(),
        moves: vec![],
    }).unwrap();

    assert_eq!(uci.client.borrow().last_fen, position);
    assert_eq!(uci.client.borrow().moves_made, vec![]);
}