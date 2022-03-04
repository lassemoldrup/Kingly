use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::time::Instant;

use crusty::framework::Client;
use crusty::framework::io::{Input, Output};
use crusty::framework::moves::Move;

use crate::uci::Uci;

pub struct Cli<C, I, O> {
    client: C,
    input: I,
    output: O,
    uci: bool,
}

impl<C, I, O> Cli<C, I, O> where
    C: Client + Send + Debug + 'static,
    I: Input,
    O: Output + Send + 'static
{
    pub fn new(client: C, input: I, output: O) -> Self {
        Self {
            client,
            input,
            output,
            uci: false
        }
    }

    pub fn start(mut self) -> std::io::Result<()> {
        self.print_welcome()?;

        loop {
            if self.uci {
                let uci = Uci::new(self.client, self.input, self.output);
                return uci.start();
            }

            write!(self.output, "> ")?;
            self.output.flush()?;

            let command = self.input.read_line()?;
            if command.trim().is_empty() {
                continue;
            }

            match self.parse_command(&command) {
                Ok(cmd) => self.execute(cmd)?,
                Err(err) => writeln!(self.output, "{}\n", err.0)?,
            }
        }
    }

    fn print_welcome(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "Crusty ver. {}\n", env!("CARGO_PKG_VERSION"))?;
        writeln!(self.output, "Commands:")?;
        writeln!(self.output, "uci\t\t\tStarts UCI mode")?;
        writeln!(self.output, "fen <arg>\t\tSets the position to the given FEN")?;
        writeln!(self.output, "move <arg1> [<arg2>..]\tMakes the supplied list of moves on the board")?;
        writeln!(self.output, "perft <arg>\t\tRuns Perft with the given depth")?;
        writeln!(self.output, "divide <arg>\t\tRuns Divide with the given depth")?;
        writeln!(self.output, "debug\t\t\tPrints debug information for the current position")?;
        writeln!(self.output)
    }

    fn init_client(&mut self) -> std::io::Result<()> {
        if self.client.is_init() {
            return Ok(());
        }

        writeln!(self.output, "Initializing tables..")?;
        self.client.init();
        writeln!(self.output, "Tables initialized")
    }

    fn parse_command(&mut self, command: &str) -> Result<Command, ParseError> {
        let command_args: Vec<&str> = command.split_ascii_whitespace().collect();
        match command_args[0] {
            "uci" => Ok(Command::Uci),
            "perft" => Ok(Command::Perft(
                command_args.get(1)
                    .ok_or_else(|| ParseError("Missing argument".to_string()))?.parse()
                    .map_err(|_| ParseError("Argument must be a number".to_string()))?
            )),
            "divide" => Ok(Command::Divide(
                command_args.get(1)
                    .ok_or_else(|| ParseError("Missing argument".to_string()))?.parse()
                    .map_err(|_| ParseError("Argument must be a number".to_string()))?
            )),
            "fen" => Ok(Command::Fen(
                command[3..].trim().to_string()
            )),
            "move" => {
                // We need to use the client in order to parse the moves
                self.init_client().unwrap();
                
                let mut moves = Vec::new();

                for mv in &command_args[1..] {
                    match Move::try_from(mv, self.client.get_moves().as_ref()) {
                        Ok(mv) => {
                            moves.push(mv);
                            self.client.make_move(mv).unwrap();
                        },
                        Err(_) => {
                            moves.iter()
                                .for_each(|_| self.client.unmake_move().unwrap());
                            break;
                        },
                    }
                }

                for _ in 0..moves.len() {
                    self.client.unmake_move().unwrap();
                }

                if !moves.is_empty() && moves.len() == command_args[1..].len() {
                    Ok(Command::Move(moves))
                } else {
                    Err(ParseError("Missing argument".to_string()))
                }
            },
            "debug" => Ok(Command::Debug),
            _ => Err(ParseError("Invalid command".to_string())),
        }
    }

    fn execute(&mut self, command: Command) -> std::io::Result<()> {
        match command {
            Command::Uci => {
                self.uci = true;
                return Ok(());
            },
            Command::Perft(depth) => {
                self.init_client()?;

                writeln!(self.output, "Running Perft with depth {}...", depth)?;
                let start = Instant::now();
                let nodes = self.client.perft(depth);
                let elapsed = start.elapsed();
                writeln!(self.output, "Nodes:\t{}\nTime:\t{} ms\nNPS:\t{:.0} kn/s",
                         nodes, elapsed.as_millis(), (nodes as f64)/elapsed.as_secs_f64()/1000.0)?;
            },
            Command::Divide(depth) => {
                self.init_client()?;

                writeln!(self.output, "Running Divide with depth {}...", depth)?;

                let mut total = 0;
                for mv in self.client.get_moves() {
                    self.client.make_move(mv).unwrap();
                    let nodes = self.client.perft(depth - 1);
                    total += nodes;
                    self.client.unmake_move().unwrap();

                    writeln!(self.output, "{}: {}", mv, nodes)?;
                }

                writeln!(self.output, "\nTotal: {}", total)?;
            },
            Command::Fen(fen) => match self.client.set_position(&fen) {
                Ok(_) => { },
                Err(err) => writeln!(self.output, "{}", err)?,
            },
            Command::Move(moves) => {
                self.init_client()?;

                for mv in moves {
                    if self.client.make_move(mv).is_err() {
                        writeln!(self.output, "Illegal move '{}'", mv)?;
                        break;
                    }
                }
            }
            Command::Debug => writeln!(self.output, "{:?}", self.client)?,
        }
        writeln!(self.output)
    }
}


enum Command {
    Uci,
    Perft(u32),
    Divide(u32),
    Fen(String),
    Move(Vec<Move>),
    Debug,
}


struct ParseError(String);

impl Error for ParseError { }

impl From<String> for ParseError {
    fn from(err: String) -> Self {
        Self(err)
    }
}

impl Debug for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl Display for ParseError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}
