use std::io::{stdin, stdout};

use cli::Cli;
use crusty::{standard::{Client, Eval, MoveGen}, framework::io::{LoggingInput, LoggingOutput}};

mod cli;
mod uci;

fn main() -> std::io::Result<()> {
    let client = Client::<MoveGen, Eval>::new();

    let input = LoggingInput::new(stdin());
    let output = LoggingOutput::new(stdout());
    let cli = Cli::new(client, input, output);

    cli.start()
}