use std::fmt::Display;
use std::io::{self, BufRead, StdoutLock, Write};
use std::str::FromStr;
use std::time::Duration;
use std::{process, thread};

use crossbeam::channel::{self, Receiver, Sender};
use kingly_lib::search::{info_channel, InfoSender, ThreadPool};
use kingly_lib::types::PseudoMove;

#[cfg(test)]
mod tests;

pub struct Uci<W> {
    input_rx: Receiver<String>,
    write_handle: W,
    thread_pool: ThreadPool,
    debug_mode: bool,
}

impl Uci<StdoutLock<'_>> {
    pub fn with_standard_io() -> Self {
        let (input_tx, input_rx) = channel::unbounded();
        thread::spawn(move || {
            let stdin = io::stdin().lock();
            handle_input(stdin, input_tx)
        });
        Self {
            input_rx,
            write_handle: io::stdout().lock(),
            thread_pool: ThreadPool::new(),
            debug_mode: false,
        }
    }
}

impl<W: Write> Uci<W> {
    pub fn repl(mut self) -> io::Result<()> {
        self.print_prelude()?;

        let (info_tx, info_rx) = info_channel();

        loop {
            channel::select! {
                recv(self.input_rx) -> line => {
                    let line = line.expect("sender should be alive");
                    self.handle_command(&line, &info_tx)?;
                }
                recv(info_rx) -> info => {
                    todo!();
                }
            }
        }
    }

    fn print_prelude(&mut self) -> io::Result<()> {
        writeln!(self.write_handle, "id name Kingly")?;
        writeln!(self.write_handle, "id author {}", env!("CARGO_PKG_AUTHORS"))?;
        // TODO: Add constant for max value.
        writeln!(
            self.write_handle,
            "option name Hash type spin default 16 min 1 max 1048576"
        )?;
        writeln!(self.write_handle, "uciok")
    }

    fn print_debug(&mut self, message: impl Display) -> io::Result<()> {
        if self.debug_mode {
            writeln!(self.write_handle, "info Debug: {message}")
        } else {
            Ok(())
        }
    }

    fn handle_command(&mut self, command: &str, info_tx: &InfoSender) -> io::Result<()> {
        if command.trim().is_empty() {
            return Ok(());
        }

        let command = match command.parse::<Command>() {
            Ok(command) => command,
            Err(err) => {
                self.print_debug(format!("Failed to parse command: {err}."))?;
                return Ok(());
            }
        };

        match command {
            Command::Debug(value) => self.debug_mode = value,
            Command::IsReady => writeln!(self.write_handle, "readyok")?,
            Command::SetOption(_) => todo!(),
            Command::UciNewGame => todo!(),
            Command::Position { fen, moves } => todo!(),
            Command::Go(_) => todo!(),
            Command::Stop => {
                self.thread_pool.stop();
            }
            Command::PonderHit => todo!(),
            Command::Quit => process::exit(0),
        }
        Ok(())
    }
}

fn handle_input<R: BufRead>(read_handle: R, tx: Sender<String>) -> io::Result<()> {
    for line in read_handle.lines() {
        tx.send(line?).expect("receiver should be alive");
    }
    Ok(())
}

#[derive(Debug, PartialEq)]
enum Command {
    Debug(bool),
    IsReady,
    SetOption(UciOption),
    UciNewGame,
    Position { fen: String, moves: Vec<PseudoMove> },
    Go(Vec<GoOption>),
    Stop,
    PonderHit,
    Quit,
}

#[derive(Debug, PartialEq)]
enum UciOption {
    Hash(usize),
}

#[derive(thiserror::Error, Debug, PartialEq)]
enum ParseCommandError {
    #[error("Unsupported command: {0}")]
    UnsupportedCommand(String),
    #[error("Missing option")]
    MissingOption,
    #[error("Invalid option: {0}")]
    InvalidOption(String),
    #[error("Unsupported option: {0}")]
    UsupportedOption(String),
}

