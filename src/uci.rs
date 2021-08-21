use std::convert::AsRef;
use std::fmt::{Debug, Display, Formatter};
use std::io;
use std::mem::swap;
use std::ops::Deref;
use std::process::exit;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Instant;

use itertools::Itertools;
use strum_macros::Display;

use crusty::framework::{Client, Searchable};
use crusty::framework::io::{Input, Output};
use crusty::framework::moves::{Move, PseudoMove};
use crusty::framework::search::{Search, SearchResult};
use crusty::framework::value::Value;
use parser::Parser;

use crate::uci::writer::Writer;

#[cfg(test)]
mod tests;
mod parser;
mod writer;

pub struct Uci<C: 'static, I, O: 'static> {
    client: &'static Mutex<C>,
    stop_search: &'static AtomicBool,
    search_thread: Option<JoinHandle<()>>,
    parser: Parser<I>,
    writer: &'static Mutex<Writer<O>>,
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
            client: Box::leak(Box::new(Mutex::new(client))),
            stop_search: Box::leak(Box::new(AtomicBool::new(true))),
            search_thread: None,
            parser: Parser::new(input),
            writer: Box::leak(Box::new(Mutex::new(Writer::new(output)))),
            debug: false,
        }
    }

    pub fn start(mut self) -> io::Result<()> {
        let mut writer = self.writer.lock().unwrap();

        writer.id()?;
        writer.uci_ok()?;
        // Unlock the writer mutex
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

    fn wait_for_search(&mut self) {
        let mut search_thread = None;
        swap(&mut self.search_thread, &mut search_thread);

        if let Some(handle) = search_thread {
            handle.join().unwrap();
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
                if self.is_searching() {
                    return self.debug("Already searching");
                }
                self.wait_for_search();

                self.stop_search.store(false, Ordering::SeqCst);

                let stop_search = self.stop_search;
                let client = self.client;
                let writer = self.writer;
                self.search_thread = Some(thread::spawn(move || {
                    let client = client.lock().unwrap();

                    let start = Instant::now();
                    let mut search = client.deref().search();

                    for option in options {
                        match option {
                            // TODO: How to test this?
                            GoOption::SearchMoves(moves) => search = search.moves(&moves),
                            GoOption::Depth(depth) => search = search.depth(depth),
                            _ => { },
                        }
                    }

                    search.on_info(|info| {
                        let mut info = search_result_to_info(info);
                        let elapsed = start.elapsed().as_millis() as u64;
                        info.push(SearchInfo::Time(elapsed));

                        writer.lock().unwrap()
                            .info(&info)
                            .unwrap();
                    })
                        .start(stop_search);

                    stop_search.store(true, Ordering::Release);
                }));
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
    info.push(SearchInfo::Nps(nps as u64));
    info.push(SearchInfo::Pv(result.line().to_vec()));

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

#[allow(dead_code)]
#[derive(Debug)]
enum SearchInfo {
    Depth(u32),
    SelDepth(u32),
    Time(u64),
    Nodes(u64),
    Pv(Vec<Move>),
    Score(Value),
    CurrMove(Move),
    CurrMoveNumber(u32),
    HashFull(u32),
    Nps(u64),
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
            SearchInfo::Pv(pv) => write!(f, "pv {}", pv.iter().join(" ")),
            SearchInfo::Score(score) => write!(f, "score {}", score),
            SearchInfo::CurrMove(mv) => write!(f, "currmove {}", mv),
            SearchInfo::CurrMoveNumber(mv_number) => write!(f, "currmovenumber {}", mv_number),
            SearchInfo::HashFull(hash) => write!(f, "hashfull {}", hash),
            SearchInfo::Nps(nps) => write!(f, "nps {}", nps),
            SearchInfo::String(string) => write!(f, "string {}", string),
            SearchInfo::CurrLine { cpu_number, line } =>
                write!(f, "currline {} {}", cpu_number, line.iter().join(" ")),
        }
    }
}
