use std::sync::atomic::AtomicBool;
use std::time::Duration;

use crate::framework::moves::{Move, PseudoMove};
use crate::framework::value::Value;

pub trait Search<'f> {
    fn moves(self, moves: &[PseudoMove]) -> Self;
    fn depth(self, depth: u8) -> Self;
    fn time(self, time: Duration) -> Self;
    fn nodes(self, nodes: u64) -> Self;
    fn on_info<F: FnMut(&SearchResult) + 'f>(self, callback: F) -> Self;
    fn start(self, stop_search: &AtomicBool);
}


#[derive(Clone)]
pub struct SearchResult {
    pub value: Value,
    pub line: Vec<Move>,
    pub depth: u8,
    pub sel_depth: u8,
    pub nodes_searched: u64,
    pub duration: Duration,
    pub hash_full: u32,
}

impl SearchResult {
    pub fn new(value: Value, line: Vec<Move>, depth: u8, sel_depth: u8,
        nodes_searched: u64, duration: Duration, hash_full: u32) -> Self
    {
        Self {
            value,
            line,
            depth,
            sel_depth,
            nodes_searched,
            duration,
            hash_full
        }
    }
}
