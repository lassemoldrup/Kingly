use std::io::{empty, sink};

use crate::client::Client;
use crate::io::{Input, Output};
use crate::uci::{Command, Uci};

fn get_uci<I: Input, O: Output + Send + 'static>(inp: I, out: O) -> Uci<I, O> {
    let client = Client::new();
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
    assert_eq!(uci.writer.get_output(), "readyok\n");
}
