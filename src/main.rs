use crate::cli::Cli;
use std::io::{stdin, stdout, Write};
use crusty::standard::game::StandardGame;

mod cli;

fn main() -> std::io::Result<()> {
    let game = StandardGame::new();
    let std_in = stdin();
    let std_out = stdout();

    let cli = Cli::new(game, std_in.lock(), std_out.lock());

    cli.start();

    Ok(())
}