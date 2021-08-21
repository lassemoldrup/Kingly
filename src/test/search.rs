use std::sync::atomic::AtomicBool;

use crate::framework::search::{Search, SearchResult};
use crate::framework::moves::PseudoMove;
use std::time::Duration;

pub struct SearchStub<'a> {
    search_moves: Option<Vec<PseudoMove>>,
    callback: Box<dyn FnMut(&SearchResult) + 'a>,
    search_result: SearchResult,
}

impl<'a> SearchStub<'a> {
    pub fn new(search_result: SearchResult) -> Self {
        Self {
            search_moves: None,
            callback: Box::new(|_| {}),
            search_result,
        }
    }
}

impl<'a> Search<'a> for SearchStub<'a> {
    fn moves(mut self, moves: &[PseudoMove]) -> Self {
        self.search_moves = Some(moves.to_vec());
        self
    }

    fn depth(self, _depth: u32) -> Self {
        todo!()
    }

    fn time(self, _time: Duration) -> Self {
        todo!()
    }

    fn nodes(self, _nodes: u64) -> Self {
        todo!()
    }

    fn on_info<F: FnMut(&SearchResult) + 'a>(mut self, callback: F) -> Self {
        self.callback = Box::new(callback);
        self
    }

    fn start(mut self, _stop_search: &AtomicBool) {
        (self.callback)(&self.search_result);
    }
}