use crusty::framework::Game;
use std::io::{BufRead, Write};
use crusty::framework::fen::{STARTING_FEN, FenParseError};
use std::process::exit;
use crusty::framework::moves::Move;
use crusty::framework::util::{get_king_sq, get_castling_sq};
use crusty::framework::piece::PieceKind;
use std::convert::TryFrom;
use crusty::framework::square::Square;
use std::time::Instant;
use std::sync::{Arc, Mutex, MutexGuard, TryLockError};
use std::thread;
use std::thread::JoinHandle;

pub struct Uci<'a, G, I, O> {
    game: Arc<Mutex<G>>,
    input: &'a mut I,
    output: &'a mut O,
    debug: bool,
    think: Option<JoinHandle<()>>,
}

impl<'a, G: Game, I: BufRead + 'a, O: Write + 'a> Uci<'a, G, I, O> {
    pub fn new(game: G, input: &'a mut I, output: &'a mut O) -> Self {
        Self {
            game: Arc::new(Mutex::new(game)),
            input,
            output,
            debug: false,
            think: None,
        }
    }

    pub fn start(mut self) -> std::io::Result<()> {
        self.id()?;
        self.uci_ok()?;

        loop {
            let mut command = String::new();
            self.input.read_line(&mut command)?;

            match self.parse_command(&command.trim()) {
                Ok(cmd) => self.execute(cmd)?,
                Err(_) => if self.debug {
                    writeln!(self.output, "Failed to parse command '{:?}'", command.trim())?
                },
            }
        }
    }

    fn parse_command(&mut self, cmd: &str) -> Result<Command, ()> {
        let cmd = cmd.to_ascii_lowercase();
        let command_args: Vec<&str> = cmd.split_ascii_whitespace()
            .collect();

        match *command_args.get(0).unwrap_or(&"") {
            "debug" => match *command_args.get(1).ok_or(())? {
                "on" => Ok(Command::Debug(true)),
                "off" => Ok(Command::Debug(false)),
                _ => Err(()),
            },

            "isready" => Ok(Command::IsReady),

            "setoption" => Ok(Command::SetOption(UciOption::None)),

            "register" => {
                let mut name = None;
                let mut code = None;

                let mut args = command_args[1..].iter();
                loop {
                    let arg = match args.next() {
                        None => break,
                        Some(a) => a,
                    };
                    match *arg {
                        "later" => return Ok(Command::Register {
                            later: true,
                            name: None,
                            code: None,
                        }),
                        "name" => name = args.next().map(|a| a.to_string()),
                        "code" => code = args.next().map(|a| a.to_string()),
                        _ => return Err(()),
                    }
                }

                Ok(Command::Register {
                    later: false,
                    name,
                    code,
                })
            },

            "ucinewgame" => Ok(Command::UciNewGame),

            "position" => match self.game.try_lock() {
                Ok(ref mut game) => match *command_args.get(1).ok_or(())? {
                    "startpos" => match command_args.get(2) {
                        Some(&"moves") => Ok(Command::Position {
                            fen: STARTING_FEN.to_string(),
                            moves: Self::parse_move_list(game, &command_args[3..])?,
                        }),
                        Some(_) => Err(()),
                        None => Ok(Command::Position {
                            fen: STARTING_FEN.to_string(),
                            moves: Vec::new(),
                        }),
                    },
                    "fen" => {
                        let fen_vec = command_args[2..].iter()
                            .take_while(|&&arg| arg != "moves")
                            .map(|s| *s)
                            .collect::<Vec<_>>();
                        let fen = fen_vec.join(" ");

                        match command_args.get(2 + fen_vec.len()) {
                            Some(&"moves") => Ok(Command::Position {
                                fen,
                                moves: Self::parse_move_list(game, &command_args[3 + fen_vec.len()..])?,
                            }),
                            Some(_) => Err(()),
                            None => Ok(Command::Position {
                                fen,
                                moves: Vec::new(),
                            }),
                        }
                    },
                    _ => Err(())
                },
                Err(_) => Err(()),
            },

            "go" => match self.game.try_lock() {
                Ok(ref game) => {
                    let mut opts = Vec::new();

                    let mut i = 1;
                    while i < command_args.len() {
                        match command_args[i] {
                            "searchmoves" => {
                                let mut moves = Vec::new();

                                while let Some(arg) = command_args.get(i + 1) {
                                    if Self::is_go_command(arg) {
                                        break;
                                    }

                                    moves.push(Self::parse_move(game, arg)?);
                                    i += 1;
                                }

                                opts.push(GoOption::SearchMoves(moves))
                            },
                            "ponder" => opts.push(GoOption::Ponder),
                            "wtime" =>
                                opts.push(GoOption::WTime(Self::parse_num_arg(&command_args, &mut i)?)),
                            "btime" =>
                                opts.push(GoOption::BTime(Self::parse_num_arg(&command_args, &mut i)?)),
                            "winc" =>
                                opts.push(GoOption::WInc(Self::parse_num_arg(&command_args, &mut i)?)),
                            "binc" =>
                                opts.push(GoOption::BInc(Self::parse_num_arg(&command_args, &mut i)?)),
                            "movestogo" =>
                                opts.push(GoOption::MovesToGo(Self::parse_num_arg(&command_args, &mut i)?)),
                            "depth" =>
                                opts.push(GoOption::Depth(Self::parse_num_arg(&command_args, &mut i)?)),
                            "nodes" =>
                                opts.push(GoOption::Nodes(Self::parse_num_arg(&command_args, &mut i)?)),
                            "mate" =>
                                opts.push(GoOption::Mate(Self::parse_num_arg(&command_args, &mut i)?)),
                            "movetime" =>
                                opts.push(GoOption::MoveTime(Self::parse_num_arg(&command_args, &mut i)?)),
                            "infinite" => opts.push(GoOption::Infinite),
                            _ => return Err(()),
                        }

                        i += 1;
                    }

                    Ok(Command::Go(opts))
                },
                Err(_) => Err(()),
            },

            "stop" => Ok(Command::Stop),

            "ponderhit" => Ok(Command::PonderHit),

            "quit" => Ok(Command::Quit),

            "" => Err(()),

            _ => self.parse_command(&command_args[1..].join(" ")),
        }
    }

