use crusty::framework::Game;
use std::io::{BufRead, Write};
use crusty::framework::fen::STARTING_FEN;

pub struct Uci<'a, G, I, O> {
    game: G,
    input: &'a mut I,
    output: &'a mut O,
    debug: bool,
}

impl<'a, G: Game, I: BufRead + 'a, O: Write + 'a> Uci<'a, G, I, O> {
    pub fn new(game: G, input: &'a mut I, output: &'a mut O) -> Self {
        Self {
            game,
            input,
            output,
            debug: false,
        }
    }

    pub fn start(mut self) -> std::io::Result<()> {
        self.id()?;
        self.uci_ok()?;

        loop {
            let mut command = String::new();
            self.input.read_line(&mut command)?;

            match Self::parse_command(&command) {
                Ok(cmd) => self.execute(cmd)?,
                Err(_) => if self.debug {
                    writeln!(self.output, "Failed to parse command '{:?}'", command.trim())?
                },
            }
        }
    }

    fn parse_command(cmd: &str) -> Result<Command, ()> {
        let cmd = cmd.to_ascii_lowercase();
        let command_args: Vec<&str> = cmd.split_ascii_whitespace()
            .collect();

        match command_args[0] {
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

            "position" => match *command_args.get(1).ok_or(())? {
                "startpos" => match command_args.get(2) {
                    Some(&"moves") => Ok(Command::Position {
                        fen: STARTING_FEN.to_string(),
                        moves: command_args[3..].iter()
                            .map(|m| m.to_string())
                            .collect(),
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
                            moves: command_args[3 + fen_vec.len()..].iter()
                                .map(|m| m.to_string())
                                .collect(),
                        }),
                        Some(_) => Err(()),
                        None => Ok(Command::Position {
                            fen,
                            moves: Vec::new(),
                        }),
                    }
                },
                _ => Err(()),
            },

            "go" => {
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

                                moves.push(arg.to_string());
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

            "stop" => Ok(Command::Stop),

            "ponderhit" => Ok(Command::PonderHit),

            "quit" => Ok(Command::Quit),

            "" => Err(()),

            _ => Self::parse_command(&command_args[1..].join(" ")),
        }
    }

    fn is_go_command(cmd: &str) -> bool {
        let cmds = ["searchmoves", "ponder", "wtime", "btime", "winc", "binc", "movestogo", "depth",
            "nodes", "mate", "movetime", "infinite"];

        cmds.contains(&cmd)
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
            _ => {}
        }

        Ok(())
    }

    fn id(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "id name Crusty")?;
        writeln!(self.output, "id author Lasse Møldrup")
    }

    fn uci_ok(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "uciok")
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
        moves: Vec<String>,
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
    SearchMoves(Vec<String>),
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
