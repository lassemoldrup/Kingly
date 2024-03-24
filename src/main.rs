use std::io;
use std::time::Instant;

use clap::{Parser, Subcommand};
use kingly_lib::position::ParseFenError;
use kingly_lib::MoveGen;
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
    #[command(arg_required_else_help = true)]
    Divide { fen: String, depth: u8 },
}

#[derive(thiserror::Error, Debug)]
enum Error {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    ParseFen(#[from] ParseFenError),
    #[error("invalid divide depth")]
    InvalidDivideDepth,
}

fn main() -> Result<(), Error> {
    let app = App::parse();
    match app.command {
        Some(Command::Perft { fen, depth }) => {
            let position = fen.parse()?;
            let move_gen = MoveGen::init();
            let start = Instant::now();
            let res = move_gen.perft(position, depth);
            let elapsed = start.elapsed();
            println!("Nodes: {}, Elapsed: {} ms", res, elapsed.as_millis());
        }
        Some(Command::Divide { fen, depth }) => {
            if depth == 0 {
                return Err(Error::InvalidDivideDepth);
            }
            let mut position = fen.parse()?;
            let move_gen = MoveGen::init();
            let moves = move_gen.gen_all_moves(&position);
            let mut total = 0;
            for &mv in &moves {
                position.make_move(mv);
                let count = move_gen.perft(position.clone(), depth - 1);
                position.unmake_move();
                total += count;
                println!("{mv}: {count}",);
            }
            println!("Moves: {}, Total: {total}", moves.len());
        }
        None => Uci::with_standard_io().repl()?,
    }
    Ok(())
}
