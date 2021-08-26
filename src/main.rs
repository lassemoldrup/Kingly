use std::io::{stdin, stdout};

use cli::Cli;
use crusty::standard::{Client, Eval, MoveGen};

mod cli;
mod uci;

fn main() -> std::io::Result<()> {
    let client = Client::<MoveGen, Eval>::new();

    let cli = Cli::new(client, stdin(), stdout());

    cli.start()
}