use std::error::Error;
use std::fmt::{self, Debug, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;
use std::{io, process};

use itertools::Itertools;
use kingly_lib::fen::STARTING_FEN;
use kingly_lib::search::SearchInfo;
use strum_macros::Display;

use kingly_lib::eval::StandardEval;
use kingly_lib::types::{Move, PseudoMove, Value};

use crate::client::{Client, GoInfo};
use crate::io::{Input, Output};

#[cfg(test)]
mod tests;
mod writer;
pub use writer::Writer;

#[derive(Debug)]
struct ParseCmdError(String);

impl Error for ParseCmdError {}

impl From<String> for ParseCmdError {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<ParseIntError> for ParseCmdError {
    fn from(err: ParseIntError) -> Self {
        Self(err.to_string())
    }
}

impl Display for ParseCmdError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "Failed to parse command: {}", self.0)
    }
}

macro_rules! parse_err {
    ($str:literal $(, $arg:expr)*) => {
        ParseCmdError(format!($str $(, $arg)*))
    };
}

pub struct Uci<I, O> {
    client: Client<StandardEval>,
    input: I,
    writer: Writer<O>,
    debug: bool,
}

impl<I, O> Uci<I, O>
where
    I: Input,
    O: Output + Send + 'static,
{
    pub fn new(client: Client<StandardEval>, input: I, output: O) -> Self {
        let writer = Writer::new(output);
        Self {
            client,
            input,
            writer,
            debug: false,
        }
    }

    pub fn start(mut self) -> io::Result<()> {
        self.writer.id()?;
        self.writer.options()?;
        self.writer.uci_ok()?;

        self.client.init();

        loop {
            let cmd = self.input.read_line()?;
            if cmd.trim().is_empty() {
                continue;
            }

            match Self::parse_command(&cmd) {
                Ok(cmd) => self.execute(cmd)?,
                Err(err) => self.debug(&err.0)?,
            }
        }
    }

    fn parse_command(cmd: &str) -> Result<Command, ParseCmdError> {
        let cmds: Vec<_> = cmd.split_ascii_whitespace().collect();
        match cmds[0] {
            "debug" => get_arg(&cmds, 1).and_then(|arg| match arg {
                "on" => Ok(Command::Debug(true)),
                "off" => Ok(Command::Debug(false)),
                _ => Err(parse_err!("Invalid argument '{arg}'")),
            }),

            "isready" => Ok(Command::IsReady),

            "setoption" => get_arg(&cmds, 1)
                .and_then(|arg| match arg {
                    "name" => cmds[2..]
                        .split(|&c| c == "value")
                        .map(|x| x.join(" "))
                        .collect_tuple::<(_, _)>()
                        .map(parse_uci_option)
                        .unwrap_or_else(|| Err(parse_err!("Invalid argument"))),
                    _ => Err(parse_err!("Invalid argument '{arg}'")),
                })
                .map(Command::SetOption),

            "register" => get_arg(&cmds, 1).and_then(|arg| match arg {
                "later" => Ok(Command::RegisterLater),
                "name" => cmds[2..]
                    .split(|&c| c == "code")
                    .map(|x| x.join(" "))
                    .collect_tuple::<(_, _)>()
                    .ok_or_else(|| parse_err!("Missing code"))
                    .map(|(name, code)| Command::Register { name, code }),
                _ => Err(parse_err!("Invalid argument(s) '{}'", cmds[1..].join(" "))),
            }),

            "ucinewgame" => Ok(Command::UciNewGame),

            "position" => {
                let mut sections = cmds[1..].split(|&c| c == "moves");
                let fen = match sections.next() {
                    Some(["fen", end @ ..]) if !end.is_empty() => Ok(end.join(" ")),
                    Some(["startpos", ..]) => Ok(STARTING_FEN.to_string()),
                    _ => Err(parse_err!("Invalid argument(s) '{}'", cmds[1..].join(" "))),
                }?;

                let mut moves = vec![];
                if let Some(move_strs) = sections.next() {
                    moves = move_strs
                        .iter()
                        .map(|&mv| PseudoMove::from_str(mv))
                        .collect::<Result<Vec<_>, String>>()?;
                }

                Ok(Command::Position { fen, moves })
            }

            "go" => {
                let mut options = vec![];
                let mut remaining = &cmds[1..];

                while !remaining.is_empty() {
                    let (opt, rem) = parse_go_option(remaining)?;
                    options.push(opt);
                    remaining = rem;
                }

                Ok(Command::Go(options))
            }

            "stop" => Ok(Command::Stop),

            "ponderhit" => Ok(Command::PonderHit),

            "quit" => Ok(Command::Quit),

            "" => Err(parse_err!("Missing command")),

            _ => Err(parse_err!("Unrecognized command '{}'", cmds[0])),
        }
    }

    fn set_option(&mut self, opt: UciOption) -> Result<(), String> {
        match opt {
            UciOption::Hash(hash_size) => self.client.set_option_hash(hash_size),
        }
    }

    fn execute(&mut self, cmd: Command) -> io::Result<()> {
        match cmd {
            Command::Debug(debug) => Ok(self.debug = debug),
            Command::IsReady => Ok(self.writer.ready_ok()?),
            Command::SetOption(opt) => self.set_option(opt),
            Command::UciNewGame => self.client.uci_new_game(),
            Command::Position { fen, moves } => self.client.position(&fen, &moves),
            Command::Go(opts) => {
                let writer = self.writer.clone();
                // TODO: Don't unwrap
                self.client.go(opts, move |info| match info {
                    GoInfo::NewDepth(res) => writer.info(&map_search_result(res)).unwrap(),
                    GoInfo::BestMove(mv) => writer.best_move(mv).unwrap(),
                })
            }
            Command::Stop => self.client.stop(),
            Command::Quit => process::exit(0),
            _ => Ok(self.debug(&format!("Unsupported command '{cmd:?}'"))?),
        }
        .or_else(|err| self.debug(&err))
    }

    fn debug(&self, msg: &str) -> io::Result<()> {
        if self.debug {
            self.writer.debug(msg)?;
        }
        Ok(())
    }
}

