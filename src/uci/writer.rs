use std::io;
use std::sync::Arc;

use crusty::types::Move;
use itertools::Itertools;
use parking_lot::Mutex;

use crate::io::Output;

use super::SearchInfo;

/// Writes UCI-messages to the output atomically. Has the same Clone-semantics as `Arc`
#[derive(Clone, Debug)]
pub struct Writer<O> {
    output: Arc<Mutex<O>>,
}

impl<O: Output> Writer<O> {
    pub fn new(output: O) -> Self {
        Self {
            output: Arc::new(Mutex::new(output)),
        }
    }

    pub fn debug(&self, msg: impl AsRef<str>) -> io::Result<()> {
        self.info(&[SearchInfo::String(format!("Debug: {}", msg.as_ref()))])
    }

    pub fn id(&self) -> io::Result<()> {
        let output = self.output.lock();
        writeln!(output, "id name Crusty")?;
        writeln!(output, "id author Lasse Møldrup")
    }

    pub fn options(&self) -> io::Result<()> {
        writeln!(
            self.output.lock(),
            "option name Hash type spin default 16 min 1 max 1048576"
        )
    }

    pub fn uci_ok(&self) -> io::Result<()> {
        writeln!(self.output.lock(), "uciok")
    }

    pub fn ready_ok(&self) -> io::Result<()> {
        writeln!(self.output.lock(), "readyok")
    }

    pub(in crate::uci) fn info(&self, info: &[SearchInfo]) -> io::Result<()> {
        writeln!(self.output.lock(), "info {}", info.iter().join(" "))
    }

    pub fn best_move(&self, best_move: Move) -> io::Result<()> {
        writeln!(self.output.lock(), "bestmove {}", best_move)
    }
}

#[cfg(test)]
impl<O: Clone> Writer<O> {
    pub fn get_output(&self) -> O {
        self.output.lock().clone()
    }
}
