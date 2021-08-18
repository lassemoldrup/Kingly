use std::num::ParseIntError;
use std::str::FromStr;

use itertools::Itertools;

use crusty::framework::fen::STARTING_FEN;
use crusty::framework::io::Input;

use crate::uci::{GoOption, PseudoMove, UciOption};

use super::Command;

pub struct Parser<I> {
    input: I,
}

impl<I: Input> Parser<I> {
    pub(in crate::uci) fn new(input: I) -> Self {
        Self {
            input,
        }
    }

    pub(in crate::uci) fn parse(&mut self) -> Result<Command, String> {
        let cmd = self.input.read_line().unwrap();
        let cmds: Vec<_> = cmd.trim()
            .split_ascii_whitespace()
            .collect();

        match cmds[0] {
            "debug" => get_arg(&cmds, 1)
                .and_then(|arg| match arg {
                    "on" => Ok(Command::Debug(true)),
                    "off" => Ok(Command::Debug(false)),
                    _ => Err(format!("Invalid argument '{}'", arg)),
                }),

            "isready" => Ok(Command::IsReady),

            "setoption" => get_arg(&cmds, 1)
                .and_then(|arg| match arg {
                    "name" => cmds[2..].split(|&c| c == "value")
                        .map(|x| x.join(" "))
                        .collect_tuple::<(_, _)>()
                        .map(|(_, _)| Ok(Command::SetOption(UciOption::None)))
                        .unwrap_or(Ok(Command::SetOption(UciOption::None))),
                    _ => Err(format!("Invalid argument '{}'", arg)),
                }),

            "register" => get_arg(&cmds, 1)
                .and_then(|arg| match arg {
                    "later" => Ok(Command::RegisterLater),
                    "name" => cmds[2..].split(|&c| c == "code")
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
                    moves = move_strs.iter()
                        .map(|&mv| PseudoMove::from_str(mv))
                        .collect::<Result<Vec<_>, String>>()?;
                }

                Ok(Command::Position {
                    fen, moves
                })
            },

            "go" => {
                let mut options = vec![];
                let mut remaining = &cmds[1..];

                while !remaining.is_empty() {
                    let (opt, rem) = parse_go_option(remaining)?;
                    options.push(opt);
                    remaining = rem;
                }

                Ok(Command::Go(options))
            },

            "stop" => Ok(Command::Stop),

            "ponderhit" => Ok(Command::PonderHit),

            "quit" => Ok(Command::Quit),

            "" => Err("Missing command".to_string()),

            _ => Err(format!("Unrecognized command '{}'", cmds[0])),
        }
    }
}

fn get_arg<'a>(cmds: &[&'a str], idx: usize) -> Result<&'a str, String> {
    cmds.get(idx)
        .ok_or_else(|| "Missing argument".to_string())
        .map(|&arg| arg)
}

fn parse_go_option<'a, 'b>(opts: &'a [&'b str]) -> Result<(GoOption, &'a [&'b str]), String> {
    match opts {
        ["searchmoves", end @ ..] => todo!(),
        ["ponder", end @ ..] => Ok((GoOption::Ponder, end)),
        ["wtime", time, end @ ..] => {
            let time = time.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::WTime(time), end))
        },
        ["btime", time, end @ ..] => {
            let time = time.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::BTime(time), end))
        },
        ["winc", time, end @ ..] => {
            let time = time.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::WInc(time), end))
        },
        ["binc", time, end @ ..] => {
            let time = time.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::BInc(time), end))
        },
        ["movestogo", num_moves, end @ ..] => {
            let num_moves = num_moves.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::MovesToGo(num_moves), end))
        },
        ["depth", depth, end @ ..] => {
            let depth = depth.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::Depth(depth), end))
        },
        ["nodes", nodes, end @ ..] => {
            let nodes = nodes.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::Nodes(nodes), end))
        },
        ["mate", num_moves, end @ ..] => {
            let num_moves = num_moves.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::Mate(num_moves), end))
        },
        ["movetime", time, end @ ..] => {
            let time = time.parse()
                .map_err(|err: ParseIntError| err.to_string())?;
            Ok((GoOption::MoveTime(time), end))
        },
        ["infinite", end @ ..] => Ok((GoOption::Infinite, end)),
        [opt, ..] => Err(format!("Unrecognized option '{}'", opt)),
        [] => Err("Missing go option".to_string()),
    }
}