use std::io;

use itertools::Itertools;

use crusty::framework::io::Output;

use crate::uci::SearchInfo;
use crusty::framework::moves::Move;

#[derive(Clone, Debug)]
pub struct Writer<O> {
    output: O,
}

impl<O: Output> Writer<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
        }
    }

    #[cfg(test)]
    pub fn into_output(self) -> O {
        self.output
    }

    pub fn debug(&mut self, msg: impl AsRef<str>) -> io::Result<()> {
        self.info(&[SearchInfo::String(format!("Debug: {}", msg.as_ref()))])
    }

    pub fn id(&mut self) -> io::Result<()> {
        writeln!(self.output, "id name Crusty")?;
        writeln!(self.output, "id author Lasse Møldrup")
    }

    pub fn uci_ok(&mut self) -> io::Result<()> {
        writeln!(self.output, "uciok")
    }

    pub fn ready_ok(&mut self) -> io::Result<()> {
        writeln!(self.output, "readyok")
    }

    pub(in crate::uci) fn info(&mut self, info: &[SearchInfo]) -> io::Result<()> {
        writeln!(self.output, "info {}", info.iter().join(" "))
    }

    pub fn best_move(&mut self, best_move: Move) -> io::Result<()> {
        writeln!(self.output, "bestmove {}", best_move)
    }
}