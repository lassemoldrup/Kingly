use std::io;
use std::sync::Arc;

use itertools::Itertools;
use kingly_lib::types::Move;
use parking_lot::Mutex;

use crate::io::Output;

use super::GoInfoPair;

/// Writes UCI-messages to the output atomically. Has the same Clone-semantics as `Arc`
#[derive(Debug)]
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
        self.info(&[GoInfoPair::String(format!("Debug: {}", msg.as_ref()))])
    }

    pub fn id(&self) -> io::Result<()> {
        let mut output = self.output.lock();
        writeln!(output, "id name Kingly")?;
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

    pub(super) fn info(&self, info: &[GoInfoPair]) -> io::Result<()> {
        writeln!(self.output.lock(), "info {}", info.iter().join(" "))
    }

    pub fn best_move(&self, best_move: Move) -> io::Result<()> {
        writeln!(self.output.lock(), "bestmove {}", best_move)
    }
}

impl<O> Clone for Writer<O> {
    fn clone(&self) -> Self {
        Self {
            output: self.output.clone(),
        }
    }
}

#[cfg(test)]
impl<O: Clone> Writer<O> {
    pub fn get_output(&self) -> O {
        self.output.lock().clone()
    }
}
