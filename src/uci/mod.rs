use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::num::ParseIntError;
use std::str::FromStr;
use std::time::Duration;

use crusty::fen::STARTING_FEN;
use crusty::search::SearchResult;
use itertools::Itertools;
use strum_macros::Display;

use crusty::client::Client;
use crusty::eval::StandardEval;
use crusty::types::{Move, PseudoMove, Value};

use crate::io::{Input, Output};

use self::writer::Writer;

#[cfg(test)]
mod tests;
mod writer;

pub struct Uci<I, O> {
    client: Client<StandardEval>,
    input: I,
    writer: Writer<O>,
    debug: bool,
}

impl<I, O> Uci<I, O>
where
    I: Input,
    O: Output,
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

    fn execute(&mut self, cmd: Command) -> io::Result<()> {
        match cmd {
            Command::Debug(_) => todo!(),
            Command::IsReady => todo!(),
            Command::SetOption(_) => todo!(),
            Command::Register { name, code } => todo!(),
            Command::RegisterLater => todo!(),
            Command::UciNewGame => todo!(),
            Command::Position { fen, moves } => todo!(),
            Command::Go(_) => todo!(),
            Command::Stop => todo!(),
            Command::PonderHit => todo!(),
            Command::Quit => todo!(),
            Command::Empty => todo!(),
        }
    }

    fn debug(&self, msg: &str) -> io::Result<()> {
        if self.debug {
            self.writer.debug(msg)?;
        }
        Ok(())
    }
}

fn search_result_to_info(result: &SearchResult) -> Vec<SearchInfo> {
    let mut info = Vec::with_capacity(7);

    info.push(SearchInfo::Depth(result.depth));
    info.push(SearchInfo::SelDepth(result.sel_depth));
    info.push(SearchInfo::Score(result.value));
    info.push(SearchInfo::Nodes(result.nodes_searched));
    let nps = result.nodes_searched as u128 * 1_000_000_000 / result.duration.as_nanos();
    info.push(SearchInfo::Nps(nps as u64));
    info.push(SearchInfo::HashFull(result.hash_full));
    info.push(SearchInfo::Pv(result.line.to_vec()));

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
    Empty,
}

#[derive(Display, Debug)]
enum UciOption {
    Hash(usize),
}

#[derive(PartialEq, Debug)]
enum GoOption {
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
enum SearchInfo {
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

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchInfo::Depth(depth) => write!(f, "depth {}", depth),
            SearchInfo::SelDepth(depth) => write!(f, "seldepth {}", depth),
            SearchInfo::Time(time) => write!(f, "time {}", time),
            SearchInfo::Nodes(nodes) => write!(f, "nodes {}", nodes),
            SearchInfo::Pv(pv) => write!(f, "pv {}", pv.iter().join(" ")),
            SearchInfo::Score(score) => write!(f, "score {}", score),
            SearchInfo::CurrMove(mv) => write!(f, "currmove {}", mv),
            SearchInfo::CurrMoveNumber(mv_number) => write!(f, "currmovenumber {}", mv_number),
            SearchInfo::HashFull(hash) => write!(f, "hashfull {}", hash),
            SearchInfo::Nps(nps) => write!(f, "nps {}", nps),
            SearchInfo::String(string) => write!(f, "string {}", string),
            SearchInfo::CurrLine { cpu_number, line } => {
                write!(f, "currline {} {}", cpu_number, line.iter().join(" "))
            }
        }
    }
}
