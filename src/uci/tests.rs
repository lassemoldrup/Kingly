use std::io::{empty, sink};
use std::time::Duration;

use crusty::types::{PseudoMove, Square};

use crate::uci::{Command, GoOption, Uci};

fn get_uci<I: Input, O: Output + Send + 'static>(inp: I, out: O) -> Uci<ClientStub, I, O> {
    let search_result = SearchResult::new(
        Value::from_cp(10),
        vec![Move::new_regular(Square::A1, Square::A2)],
        1,
        2,
        100,
        Duration::from_millis(1000),
        0,
    );
    let client = ClientStub::new(search_result);
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
    let moves = vec![PseudoMove::new(Square::A1, Square::A2, None)];

    uci.execute(Command::Position {
        fen: position.to_string(),
        moves: moves.clone(),
    })
    .unwrap();

    assert_eq!(uci.client.lock().unwrap().last_fen, position);
    assert_eq!(moves, uci.client.lock().unwrap().moves_made);

    let position = "rnbqkbnr/ppp2ppp/8/3pp3/3PP3/8/PPP2PPP/RNBQKBNR w KQkq - 0 3";
    uci.execute(Command::Position {
        fen: position.to_string(),
        moves: vec![],
    })
    .unwrap();

    assert_eq!(uci.client.lock().unwrap().last_fen, position);
    assert_eq!(uci.client.lock().unwrap().moves_made, vec![]);
}

#[test]
fn go_infinite_cmd_correctly_starts_search() {
    let mut uci = get_uci(empty(), String::new());

    uci.execute(Command::Go(vec![GoOption::Infinite])).unwrap();
    uci.wait_for_search();
    assert!(uci
        .get_output()
        .starts_with("info depth 1 seldepth 2 score cp 10 nodes 100 nps 100 hashfull 0 pv a1a2"));
}
