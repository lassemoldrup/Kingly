use std::str::FromStr;
use std::time::Instant;

use crusty::eval::StandardEval;
use crusty::types::PseudoMove;

use crate::client::Client;
use crate::uci::Uci;

use super::io::{Input, Output};

pub struct Cli<I, O> {
    client: Client<StandardEval>,
    input: I,
    output: O,
    uci: bool,
}

impl<I, O> Cli<I, O>
where
    I: Input,
    O: Output + Send + 'static,
{
    pub fn new(client: Client<StandardEval>, input: I, output: O) -> Self {
        Self {
            client,
            input,
            output,
            uci: false,
        }
    }

    pub fn start(mut self) -> std::io::Result<()> {
        self.print_welcome()?;

        while !self.uci {
            write!(self.output, "> ")?;
            self.output.flush()?;

            let command = self.input.read_line()?;
            if command.trim().is_empty() {
                continue;
            }

            match Self::parse_command(&command) {
                Ok(cmd) => self.execute(cmd)?,
                Err(err) => writeln!(self.output, "{}\n", err)?,
            }
        }

        Uci::new(self.client, self.input, self.output).start()
    }

    fn print_welcome(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "Crusty ver. {}\n", env!("CARGO_PKG_VERSION"))?;
        writeln!(self.output, "Commands:")?;
        writeln!(self.output, "uci\t\t\tStarts UCI mode")?;
        writeln!(
            self.output,
            "fen <arg>\t\tSets the position to the given FEN"
        )?;
        writeln!(
            self.output,
            "move <arg1> [<arg2>..]\tMakes the supplied list of moves on the board"
        )?;
        writeln!(
            self.output,
            "perft <arg>\t\tRuns Perft with the given depth"
        )?;
        writeln!(
            self.output,
            "divide <arg>\t\tRuns Divide with the given depth"
        )?;
        writeln!(
            self.output,
            "debug\t\t\tPrints debug information for the current position"
        )?;
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

    fn parse_command(cmd: &str) -> Result<Command, String> {
        let cmds: Vec<&str> = cmd.split_ascii_whitespace().collect();
        match cmds[0] {
            "uci" => Ok(Command::Uci),
            "perft" => Ok(Command::Perft(
                cmds.get(1)
                    .ok_or_else(|| "Missing argument".to_string())?
                    .parse()
                    .map_err(|_| "Argument must be a number".to_string())?,
            )),
            "divide" => Ok(Command::Divide(
                cmds.get(1)
                    .ok_or_else(|| "Missing argument".to_string())?
                    .parse()
                    .map_err(|_| "Argument must be a number".to_string())?,
            )),
            "fen" => Ok(Command::Fen(cmd[3..].trim().to_string())),
            "move" => cmds[1..]
                .iter()
                .copied()
                .map(<PseudoMove as FromStr>::from_str)
                .collect::<Result<_, _>>()
                .map(|moves| Command::Move(moves)),
            "debug" => Ok(Command::Debug),
            _ => Err("Invalid command".to_string()),
        }
    }

    fn execute(&mut self, command: Command) -> std::io::Result<()> {
        match command {
            Command::Uci => {
                self.uci = true;
                return Ok(());
            }
            Command::Perft(depth) => {
                self.init_client()?;

                writeln!(self.output, "Running Perft with depth {}..", depth)?;
                let start = Instant::now();
                let nodes = self.client.perft(depth);
                let elapsed = start.elapsed();
                writeln!(
                    self.output,
                    "Nodes:\t{}\nTime:\t{} ms\nNPS:\t{:.0} kn/s",
                    nodes,
                    elapsed.as_millis(),
                    (nodes as f64) / elapsed.as_secs_f64() / 1000.0
                )?;
            }
            Command::Divide(depth) => {
                self.init_client()?;

                writeln!(self.output, "Running Divide with depth {}..", depth)?;

                let mut total = 0;
                for mv in self.client.get_moves() {
                    self.client.make_move(mv).unwrap();
                    let nodes = self.client.perft(depth - 1);
                    total += nodes;
                    self.client.unmake_move().unwrap();

                    writeln!(self.output, "{}: {}", mv, nodes)?;
                }

                writeln!(self.output, "\nTotal: {}", total)?;
            }
            Command::Fen(fen) => {
                self.init_client()?;

                match self.client.position(&fen, &[]) {
                    Ok(_) => {}
                    Err(err) => writeln!(self.output, "{}", err)?,
                }
            }
            Command::Move(moves) => {
                self.init_client()?;

                let legal_moves = self.client.get_moves();

                for mv in moves {
                    if let Err(err) = mv
                        .into_move(&legal_moves)
                        .and_then(|mv| self.client.make_move(mv))
                    {
                        writeln!(self.output, "{}", err)?;
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
    Move(Vec<PseudoMove>),
    Debug,
}