fn map_search_result(result: &SearchInfo) -> Vec<GoInfoPair> {
    let info = vec![
        GoInfoPair::Depth(result.depth),
        GoInfoPair::SelDepth(result.sel_depth),
        GoInfoPair::Score(result.score),
        GoInfoPair::Nodes(result.nodes_searched),
        GoInfoPair::Nps(result.nps),
        GoInfoPair::HashFull(result.hash_full),
        GoInfoPair::Pv(result.pv.to_vec()),
        GoInfoPair::Time(result.total_duration.as_millis() as u64),
    ];

    info
}

fn get_arg<'a>(cmds: &[&'a str], idx: usize) -> Result<&'a str, ParseCmdError> {
    cmds.get(idx)
        .ok_or_else(|| parse_err!("Missing argument"))
        .map(|&arg| arg)
}

fn parse_uci_option((name, value): (String, String)) -> Result<UciOption, ParseCmdError> {
    match name.as_str() {
        "Hash" => value
            .parse()
            .map(UciOption::Hash)
            .map_err(|_| parse_err!("Illegal value '{value}'")),
        _ => Err(parse_err!("Unrecognized option '{name}'")),
    }
}

fn parse_go_option<'a, 'b>(
    opts: &'a [&'b str],
) -> Result<(GoOption, &'a [&'b str]), ParseCmdError> {
    match opts {
        ["searchmoves", end @ ..] => {
            let moves: Vec<_> = end
                .iter()
                .map(|&opt| PseudoMove::from_str(opt).ok())
                .while_some()
                .collect();

            if moves.is_empty() {
                Err(parse_err!(
                    "searchmoves must be provided with at least 1 move"
                ))
            } else {
                let num = moves.len();
                Ok((GoOption::SearchMoves(moves), &end[num..]))
            }
        }
        ["ponder", end @ ..] => Ok((GoOption::Ponder, end)),
        ["wtime", time, end @ ..] => {
            let time = time.parse::<u32>()?;
            Ok((GoOption::WTime(time), end))
        }
        ["btime", time, end @ ..] => {
            let time = time.parse::<u32>()?;
            Ok((GoOption::BTime(time), end))
        }
        ["winc", time, end @ ..] => {
            let time = time.parse::<u32>()?;
            Ok((GoOption::WInc(time), end))
        }
        ["binc", time, end @ ..] => {
            let time = time.parse::<u32>()?;
            Ok((GoOption::BInc(time), end))
        }
        ["movestogo", num_moves, end @ ..] => {
            let num_moves = num_moves.parse::<u32>()?;
            Ok((GoOption::MovesToGo(num_moves), end))
        }
        ["depth", depth, end @ ..] => {
            let depth = depth.parse::<u8>()?;
            Ok((GoOption::Depth(depth), end))
        }
        ["nodes", nodes, end @ ..] => {
            let nodes = nodes.parse::<u64>()?;
            Ok((GoOption::Nodes(nodes), end))
        }
        ["mate", num_moves, end @ ..] => {
            let num_moves = num_moves.parse::<u32>()?;
            Ok((GoOption::Mate(num_moves), end))
        }
        ["movetime", time, end @ ..] => {
            let time = time.parse::<u64>()?;
            Ok((GoOption::MoveTime(Duration::from_millis(time)), end))
        }
        ["infinite", end @ ..] => Ok((GoOption::Infinite, end)),
        [_] => Err(parse_err!("Unrecognized option or missing argument")),
        [opt, ..] => Err(parse_err!("Unrecognized option '{opt}'")),
        [] => Err(parse_err!("Missing go option")),
    }
}

