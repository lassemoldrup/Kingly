use std::io::{stdin, stdout};

use cli::Cli;
use crusty::standard::{Client, Eval, MoveGenFactory};

mod cli;
mod uci;

fn main() -> std::io::Result<()> {
    let client = Client::new(MoveGenFactory, Eval);

    let cli = Cli::new(client, stdin(), stdout());

    cli.start()
}