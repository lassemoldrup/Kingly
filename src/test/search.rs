use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use crate::framework::search::{Search, SearchResult};

pub struct SearchStub<'a> {
    callback: Box<dyn FnMut(&SearchResult) + 'a>,
    search_result: SearchResult,
}

impl<'a> SearchStub<'a> {
    pub fn new(search_result: SearchResult) -> Self {
        Self {
            callback: Box::new(|_| {}),
            search_result,
        }
    }
}

impl<'a> Search<'a> for SearchStub<'a> {
    fn on_info<F: FnMut(&SearchResult) + 'a>(&mut self, callback: F) {
        self.callback = Box::new(callback);
    }

    fn start(mut self, _stop_switch: Arc<AtomicBool>) {
        (self.callback)(&self.search_result);
    }
}