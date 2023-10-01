use std::io::{self, BufRead, StdoutLock, Write};
use std::thread;

use crossbeam::channel::{self, Receiver, Sender};

pub struct Uci<W> {
    input_rx: Receiver<String>,
    write_handle: W,
}

impl Uci<StdoutLock<'_>> {
    pub fn new_standard() -> Self {
        let (input_tx, input_rx) = channel::unbounded();
        thread::spawn(move || {
            let stdin = io::stdin().lock();
            handle_input(stdin, input_tx)
        });
        Self {
            input_rx,
            write_handle: io::stdout().lock(),
        }
    }
}

impl<W: Write> Uci<W> {
    pub fn repl(mut self) -> io::Result<()> {
        self.print_prelude()?;

        let (search_tx, search_rx) = channel::unbounded::<()>();
        // TODO: Spawn search thread.
        let command_handler = CommandHandler { search_tx };

        loop {
            channel::select! {
                recv(self.input_rx) -> line => {
                    let line = line.expect("sender should be alive");
                    command_handler.handle(&line, &mut self.write_handle)?;
                }
                recv(search_rx) -> _ => {
                    todo!();
                }
            }
        }
    }

    fn print_prelude(&mut self) -> io::Result<()> {
        writeln!(self.write_handle, "id name Kingly")?;
        writeln!(self.write_handle, "id author {}", env!("CARGO_PKG_AUTHORS"))?;
        // TODO: Add constant for max value.
        writeln!(
            self.write_handle,
            "option name Hash type spin default 16 min 1 max 1048576"
        )?;
        writeln!(self.write_handle, "uciok")
    }
}

fn handle_input<R: BufRead>(read_handle: R, tx: Sender<String>) -> io::Result<()> {
    for line in read_handle.lines() {
        tx.send(line?).expect("receiver should be alive");
    }
    Ok(())
}

struct CommandHandler {
    search_tx: Sender<()>,
}

impl CommandHandler {
    fn handle<W: Write>(&self, command: &str, write_handle: &mut W) -> io::Result<()> {
        todo!()
    }
}

enum Command {}
