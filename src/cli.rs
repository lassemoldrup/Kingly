use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::io::{BufRead, Write};

use crusty::framework::Game;

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
                Err(err) => self.write(err.0)?,
            }
        }
    }

    fn write(&mut self, msg: &str) -> std::io::Result<()> {
        writeln!(self.output, "{}", msg)
    }

    fn print_welcome(&mut self) -> std::io::Result<()> {
        self.write("Crusty ver. 0.0.1")
    }

    fn parse_command(command: &str) -> Result<Command, ParseError> {
        let command: Vec<&str> = command.split_ascii_whitespace().collect();
        match command[0] {
            "uci" => Ok(Command::Uci),
            "perft" => Ok(Command::Perft(
                command.get(1)
                    .ok_or(ParseError("Missing argument"))?.parse()
                    .map_err(|_| ParseError("Argument must be a number"))?
            )),
            _ => Err(ParseError("Invalid command"))
        }
    }

    fn execute(&mut self, command: Command) -> std::io::Result<()> {
        match command {
            Command::Uci => unimplemented!(),
            Command::Perft(_) => self.write("Perft"),
        }
    }
}


enum Command {
    Uci,
    Perft(u32),
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
