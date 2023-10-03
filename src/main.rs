use std::io;

use clap::{Parser, Subcommand};
use uci::Uci;

mod uci;

#[derive(Parser)]
struct App {
    #[clap(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(arg_required_else_help = true)]
    Perft { fen: String, depth: u8 },
}

fn main() -> io::Result<()> {
    let app = App::parse();
    match app.command {
        Some(Command::Perft { fen, depth }) => println!("Perft {depth} {fen}"),
        None => Uci::new_standard().repl()?,
    }
    Ok(())
}
