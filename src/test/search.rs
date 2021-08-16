use crate::framework::search::{Search, SearchResult};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

pub struct SearchStub;

impl<'f> Search<'f> for SearchStub {
    fn on_info<F: FnMut(&SearchResult) + 'f>(&mut self, callback: F) {
        todo!()
    }

    fn start(self, stop_switch: Arc<AtomicBool>) {
        todo!()
    }
}