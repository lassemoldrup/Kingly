use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::framework::moves::{Move, PseudoMove};
use crate::framework::value::Value;

pub trait Search<'f> {
    fn moves(self, moves: &[PseudoMove]) -> Self;
    fn depth(self, depth: u32) -> Self;
    fn time(self, time: Duration) -> Self;
    fn nodes(self, nodes: u64) -> Self;
    fn on_info<F: FnMut(&SearchResult) + 'f>(self, callback: F) -> Self;
    fn start(self, stop_search: &AtomicBool);
}


#[derive(Clone)]
pub struct SearchResult {
    value: Value,
    line: Vec<Move>,
    depth: u32,
    nodes_searched: u64,
    duration: Duration,
}

impl SearchResult {
    pub fn new(value: Value, line: Vec<Move>, depth: u32, nodes_searched: u64, duration: Duration) -> Self {
        Self {
            value,
            line,
            depth,
            nodes_searched,
            duration,
        }
    }

    pub fn value(&self) -> Value {
        self.value
    }

    pub fn line(&self) -> &[Move] {
        &self.line
    }

    pub fn depth(&self) -> u32 {
        self.depth
    }

    pub fn nodes_searched(&self) -> u64 {
        self.nodes_searched
    }

    pub fn duration(&self) -> Duration {
        self.duration
    }
}
