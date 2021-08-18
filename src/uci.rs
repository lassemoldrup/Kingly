use std::convert::AsRef;
use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::ops::Deref;
use std::process::exit;
use std::str::FromStr;
use std::sync::{Arc, mpsc, Mutex, MutexGuard, TryLockError};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Instant;

use itertools::Itertools;
use strum_macros::Display;

use crusty::framework::{Client, Searchable};
use crusty::framework::io::{Input, Output};
use crusty::framework::moves::Move;
use crusty::framework::piece::PieceKind;
use crusty::framework::search::{Search, SearchResult};
use crusty::framework::square::Square;
use crusty::framework::value::Value;
use parser::Parser;

use crate::uci::writer::Writer;

#[cfg(test)]
mod tests;
mod parser;
mod writer;

pub struct Uci<C, I, O> {
    client: Arc<Mutex<C>>,
    stop_search: Arc<AtomicBool>,
    parser: Parser<I>,
    writer: Arc<Mutex<Writer<O>>>,
    debug: bool,
}

impl<C, I, O> Uci<C, I, O>  where
    C: Client + Send + 'static,
    for<'a> &'a C: Searchable<'a>,
    I: Input,
    O: Output + Send + 'static
{
    pub fn new(client: C, input: I, output: O) -> Self {
        let client = client;

        Self {
            client: Arc::new(Mutex::new(client)),
            stop_search: Arc::new(AtomicBool::new(true)),
            parser: Parser::new(input),
            writer: Arc::new(Mutex::new(Writer::new(output))),
            debug: false,
        }
    }

    pub fn start(mut self) -> io::Result<()> {
        let mut writer = self.writer.lock().unwrap();

        writer.id()?;
        writer.uci_ok()?;
        drop(writer);

        self.client.lock().unwrap()
            .init();

        loop {
            match self.parser.parse() {
                Ok(cmd) => self.execute(cmd)?,
                Err(err) => self.debug(err)?,
            }
        }
    }

    fn execute(&mut self, cmd: Command) -> io::Result<()> {
        match cmd {
            Command::Debug(val) => self.debug = val,

            Command::IsReady => self.writer.lock().unwrap()
                .ready_ok()?,

            Command::SetOption(option) =>
                self.debug(format!("Unsupported option {}", option))?,

            Command::Register { .. } | Command::RegisterLater =>
                self.debug("Registration not supported")?,

            Command::UciNewGame => {},

            Command::Position { fen, moves } => self.client.try_lock()
                .map_err(|_| "Attempt to change position while searching".to_string())
                .and_then(|mut client| client.set_position(&fen).as_ref()
                    .map_err(ToString::to_string)
                    .and_then(|_| {
                        for mv in moves {
                            let mv = mv.into_move(&client.get_moves())?;
                            client.make_move(mv)?;
                        }
                        Ok(())
                    }))
                .or_else(|err| self.debug(err))?,

            Command::Go(options) => {
                if !options.contains(&GoOption::Infinite) {
                    panic!("Only infinite searching is currently supported")
                }

                if self.is_searching() {
                    return self.debug("Already searching");
                }

                let stop_search = self.stop_search.clone();
                self.stop_search.store(false, Ordering::SeqCst);

                let client = self.client.clone();
                let writer = self.writer.clone();
                thread::spawn(move || {
                    let client = client.lock().unwrap();
                    let mut search = client.deref().search();

                    let start = Instant::now();
                    search.on_info(|info| {
                        let mut info = search_result_to_info(info);
                        let elapsed = start.elapsed().as_millis() as u64;
                        info.push(SearchInfo::Time(elapsed));

                        writer.lock().unwrap().info(&info).unwrap();
                    });

                    search.start(stop_search);
                });
            },

            // TODO: Ordering ok?
            Command::Stop => if self.is_searching() {
                self.stop_search.store(true, Ordering::SeqCst);
            } else {
                self.debug("Attempt to stop with no search")?;
            },

            Command::PonderHit => self.debug("Pondering not supported")?,

            Command::Quit => exit(0),
        }
        Ok(())
    }

    fn debug(&self, msg: impl AsRef<str>) -> io::Result<()> {
        if self.debug {
            self.writer.lock().unwrap().debug(msg)?;
        }
        Ok(())
    }

    fn is_searching(&self) -> bool {
        !self.stop_search.load(Ordering::SeqCst)
    }
}