#[allow(dead_code)]
#[derive(Debug)]
enum Command {
    Debug(bool),
    IsReady,
    SetOption(UciOption),
    Register { name: String, code: String },
    RegisterLater,
    UciNewGame,
    Position { fen: String, moves: Vec<PseudoMove> },
    Go(Vec<GoOption>),
    Stop,
    PonderHit,
    Quit,
}

#[derive(Display, Debug)]
enum UciOption {
    Hash(usize),
}

#[derive(PartialEq, Debug)]
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

#[allow(dead_code)]
#[derive(Debug)]
enum GoInfoPair {
    Depth(u8),
    SelDepth(u8),
    Time(u64),
    Nodes(u64),
    Pv(Vec<Move>),
    Score(Value),
    CurrMove(Move),
    CurrMoveNumber(u32),
    HashFull(u32),
    Nps(u64),
    String(String),
    CurrLine { cpu_number: u32, line: Vec<Move> },
}

impl Display for GoInfoPair {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            GoInfoPair::Depth(depth) => write!(f, "depth {depth}"),
            GoInfoPair::SelDepth(depth) => write!(f, "seldepth {depth}"),
            GoInfoPair::Time(time) => write!(f, "time {time}"),
            GoInfoPair::Nodes(nodes) => write!(f, "nodes {nodes}"),
            GoInfoPair::Pv(pv) => write!(f, "pv {}", pv.iter().join(" ")),
            GoInfoPair::Score(score) => write!(f, "score {score}"),
            GoInfoPair::CurrMove(mv) => write!(f, "currmove {mv}"),
            GoInfoPair::CurrMoveNumber(mv_number) => write!(f, "currmovenumber {mv_number}"),
            GoInfoPair::HashFull(hash) => write!(f, "hashfull {hash}"),
            GoInfoPair::Nps(nps) => write!(f, "nps {nps}"),
            GoInfoPair::String(string) => write!(f, "string {string}"),
            GoInfoPair::CurrLine { cpu_number, line } => {
                write!(f, "currline {cpu_number} {}", line.iter().join(" "))
            }
        }
    }
}
