use std::io::{stdin, stdout};

use crusty::standard::{Client, MoveGenFactory, Eval};
use cli::Cli;

mod cli;
mod uci;

fn main() -> std::io::Result<()> {
    let client = Client::new(MoveGenFactory, Eval);

    let cli = Cli::new(client, stdin(), stdout());

    cli.start()
}