fn search_result_to_info(result: &SearchResult) -> Vec<SearchInfo> {
    let mut info = Vec::with_capacity(6);

    info.push(SearchInfo::Depth(result.depth()));
    info.push(SearchInfo::Score(result.value()));
    info.push(SearchInfo::Nodes(result.nodes_searched()));
    let nps = result.nodes_searched() as u128 * 1_000_000_000 / result.duration().as_nanos();
    info.push(SearchInfo::NPS(nps as u64));
    info.push(SearchInfo::PV(result.line().to_vec()));

    info
}

#[cfg(test)]
impl<C, I, O: Output + Clone> Uci<C, I, O> {
    fn get_output(&self) -> O {
        self.writer.lock().unwrap().clone().into_output()
    }
}


#[derive(Debug)]
enum Command {
    Debug(bool),
    IsReady,
    SetOption(UciOption),
    Register {
        name: String,
        code: String,
    },
    RegisterLater,
    UciNewGame,
    Position {
        fen: String,
        moves: Vec<PseudoMove>,
    },
    Go(Vec<GoOption>),
    Stop,
    PonderHit,
    Quit,
}


#[derive(Display, Debug)]
enum UciOption {
    None,
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
    Depth(u32),
    Nodes(u64),
    Mate(u32),
    MoveTime(u32),
    Infinite,
}


#[derive(Copy, Clone, PartialEq, Debug)]
struct PseudoMove {
    from: Square,
    to: Square,
    promotion: Option<PieceKind>,
}

impl PseudoMove {
    fn into_move(self, legal_moves: &[Move]) -> Result<Move, String> {
        legal_moves.iter().copied()
            .find(|&mv| mv.from() == self.from && mv.to() == self.to && match mv {
                Move::Promotion(_, _, kind) => Some(kind) == self.promotion,
                _ => true,
            })
            .ok_or_else(|| format!("Illegal move '{}'", self))
    }
}

impl From<Move> for PseudoMove {
    fn from(mv: Move) -> Self {
        Self {
            from: mv.from(),
            to: mv.to(),
            promotion: match mv {
                Move::Promotion(_, _, kind) => Some(kind),
                _ => None
            }
        }
    }
}

impl PartialEq<Move> for PseudoMove {
    fn eq(&self, other: &Move) -> bool {
        self.from == other.from()
            && self.to == other.to()
            && match other {
                Move::Promotion(_, _, kind) => self.promotion == Some(*kind),
                _ => true,
            }
    }
}

impl FromStr for PseudoMove {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 4 || s.len() == 5 {
            let from = Square::try_from(&s[0..2])?;
            let to = Square::try_from(&s[2..4])?;

            let promotion = if s.len() == 5 {
                Some(PieceKind::try_from(s.chars().nth(4).unwrap())?)
            } else {
                None
            };

            Ok(PseudoMove {
                from,
                to,
                promotion,
            })
        } else {
            Err(format!("Invalid move '{}'", s))
        }
    }
}

impl Display for PseudoMove {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}{}", self.from, self.to)
    }
}


#[derive(Debug)]
enum SearchInfo {
    Depth(u32),
    SelDepth(u32),
    Time(u64),
    Nodes(u64),
    PV(Vec<Move>),
    Score(Value),
    CurrMove(Move),
    CurrMoveNumber(u32),
    HashFull(u32),
    NPS(u64),
    String(String),
    CurrLine {
        cpu_number: u32,
        line: Vec<Move>,
    },
}

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchInfo::Depth(depth) => write!(f, "depth {}", depth),
            SearchInfo::SelDepth(depth) => write!(f, "seldepth {}", depth),
            SearchInfo::Time(time) => write!(f, "time {}", time),
            SearchInfo::Nodes(nodes) => write!(f, "nodes {}", nodes),
            SearchInfo::PV(pv) => write!(f, "pv {}", pv.iter().join(" ")),
            SearchInfo::Score(score) => write!(f, "score {}", score),
            SearchInfo::CurrMove(mv) => write!(f, "currmove {}", mv),
            SearchInfo::CurrMoveNumber(mv_number) => write!(f, "currmovenumber {}", mv_number),
            SearchInfo::HashFull(hash) => write!(f, "hashfull {}", hash),
            SearchInfo::NPS(nps) => write!(f, "nps {}", nps),
            SearchInfo::String(string) => write!(f, "string {}", string),
            SearchInfo::CurrLine { cpu_number, line } =>
                write!(f, "currline {} {}", cpu_number, line.iter().join(" ")),
        }
    }
}