    fn is_go_command(cmd: &str) -> bool {
        let cmds = ["searchmoves", "ponder", "wtime", "btime", "winc", "binc", "movestogo", "depth",
            "nodes", "mate", "movetime", "infinite"];

        cmds.contains(&cmd)
    }

    fn parse_move(game: &G, mv_str: &str) -> Result<Move, ()> {
        let to_move = game.to_move();
        let mut available_moves = game.get_moves().into_iter().map(|mv| match mv {
            Move::Regular(from, to) |
            Move::Promotion(from, to, _) |
            Move::EnPassant(from, to) => (from, to, mv),
            Move::Castling(side) => (get_king_sq(to_move), get_castling_sq(to_move, side), mv),
        });

        if mv_str.len() == 4 {
            let from = Square::try_from(&mv_str[..2]).map_err(|_| ())?;
            let to = Square::try_from(&mv_str[2..]).map_err(|_| ())?;

            available_moves.find(|(f, t, _)| *f == from && *t == to)
                .ok_or(()).map(|(_, _, mv)| mv)
        } else if mv_str.len() == 5 {
            let from = Square::try_from(&mv_str[..2]).map_err(|_| ())?;
            let to = Square::try_from(&mv_str[2..4]).map_err(|_| ())?;
            let kind = PieceKind::try_from(mv_str.chars().last().unwrap()).map_err(|_| ())?;
            let mv = Move::Promotion(from, to, kind);

            available_moves.find(|(_, _, m)| *m == mv)
                .ok_or(()).map(|(_, _, mv)| mv)
        } else {
            Err(())
        }
    }