impl FromStr for Command {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (cmd, opts) = s.split_once(' ').unwrap_or((s, ""));
        let opts = opts.trim_start();
        match cmd {
            "debug" => {
                let value = opts.split_whitespace().next();
                match value {
                    Some("on") => Ok(Self::Debug(true)),
                    Some("off") => Ok(Self::Debug(false)),
                    Some(val) => Err(ParseCommandError::InvalidOption(val.into())),
                    None => Err(ParseCommandError::MissingOption),
                }
            }
            "isready" => Ok(Self::IsReady),
            "setoption" => {
                let mut opts = opts.split_whitespace();
                match opts.next() {
                    Some("Hash") => {
                        let value = parse_next_option(&mut opts)?;
                        Ok(Self::SetOption(UciOption::Hash(value)))
                    }
                    Some(name) => Err(ParseCommandError::UsupportedOption(name.into())),
                    None => Err(ParseCommandError::MissingOption),
                }
            }
            "ucinewgame" => Ok(Self::UciNewGame),
            "position" => {
                let (pos, moves) = opts.split_once("moves").unwrap_or((opts, ""));
                let (mode, rest) = pos.split_once(' ').unwrap_or((pos, ""));
                let fen = match mode {
                    // TODO
                    "startpos" => String::new(),
                    "fen" if rest.trim().is_empty() => {
                        return Err(ParseCommandError::MissingOption)
                    }
                    "fen" => rest.trim().into(),
                    "" => return Err(ParseCommandError::MissingOption),
                    _ => return Err(ParseCommandError::InvalidOption(mode.into())),
                };
                let moves = moves
                    .split_whitespace()
                    .map_while(|s| s.parse().ok())
                    .collect();
                Ok(Self::Position { fen, moves })
            }
            "go" => {
                let mut options = Vec::new();
                let mut opts = opts.split_whitespace().peekable();
                while let Some(opt) = opts.next() {
                    match opt {
                        "searchmoves" => {
                            let mut moves = Vec::new();
                            while let Some(mv) = opts.peek() {
                                // If we can parse the move, add it to the list of moves.
                                // Otherwise, it is probably a different option, so we break.
                                let Ok(mv) = mv.parse() else {
                                    break;
                                };
                                opts.next();
                                moves.push(mv);
                            }
                            options.push(GoOption::SearchMoves(moves));
                        }
                        "ponder" => options.push(GoOption::Ponder),
                        "wtime" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::WTime(value));
                        }
                        "btime" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::BTime(value));
                        }
                        "winc" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::WInc(value));
                        }
                        "binc" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::BInc(value));
                        }
                        "movestogo" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::MovesToGo(value));
                        }
                        "depth" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::Depth(value));
                        }
                        "nodes" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::Nodes(value));
                        }
                        "mate" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::Mate(value));
                        }
                        "movetime" => {
                            let value = parse_next_option(&mut opts)?;
                            options.push(GoOption::MoveTime(Duration::from_millis(value)));
                        }
                        "infinite" => options.push(GoOption::Infinite),
                        _ => break,
                    }
                }
                Ok(Self::Go(options))
            }
            "stop" => Ok(Self::Stop),
            "ponderhit" => Ok(Self::PonderHit),
            "quit" => Ok(Self::Quit),
            _ => Err(ParseCommandError::UnsupportedCommand(s.into())),
        }
    }
}

fn parse_next_option<'cmd, T: FromStr>(
    mut opts: impl Iterator<Item = &'cmd str>,
) -> Result<T, ParseCommandError> {
    let value = opts.next().ok_or(ParseCommandError::MissingOption)?;
    value
        .parse()
        .map_err(|_| ParseCommandError::InvalidOption(value.into()))
}

#[derive(Debug, PartialEq)]
pub enum GoOption {
    SearchMoves(Vec<PseudoMove>),
    Ponder,
    WTime(u32),
    BTime(u32),
    WInc(u32),
    BInc(u32),
    MovesToGo(u32),
    Depth(u8),
    Nodes(u64),
    Mate(u32),
    MoveTime(Duration),
    Infinite,
}
