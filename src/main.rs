use std::io;
use std::time::Instant;

use clap::{Parser, Subcommand};
use kingly_lib::position::ParseFenError;
use kingly_lib::search::{SearchInfo, SearchJob, ThreadPool};
use kingly_lib::{MoveGen, Position};
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
    Perft { fen: String, depth: i8 },
    #[command(arg_required_else_help = true)]
    Divide { fen: String, depth: i8 },
    /// Required for OpenBench - tests the search performance of the system
    Bench,
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
    pretty_env_logger::init();

    let app = App::parse();
    match app.command {
        Some(Command::Perft { fen, depth }) => {
            let position = fen.parse()?;
            let move_gen = MoveGen::init();
            let start = Instant::now();
            let res = move_gen.perft(position, depth);
            let elapsed = start.elapsed();
            println!("Nodes:\t\t{res}");
            println!("Elapsed:\t{} ms", elapsed.as_millis());
            println!("NPS:\t\t{} kn/s", res / elapsed.as_millis() as u64);
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
            println!("Moves: {}", moves.len());
            println!("Total: {total}");
        }
        Some(Command::Bench) => {
            let mut thread_pool = ThreadPool::new();
            let job = SearchJob::default_builder()
                .position(Position::new())
                .depth(12)
                .build();
            thread_pool
                .set_num_threads(1)
                .expect("search is not running");
            let rx = thread_pool.run(job).expect("search is not running");
            let mut seach_nps = 0;
            let mut nodes = 0;
            while let Ok(info) = rx.recv() {
                if let SearchInfo::NewDepth { stats, nps, .. } = info {
                    seach_nps = nps;
                    nodes = stats.nodes;
                }
            }
            thread_pool.wait();
            println!("{nodes} nodes {seach_nps} nps");
        }
        None => Uci::with_standard_io().repl()?,
    }
    Ok(())
}
