use std::io::{stdin, stdout};

use crusty::standard::{Client, MoveGenFactory, Eval};
use cli::Cli;

mod cli;
mod uci;

fn main() -> std::io::Result<()> {
    let client = Client::new(MoveGenFactory, Eval);
    let std_in = stdin();
    let std_out = stdout();

    let cli = Cli::new(client, std_in.lock(), std_out);

    cli.start()
}