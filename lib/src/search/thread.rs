use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crossbeam::channel::{self, Receiver, Sender};
use itertools::Itertools;

use super::{Limits, SearchInfo, SearchJob};

pub type InfoSender = Sender<SearchInfo>;
pub type InfoReceiver = Receiver<SearchInfo>;
pub fn info_channel() -> (InfoSender, InfoReceiver) {
    channel::unbounded()
}

// Right now it seems that more than 6 threads is detrimental.
const MAX_THREADS: usize = 6;

pub struct ThreadPool {
    runner_thread: Option<thread::JoinHandle<SearchInfo>>,
    kill_switch: Arc<AtomicBool>,
}

impl ThreadPool {
    pub fn new() -> Self {
        Self {
            runner_thread: None,
            kill_switch: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn spawn(&mut self, job: SearchJob, info_tx: InfoSender) {
        let runner = JobRunner {
            job,
            info_tx,
            kill_switch: self.kill_switch.clone(),
        };
        self.runner_thread = Some(thread::spawn(move || runner.run()));
    }

    pub fn stop(&mut self) -> Option<SearchInfo> {
        self.kill_switch.store(true, Ordering::Relaxed);
        self.runner_thread
            .take()
            .map(|h| h.join().expect("runner thread shouldn't panic"))
    }
}

struct JobRunner {
    job: SearchJob,
    info_tx: InfoSender,
    kill_switch: Arc<AtomicBool>,
}

impl JobRunner {
    fn run(self) -> SearchInfo {
        let num_threads = num_cpus::get().min(MAX_THREADS);
        let max_depth = self.job.limits.depth.unwrap_or(u8::MAX);
        // Iterative deepening
        for depth in 1..=max_depth {
            if self.kill_switch.load(Ordering::Relaxed) {
                break;
            }

            let info = thread::scope(|s| {
                // Need to collect the handles to ensure that the threads are spawned
                let handles = (0..num_threads)
                    .map(|_| s.spawn(|| self.search_depth(depth)))
                    .collect_vec();
                handles
                    .into_iter()
                    .map(|h| h.join().unwrap())
                    .reduce(SearchInfo::combine)
                    .expect("there should be at least one thread")
            });

            if let Err(_) = self.info_tx.send(info) {
                break;
            }
        }
        SearchInfo
    }

    fn search_depth(&self, depth: u8) -> SearchInfo {
        let job = SearchJob {
            limits: Limits {
                depth: Some(depth),
                ..self.job.limits
            },
            ..self.job
        };
        job.search(self.kill_switch.clone())
    }
}
