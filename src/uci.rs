use std::io::{BufRead, Write, self};
use crusty::framework::fen::{STARTING_FEN, FenParseError};
use std::process::exit;
use crusty::framework::moves::Move;
use crusty::framework::Client;
use crusty::framework::square::Square;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::sync::mpsc::{Receiver, channel};
use parser::Parser;
use std::fmt::{Debug, Display, Formatter};
use crusty::framework::io::{Input, Output};
use crate::uci::writer::Writer;
use std::ops::{Deref, DerefMut};
use crusty::framework::piece::PieceKind;
use std::str::FromStr;
use std::convert::TryFrom;
use crusty::framework::search::Search;
use crusty::framework::value::Value;
use std::convert::AsRef;
use strum_macros::AsRefStr;


#[cfg(test)]
mod tests;
mod parser;
mod writer;

pub struct Uci<C, I, O> {
    client: Arc<Mutex<C>>,
    search_stream: Option<Receiver<Move>>,
    parser: Parser<I>,
    writer: Arc<Mutex<Writer<O>>>,
    debug: bool,
}

impl<'a, C, I, O> Uci<C, I, O>  where
    C: Client<'a> + Send + 'static,
    I: Input,
    O: Output
{
    pub fn new(client: C, input: I, output: O) -> Self {
        let client = client;

        Self {
            client: Arc::new(Mutex::new(client)),
            search_stream: None,
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

            Command::SetOption(_) => {},

            Command::Register { .. } => {},

            Command::RegisterLater => {},

            Command::UciNewGame => {},

            Command::Position { fen, moves } => self.client.try_lock()
                .map_err(|_| "Attempt to change position while searching".to_string())
                .and_then(|mut client| client.set_position(&fen).as_ref()
                    .map_err(ToString::to_string)
                    .and_then(|_| {
                        moves.iter()
                            .find_map(|&mv| mv.into_move(&client.get_moves())
                                .and_then(|mv| client.make_move(mv))
                                .err())
                            .map_or(Ok(()), Err)
                    }))
                .or_else(|err| self.debug(err))?,

            Command::Go(options) => {
                if !options.contains(&GoOption::Infinite) {
                    panic!("Only infinite searching is currently supported")
                }

                let client = self.client.clone();
                let writer = self.writer.clone();
                thread::spawn(move || {
                    let test = client.lock().unwrap();
                    let mut search = test.search();

                    search.on_info(|info| {

                    });

                    drop(search);
                    drop(test);
                });
            },

            Command::Stop => {},

            Command::PonderHit => {},

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
        moves: Box<[PseudoMove]>,
    },
    Go(Box<[GoOption]>),
    Stop,
    PonderHit,
    Quit,
}


#[derive(Debug)]
enum UciOption {
    None,
}

#[derive(PartialEq, Debug)]
enum GoOption {
    SearchMoves(Box<[PseudoMove]>),
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
    kind: Option<PieceKind>,
}

impl PseudoMove {
    fn into_move(self, legal_moves: &[Move]) -> Result<Move, String> {
        legal_moves.iter().copied()
            .find(|&mv| mv.from() == self.from && mv.to() == self.to && match mv {
                Move::Promotion(_, _, kind) => Some(kind) == self.kind,
                _ => true,
            })
            .ok_or_else(|| format!("Illegal move '{}'", self))
    }
}

impl FromStr for PseudoMove {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() == 4 || s.len() == 5 {
            let from = Square::try_from(&s[0..2])?;
            let to = Square::try_from(&s[2..4])?;

            let kind = if s.len() == 5 {
                Some(PieceKind::try_from(s.chars().nth(4).unwrap())?)
            } else {
                None
            };

            Ok(PseudoMove {
                from, to, kind
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


#[derive(AsRefStr, Debug)]
enum SearchInfo {
    Depth(u32),
    SelDepth(u32),
    Time(u32),
    Nodes(u64),
    PV(Box<[Move]>),
    Score(Value),
    CurrMove(Move),
    CurrMoveNumber(u32),
    HashFull(u32),
    NPS(u64),
    String(String),
    CurrLine {
        cpu_number: u32,
        line: Box<[Move]>,
    },
}

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref().to_ascii_lowercase())
    }
}