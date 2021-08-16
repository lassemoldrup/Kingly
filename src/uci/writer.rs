use std::io;

use crusty::framework::io::Output;
use crate::uci::SearchInfo;
use itertools::Itertools;

#[derive(Clone)]
pub struct Writer<O> {
    output: O,
}

impl<O: Output> Writer<O> {
    pub fn new(output: O) -> Self {
        Self {
            output,
        }
    }

    pub fn into_output(self) -> O {
        self.output
    }

    pub fn debug(&mut self, msg: impl AsRef<str>) -> io::Result<()> {
        writeln!(self.output, "Debug: {}", msg.as_ref())
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
}