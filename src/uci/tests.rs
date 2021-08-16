use crate::uci::{Uci, Command, PseudoMove};
use crusty::test::client::ClientStub;
use std::io::{sink, empty};
use crusty::framework::io::{Output, Input};
use crusty::framework::square::Square;

fn get_uci<I: Input, O: Output + Send + 'static>(inp: I, out: O) -> Uci<ClientStub, I, O> {
    let client = ClientStub::new();
    Uci::new(client, inp, out)
}

#[test]
fn debug_cmd_sets_debug_mode() {
    let mut uci = get_uci(empty(), sink());

    assert!(!uci.debug);

    uci.execute(Command::Debug(true)).unwrap();
    assert!(uci.debug);

    uci.execute(Command::Debug(false)).unwrap();
    assert!(!uci.debug);
}

#[test]
fn is_ready_cmd_response_is_ready_ok() {
    let mut uci = get_uci(empty(), String::new());

    uci.execute(Command::IsReady).unwrap();
    assert_eq!(uci.get_output(), "readyok\n");
}

#[test]
fn position_cmd_updates_position_correctly() {
    let mut uci = get_uci(empty(), sink());

    let position = "rnbqkbnr/pppp1ppp/8/4p3/3P4/8/PPP1PPPP/RNBQKBNR w KQkq - 0 2";
    // Only move allowed by ClientStub
    let moves = vec![PseudoMove{ from: Square::A1, to: Square::A2, promotion: None }];

    uci.execute(Command::Position {
        fen: position.to_string(),
        moves: moves.clone(),
    }).unwrap();

    assert_eq!(uci.client.lock().unwrap().last_fen, position);
    assert_eq!(moves, uci.client.lock().unwrap().moves_made);

    let position = "rnbqkbnr/ppp2ppp/8/3pp3/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 0 3";
    uci.execute(Command::Position {
        fen: position.to_string(),
        moves: vec![],
    }).unwrap();

    assert_eq!(uci.client.lock().unwrap().last_fen, position);
    assert_eq!(uci.client.lock().unwrap().moves_made, vec![]);
}