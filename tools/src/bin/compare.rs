use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::Context;
use clap::Parser;
use interactive_process::InteractiveProcess;
use itertools::Itertools;
use kingly_lib::types::{Color, Move, PseudoMove};
use kingly_lib::{MoveGen, Position};

#[derive(Parser)]
struct App {
    old_bin: PathBuf,
    /// Move time in milliseconds
    #[clap(short, long = "move-time", default_value_t = 100)]
    move_time_ms: u64,
}

fn main() -> anyhow::Result<()> {
    let app = App::parse();
    let old_bin = app.old_bin;

    let status = Command::new("cargo")
        .arg("build")
        .arg("--release")
        .status()?;
    anyhow::ensure!(status.success(), "build failed");

    let (old_tx, old_rx) = crossbeam::channel::unbounded();
    let mut old_cmd = Command::new(old_bin);
    let mut old_proc = InteractiveProcess::new(&mut old_cmd, move |line| {
        old_tx.send(line.unwrap()).unwrap();
    })?;

    let (new_tx, new_rx) = crossbeam::channel::unbounded();
    let mut new_cmd = Command::new("target/release/kingly");
    let mut new_proc = InteractiveProcess::new(&mut new_cmd, move |line| {
        new_tx.send(line.unwrap()).unwrap();
    })?;

    old_proc.send("uci")?;
    new_proc.send("uci")?;

    loop {
        let line = old_rx
            .recv_timeout(Duration::from_secs(1))
            .context("old version timed out on uci")?;
        if line == "uciok" {
            break;
        }
    }
    loop {
        let line = new_rx
            .recv_timeout(Duration::from_secs(1))
            .context("new version timed out on uci")?;
        if line == "uciok" {
            break;
        }
    }

    let mut engines = Engines {
        old_proc,
        new_proc,
        old_rx,
        new_rx,
    };

    let fen = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";
    let res = engines.play_position(fen, app.move_time_ms)?;
    println!("{:?}", res);

    thread::sleep(Duration::from_secs(2));

    engines.new_proc.close();
    engines.old_proc.close();

    Ok(())
}

#[derive(Debug)]
enum GameResult {
    NewWin,
    OldWin,
    Draw,
}

struct Engines {
    old_proc: InteractiveProcess,
    new_proc: InteractiveProcess,
    old_rx: crossbeam::channel::Receiver<String>,
    new_rx: crossbeam::channel::Receiver<String>,
}

impl Engines {
    fn play_position(&mut self, fen: &str, move_time_ms: u64) -> anyhow::Result<GameResult> {
        self.is_ready()?;
        self.old_proc.send("ucinewgame")?;
        self.new_proc.send("ucinewgame")?;

        let mut position = Position::from_fen(fen)?;
        let move_gen = MoveGen::init();
        let mut moves = Vec::new();

        self.is_ready()?;
        loop {
            let (legal_moves, check) = move_gen.gen_all_moves_and_check(&position);
            if legal_moves.is_empty() && check {
                return match position.to_move {
                    Color::White => Ok(GameResult::OldWin),
                    Color::Black => Ok(GameResult::NewWin),
                };
            } else if legal_moves.is_empty() || position.is_draw() {
                return Ok(GameResult::Draw);
            }

            let (to_play_proc, to_play_rx) = match position.to_move {
                Color::White => (&mut self.new_proc, &self.new_rx),
                Color::Black => (&mut self.old_proc, &self.old_rx),
            };
            let moves_str = moves.iter().map(|mv: &Move| mv.to_string()).join(" ");
            to_play_proc.send(&format!("position fen {} moves {}", fen, moves_str))?;
            to_play_proc.send(&format!("go movetime {}", move_time_ms))?;

            let deadline = Instant::now() + Duration::from_millis(move_time_ms + 100);
            let mut chosen_move = None;
            while let Ok(msg) = to_play_rx.recv_deadline(deadline) {
                if msg.starts_with("bestmove") {
                    let best_move = msg
                        .split_whitespace()
                        .nth(1)
                        .context("bestmove not found")?;
                    chosen_move = Some(best_move.parse::<PseudoMove>()?);
                    break;
                }
            }
            let chosen_move = chosen_move
                .context("engine did not return a move")?
                .into_move(&legal_moves)
                .context("engine returned an illegal move")?;
            position.make_move(chosen_move);
            moves.push(chosen_move);

            println!("{position}\n");
        }
    }

    fn is_ready(&mut self) -> anyhow::Result<()> {
        self.old_proc.send("isready")?;
        self.new_proc.send("isready")?;

        loop {
            let line = self
                .old_rx
                .recv_timeout(Duration::from_secs(1))
                .context("old version timed out on isready")?;
            if line == "readyok" {
                break;
            }
        }
        loop {
            let line = self
                .new_rx
                .recv_timeout(Duration::from_secs(1))
                .context("new version timed out on isready")?;
            if line == "readyok" {
                break;
            }
        }

        Ok(())
    }
}
