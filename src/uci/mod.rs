use std::fmt::{self, Display, Formatter};
use std::io::{self, BufRead, StdoutLock, Write};
use std::str::FromStr;
use std::time::Duration;
use std::{panic, process, thread};

use crossbeam::channel::{self, Receiver, Sender};
use kingly_lib::position::{ParseFenError, STARTING_FEN};
use kingly_lib::search::{info_channel, InfoSender, SearchInfo, SearchJob, ThreadPool};
use kingly_lib::tables::Tables;
use kingly_lib::time_mananger::TimeControl;
use kingly_lib::types::{Color, IllegalMoveError, PseudoMove};
use kingly_lib::{MoveGen, Position};
use once_cell::unsync::Lazy;

#[cfg(test)]
mod tests;

pub struct Uci<W> {
    input_rx: Receiver<String>,
    write_handle: W,
    // TODO: Switch to LazyCell, once DerefMut is stabilized
    position: Lazy<Position>,
    debug_mode: bool,
    thread_pool: Lazy<ThreadPool>,
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
            position: Lazy::new(Position::new),
            debug_mode: false,
            thread_pool: Lazy::new(ThreadPool::new),
        }
    }
}

impl<W: Write> Uci<W> {
    pub fn repl(mut self) -> io::Result<()> {
        // Ensure that the process exits if a search panics
        let orig_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            orig_hook(panic_info);
            process::exit(1);
        }));

        let (info_tx, info_rx) = info_channel();
        loop {
            channel::select! {
                recv(self.input_rx) -> line => {
                    let Ok(line) = line else {
                        // IO channel is closed, exit the loop
                        return Ok(());
                    };
                    if let Err(err) = self.handle_command(&line, &info_tx) {
                        match err {
                            UciError::Io(err) => return Err(err),
                            _ => self.print_debug(err)?,
                        }
                    }
                }
                recv(info_rx) -> info => {
                    let info = info.expect("sender is alive");
                    self.print_info(&info)?;
                    // Make sure that the engine is ready to search again
                    if let SearchInfo::Finished(_) = info {
                        self.thread_pool.wait();
                    }
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
        writeln!(
            self.write_handle,
            "option name Threads type spin default 6 min 1 max 64"
        )?;
        writeln!(self.write_handle, "uciok")?;
        self.write_handle.flush()
    }

    fn print_debug(&mut self, message: impl Display) -> io::Result<()> {
        if self.debug_mode {
            writeln!(self.write_handle, "info string Debug: {message}")?;
            self.write_handle.flush()
        } else {
            Ok(())
        }
    }

    fn print_info(&mut self, info: &SearchInfo) -> io::Result<()> {
        match info {
            SearchInfo::NewDepth {
                depth,
                evaluation,
                stats,
                nps,
                total_duration,
                hash_full,
            } => {
                write!(
                    self.write_handle,
                    "info depth {} seldepth {} score {} nodes {} nps {} hashfull {} pv",
                    depth, stats.sel_depth, evaluation.score, stats.nodes, nps, hash_full,
                )?;
                for mv in &evaluation.pv {
                    write!(self.write_handle, " {}", mv)?;
                }
                writeln!(self.write_handle, " time {}", total_duration.as_millis())?;
            }
            SearchInfo::Finished(best_mv) => {
                writeln!(self.write_handle, "bestmove {}", best_mv)?;
            }
        }
        self.write_handle.flush()
    }

    fn handle_command(&mut self, command: &str, info_tx: &InfoSender) -> Result<(), UciError> {
        if command.trim().is_empty() {
            return Ok(());
        }

        let command: Command = command.parse()?;
        self.print_debug(format!("Received command: {command}"))?;
        match command {
            Command::Uci => {
                self.print_prelude()?;
                Tables::init();
                Lazy::force(&self.position);
                Lazy::force(&self.thread_pool);
            }
            Command::Debug(value) => self.debug_mode = value,
            Command::IsReady => {
                writeln!(self.write_handle, "readyok")?;
                self.write_handle.flush()?;
            }
            Command::SetOption(opt) => match opt {
                UciOption::Hash(size) => {
                    if self.thread_pool.set_hash_size(size).is_err() {
                        self.print_debug("Cannot set hash size while search is running")?;
                    }
                }
                UciOption::Threads(threads) => {
                    if self.thread_pool.set_num_threads(threads).is_err() {
                        self.print_debug("Cannot set threads while search is running")?;
                    }
                }
            },
            Command::UciNewGame => {
                if self.thread_pool.clear_t_table().is_err() {
                    self.print_debug("Search is running")?;
                }
                self.position.set_fen(STARTING_FEN)?;
            }
            Command::Position { fen, moves } => {
                self.position.set_fen(&fen)?;
                let move_gen = MoveGen::init();
                for mv in moves {
                    let legal = move_gen.gen_all_moves(&self.position);
                    self.position.make_move(mv.into_move(&legal)?);
                }
            }
            Command::Go(options) => {
                let mut builder = SearchJob::default_builder().position(self.position.clone());
                let mut white_tc = None;
                let mut black_tc = None;
                let mut move_time = None;
                for opt in options {
                    match opt {
                        GoOption::SearchMoves(moves) => builder = builder.moves(moves)?,
                        GoOption::Depth(depth) => builder = builder.depth(depth),
                        GoOption::Nodes(nodes) => builder = builder.nodes(nodes),
                        GoOption::MoveTime(time) => {
                            move_time = Some(time);
                        }
                        GoOption::Infinite => {}
                        GoOption::WTime(time) => {
                            white_tc
                                .get_or_insert_with(TimeControl::default)
                                .time_remaining = Duration::from_millis(time as u64);
                        }
                        GoOption::WInc(inc) => {
                            white_tc.get_or_insert_with(TimeControl::default).increment =
                                Duration::from_millis(inc as u64);
                        }
                        GoOption::BTime(time) => {
                            black_tc
                                .get_or_insert_with(TimeControl::default)
                                .time_remaining = Duration::from_millis(time as u64);
                        }
                        GoOption::BInc(inc) => {
                            black_tc.get_or_insert_with(TimeControl::default).increment =
                                Duration::from_millis(inc as u64);
                        }
                        _ => {
                            self.print_debug(format!("Unsupported option: {opt}"))?;
                        }
                    };
                }

                if let Some(time) = move_time {
                    builder = builder.time(time);
                } else if self.position.to_move == Color::White {
                    if let Some(tc) = white_tc {
                        builder = builder.time(tc.time_man()).allow_early_stop(true);
                    }
                } else {
                    if let Some(tc) = black_tc {
                        builder = builder.time(tc.time_man()).allow_early_stop(true);
                    }
                }

                let job = builder.build();
                if self
                    .thread_pool
                    .run_with_channel(job, info_tx.clone())
                    .is_err()
                {
                    self.print_debug("Search already in progress")?;
                }
            }
            Command::Stop => {
                self.thread_pool.stop();
            }
            Command::PonderHit => {
                self.print_debug("Unsupported command: ponderhit")?;
            }
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

#[derive(Debug, thiserror::Error)]
enum UciError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("failed to parse FEN string: {0}")]
    ParseFen(#[from] ParseFenError),
    #[error("failed to parse command: {0}")]
    ParseCommand(#[from] ParseCommandError),
    #[error("illegal move: {0}")]
    IllegalMove(#[from] IllegalMoveError),
}

#[derive(Debug, PartialEq)]
enum Command {
    Uci,
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
    Threads(usize),
}

impl Display for UciOption {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            UciOption::Hash(value) => write!(f, "Hash value {value}"),
            UciOption::Threads(value) => write!(f, "Threads value {value}"),
        }
    }
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
    #[error("The 'setoption' command should start 'setoption name'")]
    MissingNameKeyword,
    #[error("The '{0}' option should be followed by the 'value' keyword")]
    MissingValueKeyword(String),
}

impl FromStr for Command {
    type Err = ParseCommandError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (cmd, opts) = s.split_once(' ').unwrap_or((s, ""));
        let opts = opts.trim_start();
        match cmd {
            "uci" => Ok(Self::Uci),
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
                if opts.next() != Some("name") {
                    return Err(ParseCommandError::MissingNameKeyword);
                }
                match opts.next() {
                    Some("Hash") => {
                        if opts.next() != Some("value") {
                            return Err(ParseCommandError::MissingValueKeyword("Hash".into()));
                        }
                        let value = parse_next_option(&mut opts)?;
                        Ok(Self::SetOption(UciOption::Hash(value)))
                    }
                    Some("Threads") => {
                        if opts.next() != Some("value") {
                            return Err(ParseCommandError::MissingValueKeyword("Threads".into()));
                        }
                        let value = parse_next_option(&mut opts)?;
                        Ok(Self::SetOption(UciOption::Threads(value)))
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
                    "startpos" => STARTING_FEN.into(),
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

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Command::Uci => write!(f, "uci"),
            Command::Debug(on) => write!(f, "debug {}", if *on { "on" } else { "off" }),
            Command::IsReady => write!(f, "isready"),
            Command::SetOption(opt) => write!(f, "setoption name {opt}"),
            Command::UciNewGame => write!(f, "ucinewgame"),
            Command::Position { fen, moves } => {
                if fen == STARTING_FEN {
                    write!(f, "position startpos")?;
                } else {
                    write!(f, "position fen {fen}")?;
                }
                if !moves.is_empty() {
                    write!(f, " moves")?;
                    for mv in moves {
                        write!(f, " {mv}")?;
                    }
                }
                Ok(())
            }
            Command::Go(opts) => {
                write!(f, "go")?;
                for opt in opts {
                    write!(f, " {opt}")?;
                }
                Ok(())
            }
            Command::Stop => write!(f, "stop"),
            Command::PonderHit => write!(f, "ponderhit"),
            Command::Quit => write!(f, "quit"),
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
    Depth(i8),
    Nodes(u64),
    Mate(u32),
    MoveTime(Duration),
    Infinite,
}

impl Display for GoOption {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            GoOption::SearchMoves(moves) => {
                write!(f, "searchmoves")?;
                for mv in moves {
                    write!(f, " {mv}")?;
                }
                Ok(())
            }
            GoOption::Ponder => write!(f, "ponder"),
            GoOption::WTime(value) => write!(f, "wtime {value}"),
            GoOption::BTime(value) => write!(f, "btime {value}"),
            GoOption::WInc(value) => write!(f, "winc {value}"),
            GoOption::BInc(value) => write!(f, "binc {value}"),
            GoOption::MovesToGo(value) => write!(f, "movestogo {value}"),
            GoOption::Depth(value) => write!(f, "depth {value}"),
            GoOption::Nodes(value) => write!(f, "nodes {value}"),
            GoOption::Mate(value) => write!(f, "mate {value}"),
            GoOption::MoveTime(value) => write!(f, "movetime {}", value.as_millis()),
            GoOption::Infinite => write!(f, "infinite"),
        }
    }
}