    fn parse_move_list(game: &mut G, moves: &[&str]) -> Result<Vec<Move>, ()> {
        let mut parsed = Vec::new();

        for mv in moves {
            match Self::parse_move(game, mv) {
                Ok(mv) => {
                    parsed.push(mv);
                    game.make_move(mv).unwrap();
                },
                Err(_) => break,
            }
        }

        for _ in 0..parsed.len() {
            game.unmake_move().unwrap();
        }

        if parsed.len() == moves.len() {
            Ok(parsed)
        } else {
            Err(())
        }
    }

    fn parse_num_arg<T: std::str::FromStr>(command_args: &[&str], i: &mut usize) -> Result<T, ()> {
        *i += 1;
        command_args.get(*i)
            .ok_or(())?
            .parse()
            .map_err(|_| ())
    }

    fn execute(&mut self, cmd: Command) -> std::io::Result<()> {
        if self.debug {
            writeln!(self.output, "Executing command '{:?}'", cmd)?;
        }

        match cmd {
            Command::Debug(val) => {
                self.debug = val;
                writeln!(self.output, "Debug is now {}", match val {
                    true => "on",
                    false => "off",
                })?;
            },
            Command::IsReady => self.ready_ok()?,
            Command::UciNewGame => match self.game.try_lock() {
                Ok(ref mut game) => game.set_position(STARTING_FEN).unwrap(),
                Err(_) => if self.debug {
                    writeln!(self.output, "Can't start a new game while thinking")?;
                }
            }
            Command::Position { fen, moves } => match self.game.try_lock() {
                Ok(ref mut game) => match game.set_position(&fen) {
                    Ok(_) => {
                        for mv in moves {
                            game.make_move(mv).unwrap();
                        }
                    }
                    Err(err) => if self.debug {
                        writeln!(self.output, "{}", err)?;
                    }
                }
                Err(_) => if self.debug {
                    writeln!(self.output, "Can't execute position command while thinking")?;
                }
            }
            Command::Go(opts) => match self.game.try_lock() {
                Ok(game) => {
                    let think_start = Instant::now();

                    let mut search_moves = None;
                    let mut max_depth = None;
                    let mut max_nodes = None;
                    let mut search_time = None;
                    let mut find_mate = None;

                    for opt in opts {
                        match opt {
                            GoOption::SearchMoves(moves) => search_moves = Some(moves),
                            GoOption::Depth(depth) => max_depth = Some(depth),
                            GoOption::Nodes(nodes) => max_nodes = Some(nodes),
                            GoOption::Mate(mate) => find_mate = Some(mate),
                            GoOption::MoveTime(time) => search_time = Some(time),
                            GoOption::Infinite => max_depth = None,
                            _ => if self.debug {
                                writeln!(self.output, "Unsupported go option {:?}", opt)?;
                            }
                        }
                    }

                    self.start_think();
                }
                Err(_) => if self.debug {
                    writeln!(self.output, "Can't execute position command while thinking")?;
                }
            },
            Command::Quit => exit(0),
            _ => if self.debug {
                writeln!(self.output, "Unsupported command")?;
            },
        }

        Ok(())
    }

    fn start_think(&self) -> std::io::Result<()> {
        let game = Arc::clone(&self.game);
        self.think = Some(thread::spawn(move || {
            game.lock().unwrap().search(10);
        }));
        Ok(())
    }

    fn id(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "id name Crusty")?;
        writeln!(self.output, "id author Lasse Møldrup")
    }

    fn uci_ok(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "uciok")
    }

    fn ready_ok(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "readyok")
    }
}


#[derive(Debug)]
enum Command {
    Debug(bool),
    IsReady,
    SetOption(UciOption),
    Register {
        later: bool,
        name: Option<String>,
        code: Option<String>,
    },
    UciNewGame,
    Position {
        fen: String,
        moves: Vec<Move>,
    },
    Go(Vec<GoOption>),
    Stop,
    PonderHit,
    Quit,
}


#[derive(Debug)]
enum UciOption {
    None,
}


#[derive(Debug)]
enum GoOption {
    SearchMoves(Vec<Move>),
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
