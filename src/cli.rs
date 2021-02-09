use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufRead, Write};

use crusty::framework::Game;
use std::time::Instant;

pub struct Cli<G, I, O> {
    game: G,
    input: I,
    output: O,
}

impl<G: Game, I: BufRead, O: Write> Cli<G, I, O> {
    pub fn new(game: G, input: I, output: O) -> Self {
        Self {
            game,
            input,
            output,
        }
    }

    pub fn start(mut self) -> std::io::Result<()> {
        self.print_welcome()?;

        loop {
            let mut command = String::new();
            self.input.read_line(&mut command)?;

            if command.trim().is_empty() {
                continue;
            }

            match Self::parse_command(&command) {
                Ok(cmd) => self.execute(cmd)?,
                Err(err) => writeln!(self.output, "{}", err.0)?,
            }
        }
    }

    fn print_welcome(&mut self) -> std::io::Result<()> {
        writeln!(self.output, "Crusty ver. 0.0.1\n")?;
        writeln!(self.output, "Commands:")?;
        writeln!(self.output, "uci\t\tStarts UCI mode (UNIMPLEMENTED)")?;
        writeln!(self.output, "fen arg\t\tSets the position to the given FEN")?;
        writeln!(self.output, "perft arg\tRuns Perft with the given depth")?;
        writeln!(self.output)
    }

    fn parse_command(command: &str) -> Result<Command, ParseError> {
        let command_args: Vec<&str> = command.split_ascii_whitespace().collect();
        match command_args[0] {
            "uci" => Ok(Command::Uci),
            "perft" => Ok(Command::Perft(
                command_args.get(1)
                    .ok_or(ParseError("Missing argument"))?.parse()
                    .map_err(|_| ParseError("Argument must be a number"))?
            )),
            "fen" => Ok(Command::Fen(
                command[3..].trim().to_string()
            )),
            _ => Err(ParseError("Invalid command"))
        }
    }

    fn execute(&mut self, command: Command) -> std::io::Result<()> {
        match command {
            Command::Uci => unimplemented!(),
            Command::Perft(depth) => {
                writeln!(self.output, "Running Perft with depth {}...", depth)?;
                let start = Instant::now();
                let res = self.game.perft(depth);
                let elapsed = start.elapsed();
                writeln!(self.output, "Nodes: {}\nTime: {}ms\nNPS: {}",
                         res, elapsed.as_millis(), ((res as f64)/elapsed.as_secs_f64()))?;
            },
            Command::Fen(fen) => {
                match self.game.set_position(&fen) {
                    Ok(_) => { },
                    Err(err) => writeln!(self.output, "{}", err)?,
                }
            }
        }
        writeln!(self.output)
    }
}


enum Command {
    Uci,
    Perft(u32),
    Fen(String),
}


struct ParseError(&'static str);

impl Error for ParseError { }

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
