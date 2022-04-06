use std::fmt::{Debug, Display, Formatter};
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;
use std::{io, process};

use crusty::fen::STARTING_FEN;
use crusty::search::SearchResult;
use itertools::Itertools;
use strum_macros::Display;

use crusty::eval::StandardEval;
use crusty::types::{Move, PseudoMove, Value};

use crate::client::{Client, GoInfo};
use crate::io::{Input, Output};

#[cfg(test)]
mod tests;
mod writer;
pub use writer::Writer;

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
                Err(err) => self.debug(&err)?,
            }
        }
    }

    fn parse_command(cmd: &str) -> Result<Command, String> {
        let cmds: Vec<_> = cmd.split_ascii_whitespace().collect();
        match cmds[0] {
            "debug" => get_arg(&cmds, 1).and_then(|arg| match arg {
                "on" => Ok(Command::Debug(true)),
                "off" => Ok(Command::Debug(false)),
                _ => Err(format!("Invalid argument '{}'", arg)),
            }),

            "isready" => Ok(Command::IsReady),

            "setoption" => get_arg(&cmds, 1)
                .and_then(|arg| match arg {
                    "name" => cmds[2..]
                        .split(|&c| c == "value")
                        .map(|x| x.join(" "))
                        .collect_tuple::<(_, _)>()
                        .map(parse_uci_option)
                        .unwrap_or(Err("Invalid argument".into())),
                    _ => Err(format!("Invalid argument '{}'", arg)),
                })
                .map(|opt| Command::SetOption(opt)),

            "register" => get_arg(&cmds, 1).and_then(|arg| match arg {
                "later" => Ok(Command::RegisterLater),
                "name" => cmds[2..]
                    .split(|&c| c == "code")
                    .map(|x| x.join(" "))
                    .collect_tuple::<(_, _)>()
                    .ok_or_else(|| "Missing code".to_string())
                    .map(|(name, code)| Command::Register { name, code }),
                _ => Err(format!("Invalid argument(s) '{}'", cmds[1..].join(" "))),
            }),

            "ucinewgame" => Ok(Command::UciNewGame),

            "position" => {
                let mut sections = cmds[1..].split(|&c| c == "moves");
                let fen = match sections.next() {
                    Some(["fen", end @ ..]) if !end.is_empty() => Ok(end.join(" ")),
                    Some(["startpos", ..]) => Ok(STARTING_FEN.to_string()),
                    _ => Err(format!("Invalid argument(s) '{}'", cmds[1..].join(" "))),
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

            "" => Err("Missing command".to_string()),

            _ => Err(format!("Unrecognized command '{}'", cmds[0])),
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
                    GoInfo::NewDepth(res) => writer.info(&map_search_result(&res)).unwrap(),
                    GoInfo::BestMove(mv) => writer.best_move(mv).unwrap(),
                })
            }
            Command::Stop => self.client.stop(),
            Command::Quit => process::exit(0),
            _ => Ok(self.debug(&format!("Unsupported command '{:?}'", cmd))?),
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

fn map_search_result(result: &SearchResult) -> Vec<GoInfoPair> {
    let mut info = Vec::with_capacity(7);

    info.push(GoInfoPair::Depth(result.depth));
    info.push(GoInfoPair::SelDepth(result.sel_depth));
    info.push(GoInfoPair::Score(result.value));
    info.push(GoInfoPair::Nodes(result.nodes_searched));
    info.push(GoInfoPair::Nps(result.nps));
    info.push(GoInfoPair::HashFull(result.hash_full));
    info.push(GoInfoPair::Pv(result.line.to_vec()));
    info.push(GoInfoPair::Time(result.total_duration.as_millis() as u64));

    info
}

fn get_arg<'a>(cmds: &[&'a str], idx: usize) -> Result<&'a str, String> {
    cmds.get(idx)
        .ok_or_else(|| "Missing argument".to_string())
        .map(|&arg| arg)
}

fn parse_uci_option((name, value): (String, String)) -> Result<UciOption, String> {
    match name.as_str() {
        "Hash" => value
            .parse()
            .map(|v| UciOption::Hash(v))
            .map_err(|_| format!("Illegal value '{}'", value)),
        _ => Err(format!("Unrecognized option '{}'", name)),
    }
}

fn parse_go_option<'a, 'b>(opts: &'a [&'b str]) -> Result<(GoOption, &'a [&'b str]), String> {
    match opts {
        ["searchmoves", end @ ..] => {
            let moves: Vec<_> = end
                .iter()
                .map(|&opt| PseudoMove::from_str(opt).ok())
                .while_some()
                .collect();
            if moves.is_empty() {
                Err("searchmoves must be provided with at least 1 argument".to_string())
            } else {
                let num = moves.len();
                Ok((GoOption::SearchMoves(moves), &end[num..]))
            }
        }
        ["ponder", end @ ..] => Ok((GoOption::Ponder, end)),
        ["wtime", time, end @ ..] => {
            let time = time.parse().map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::WTime(time), end))
        }
        ["btime", time, end @ ..] => {
            let time = time.parse().map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::BTime(time), end))
        }
        ["winc", time, end @ ..] => {
            let time = time.parse().map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::WInc(time), end))
        }
        ["binc", time, end @ ..] => {
            let time = time.parse().map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::BInc(time), end))
        }
        ["movestogo", num_moves, end @ ..] => {
            let num_moves = num_moves
                .parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::MovesToGo(num_moves), end))
        }
        ["depth", depth, end @ ..] => {
            let depth = depth
                .parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::Depth(depth), end))
        }
        ["nodes", nodes, end @ ..] => {
            let nodes = nodes
                .parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::Nodes(nodes), end))
        }
        ["mate", num_moves, end @ ..] => {
            let num_moves = num_moves
                .parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::Mate(num_moves), end))
        }
        ["movetime", time, end @ ..] => {
            let time = time.parse().map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::MoveTime(Duration::from_millis(time)), end))
        }
        ["infinite", end @ ..] => Ok((GoOption::Infinite, end)),
        [_] => Err("Unrecognized option or missing argument".to_string()),
        [opt, ..] => Err(format!("Unrecognized option '{}'", opt)),
        [] => Err("Missing go option".to_string()),
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
            GoInfoPair::Depth(depth) => write!(f, "depth {}", depth),
            GoInfoPair::SelDepth(depth) => write!(f, "seldepth {}", depth),
            GoInfoPair::Time(time) => write!(f, "time {}", time),
            GoInfoPair::Nodes(nodes) => write!(f, "nodes {}", nodes),
            GoInfoPair::Pv(pv) => write!(f, "pv {}", pv.iter().join(" ")),
            GoInfoPair::Score(score) => write!(f, "score {}", score),
            GoInfoPair::CurrMove(mv) => write!(f, "currmove {}", mv),
            GoInfoPair::CurrMoveNumber(mv_number) => write!(f, "currmovenumber {}", mv_number),
            GoInfoPair::HashFull(hash) => write!(f, "hashfull {}", hash),
            GoInfoPair::Nps(nps) => write!(f, "nps {}", nps),
            GoInfoPair::String(string) => write!(f, "string {}", string),
            GoInfoPair::CurrLine { cpu_number, line } => {
                write!(f, "currline {} {}", cpu_number, line.iter().join(" "))
            }
        }
    }
}
