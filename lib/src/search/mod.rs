use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

mod thread;
pub use thread::{info_channel, InfoReceiver, InfoSender, ThreadPool};

#[derive(Default, Clone, Copy)]
struct Limits {
    moves: Option<()>,
    depth: Option<u8>,
    nodes: Option<u64>,
    time: Option<Duration>,
}

pub struct SearchJob {
    position: (),
    limits: Limits,
}

pub struct SearchInfo;

impl SearchInfo {
    pub fn combine(self, other: Self) -> Self {
        Self
    }
}

impl SearchJob {
    fn search(self, kill_switch: Arc<AtomicBool>) -> SearchInfo {
        // TODO: Search
        SearchInfo
    }
}
