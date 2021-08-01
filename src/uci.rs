use std::io::{BufRead, Write};
use crusty::framework::fen::STARTING_FEN;
use std::process::exit;
use crusty::framework::moves::Move;
use crusty::framework::Client;
use crusty::framework::square::Square;
use std::time::Instant;
use std::sync::{Arc, Mutex, mpsc};
use std::thread;
use std::sync::mpsc::Receiver;

pub struct Uci<C: 'static, I, O: 'static> {
    client: Arc<Mutex<&'static mut C>>,
    input: I,
    output: Arc<Mutex<&'static mut O>>,
    debug: bool,
    think: Option<Receiver<Move>>,
}

impl<C, I, O> Uci<C, I, O> where
    C: Client + Send + 'static,
    I: BufRead,
    O: Write + Send + 'static
{
    pub fn new(client: C, input: I, output: O) -> Self {
        // In order to share client and output across threads we need 'static references to them
        let client = Box::leak(Box::new(client));
        let output = Box::leak(Box::new(output));
        Self {
            client: Arc::new(Mutex::new(client)),
            input,
            output: Arc::new(Mutex::new(output)),
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
                    writeln!(self.output.lock().unwrap(), "Failed to parse command '{:?}'", command.trim())?
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

            "ucinewclient" => Ok(Command::UciNewclient),

            "position" => match self.client.try_lock() {
                Ok(ref mut client) => match *command_args.get(1).ok_or(())? {
                    "startpos" => match command_args.get(2) {
                        Some(&"moves") => Ok(Command::Position {
                            fen: STARTING_FEN.to_string(),
                            moves: Self::parse_move_list(client, &command_args[3..])?,
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
                                moves: Self::parse_move_list(client, &command_args[3 + fen_vec.len()..])?,
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

            "go" => match self.client.try_lock() {
                Ok(ref client) => {
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

                                    let mv = Move::try_from(arg, client.get_moves().as_ref())
                                        .map_err(|_| ())?;
                                    moves.push(mv);
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

    fn parse_move_list(client: &mut C, moves: &[&str]) -> Result<Vec<Move>, ()> {
        let mut parsed = Vec::new();

        for mv in moves {
            match Move::try_from(mv, client.get_moves().as_ref()) {
                Ok(mv) => {
                    parsed.push(mv);
                    client.make_move(mv).unwrap();
                },
                Err(_) => break,
            }
        }

        for _ in 0..parsed.len() {
            client.unmake_move().unwrap();
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
            writeln!(self.output.lock().unwrap(), "Executing command '{:?}'", cmd)?;
        }

        match cmd {
            Command::Debug(val) => {
                self.debug = val;
                writeln!(self.output.lock().unwrap(), "Debug is now {}", match val {
                    true => "on",
                    false => "off",
                })?;
            },
            Command::IsReady => self.ready_ok()?,
            Command::UciNewclient => match self.client.try_lock() {
                Ok(ref mut client) => client.set_position(STARTING_FEN).unwrap(),
                Err(_) => if self.debug {
                    writeln!(self.output.lock().unwrap(), "Can't start a new client while thinking")?;
                }
            }
            Command::Position { fen, moves } => match self.client.try_lock() {
                Ok(ref mut client) => match client.set_position(&fen) {
                    Ok(_) => {
                        for mv in moves {
                            client.make_move(mv).unwrap();
                        }
                    }
                    Err(err) => if self.debug {
                        writeln!(self.output.lock().unwrap(), "{}", err)?;
                    }
                }
                Err(_) => if self.debug {
                    writeln!(self.output.lock().unwrap(), "Can't execute position command while thinking")?;
                }
            }
            Command::Go(opts) => match self.client.try_lock() {
                Ok(client) => {
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
                                writeln!(self.output.lock().unwrap(), "Unsupported go option {:?}", opt)?;
                            }
                        }
                    }

                    let out = Arc::clone(&self.output);
                    let client = Arc::clone(&self.client);
                    let (tx, rx) = mpsc::channel();
                    self.think = Some(rx);
                    thread::spawn(move || {
                        let client = client.lock().unwrap();

                        match max_depth {
                            None => {
                                for d in 1.. {
                                    let best = client.search(d);
                                    if tx.send(best).is_err() {
                                        return;
                                    }
                                    writeln!(out.lock().unwrap(), "info depth {}", d).unwrap();
                                }
                            }
                            Some(depth) => {
                                let mut best = Move::Regular(Square::E1, Square::E2);
                                for d in 1..=depth {
                                    best = client.search(d);
                                    if tx.send(best).is_err() {
                                        return;
                                    }
                                    writeln!(out.lock().unwrap(), "info depth {}", d).unwrap();
                                }
                                writeln!(out.lock().unwrap(), "bestmove {}", best).unwrap();
                            }
                        }
                    });
                }
                Err(_) => if self.debug {
                    writeln!(self.output.lock().unwrap(), "Can't execute position command while thinking")?;
                }
            },
            Command::Stop => {
                let best_mv = match self.think.as_ref().and_then(|rx| rx.try_recv().ok()) {
                    None => {
                        if self.debug {
                            writeln!(self.output.lock().unwrap(), "stop called while not thinking")?;
                        }
                        return Ok(());
                    }
                    Some(mv) => mv
                };
                self.think = None;
                self.best_move(best_mv)?;
            }
            Command::Quit => exit(0),
            _ => if self.debug {
                writeln!(self.output.lock().unwrap(), "Unsupported command")?;
            },
        }

        Ok(())
    }

    fn id(&mut self) -> std::io::Result<()> {
        writeln!(self.output.lock().unwrap(), "id name Crusty")?;
        writeln!(self.output.lock().unwrap(), "id author Lasse Møldrup")
    }

    fn uci_ok(&mut self) -> std::io::Result<()> {
        writeln!(self.output.lock().unwrap(), "uciok")
    }

    fn ready_ok(&mut self) -> std::io::Result<()> {
        writeln!(self.output.lock().unwrap(), "readyok")
    }

    fn best_move(&mut self, mv: Move) -> std::io::Result<()> {
        writeln!(self.output.lock().unwrap(), "bestmove {}", mv)
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
    UciNewclient,
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
