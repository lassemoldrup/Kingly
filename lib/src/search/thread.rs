use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam::channel::{self, Receiver, Sender};
use itertools::Itertools;

use crate::types::Move;

use super::{SearchJob, SearchResult};

/// The sending side of a channel that sends search info.
pub type InfoSender = Sender<SearchInfo>;
/// The receiving side of a channel that sends search info.
pub type InfoReceiver = Receiver<SearchInfo>;
/// Create a new channel for sending search info.
pub fn info_channel() -> (InfoSender, InfoReceiver) {
    channel::unbounded()
}

// Right now it seems that more than 6 threads is detrimental.
const MAX_THREADS: usize = 6;

/// A thread pool for running search jobs.
pub struct ThreadPool {
    runner_thread: Option<std::thread::JoinHandle<Option<Move>>>,
    kill_switch: Arc<AtomicBool>,
}

impl ThreadPool {
    /// Create a new thread pool.
    pub fn new() -> Self {
        Self {
            runner_thread: None,
            kill_switch: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Try to spawn threads to run a search job. Returns `false` if a job is already running.
    pub fn spawn(&mut self, job: SearchJob, info_tx: InfoSender) -> bool {
        if self.runner_thread.is_some() {
            return false;
        }
        let runner = JobRunner {
            job,
            info_tx,
            kill_switch: self.kill_switch.clone(),
        };
        self.runner_thread = Some(std::thread::spawn(move || runner.run()));
        true
    }

    /// Stop the currently running job and return the best move found so far.
    pub fn stop(&mut self) -> Option<Move> {
        self.kill_switch.store(true, Ordering::Relaxed);
        self.wait()
    }

    /// Wait for the currently running job to finish and return the best move found.
    pub fn wait(&mut self) -> Option<Move> {
        self.runner_thread
            .take()
            .and_then(|h| h.join().expect("runner thread shouldn't panic"))
    }
}

struct JobRunner {
    job: SearchJob,
    info_tx: InfoSender,
    kill_switch: Arc<AtomicBool>,
}

impl JobRunner {
    fn run(self) -> Option<Move> {
        let search_start = Instant::now();

        log::info!("Starting search with {:?}", self.job.limits);
        let num_threads = num_cpus::get().min(MAX_THREADS);
        let max_depth = self.job.limits.depth.unwrap_or(i8::MAX);

        let mut best_move = None;
        // Iterative deepening
        for depth in 1..=max_depth {
            let iteration_start = Instant::now();

            if self.kill_switch.load(Ordering::Relaxed) {
                log::info!("Kill switch received, stopping search.");
                break;
            }

            let Some(merged_result) = std::thread::scope(|s| {
                // Need to collect the handles to ensure that the threads are spawned
                let handles = (0..num_threads)
                    .map(|_| s.spawn(|| self.search_depth(depth, search_start)))
                    .collect_vec();
                handles
                    .into_iter()
                    .filter_map(|h| h.join().expect("thread shouldn't panic"))
                    .reduce(SearchResult::combine)
            }) else {
                break;
            };

            best_move = merged_result.pv.first().copied();
            let info =
                SearchInfo::from_result(merged_result, search_start, iteration_start, depth, 0);
            if self.info_tx.send(info).is_err() {
                log::info!("Info channel closed, stopping search.");
                break;
            }
        }
        best_move
    }

    fn search_depth(&self, depth: i8, search_start: Instant) -> Option<SearchResult> {
        self.job
            .clone()
            .search(depth, search_start, self.kill_switch.clone())
    }
}

/// Information about an ongoing search. One of these is reated for each iteration of the search.
#[derive(Debug, Clone)]
pub struct SearchInfo {
    /// The depth of the last completed iteration.
    pub depth: i8,
    /// The search result of the last completed iteration.
    pub result: SearchResult,
    /// The number of nodes per second searched in the last iteration.
    pub nps: u64,
    /// The total duration of the search so far.
    pub total_duration: Duration,
    /// The fullness of the hash table as a per mille value.
    pub hash_full: u32,
}

impl SearchInfo {
    fn from_result(
        result: SearchResult,
        search_start: Instant,
        iteration_start: Instant,
        depth: i8,
        hash_full: u32,
    ) -> Self {
        let total_duration = search_start.elapsed();
        let elapsed_nanos = iteration_start.elapsed().as_nanos();
        let nps = (result.stats.nodes as u128 * 1_000_000_000 / elapsed_nanos) as u64;
        Self {
            depth,
            result,
            nps,
            total_duration,
            hash_full,
        }
    }
}

impl Display for SearchInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "depth {} seldepth {} score {} nodes {} nps {} hashfull {} pv",
            self.depth,
            self.result.stats.sel_depth,
            self.result.score,
            self.result.stats.nodes,
            self.nps,
            self.hash_full,
        )?;
        for mv in &self.result.pv {
            write!(f, " {}", mv)?;
        }
        write!(f, " time {}", self.total_duration.as_millis())
    }
}
