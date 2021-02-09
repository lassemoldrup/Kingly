use std::io::{stdin, stdout};

use crusty::standard::game::StandardGame;

use crate::cli::Cli;

mod cli;

fn main() -> std::io::Result<()> {
    let game = StandardGame::new();
    let std_in = stdin();
    let std_out = stdout();

    let cli = Cli::new(game, std_in.lock(), std_out.lock());

    cli.start()
}