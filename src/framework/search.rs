use crate::framework::moves::Move;
use crate::framework::value::Value;
use std::time::Duration;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub trait Search<'a> {
    fn on_info<F: FnMut(&SearchResult) + 'a>(&mut self, callback: F);
    fn start(self, stop_switch: Arc<AtomicBool>);
}


pub struct SearchResult {
    value: Value,
    line: Box<[Move]>,
    depth: u32,
    nodes_searched: u64,
    duration: Duration,
}

impl SearchResult {
    pub fn new(value: Value, line: Vec<Move>, depth: u32, nodes_searched: u64, duration: Duration) -> Self {
        let line = line.into_boxed_slice();
        Self {
            value, line, depth, nodes_searched, duration
        }
    }

    pub fn value(&self) -> Value {
        self.value
    }

    pub fn line(&self) -> &[Move] {
        &*self.line
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