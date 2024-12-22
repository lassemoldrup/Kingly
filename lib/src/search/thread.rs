use std::fmt::{self, Display, Formatter};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam::channel::{self, Receiver, Sender};
use itertools::Itertools;

use crate::types::Move;

use super::{SearchJob, SearchResult, TranspositionTable};

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
///
/// # Example
/// ```
/// use kingly_lib::search::{ThreadPool, SearchJob, info_channel};
/// use kingly_lib::position::Position;
///
/// let mut thread_pool = ThreadPool::new();
/// let start_pos = Position::new();
/// let job = SearchJob::default_builder()
///     .position(start_pos)
///     .depth(4)
///     .build();
///
/// let rx = thread_pool.spawn(job).expect("search is not running");
/// let best_move = thread_pool.wait();
/// assert!(best_move.is_some());
/// ```
pub struct ThreadPool {
    runner_thread: Option<std::thread::JoinHandle<Option<Move>>>,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
}

impl ThreadPool {
    /// Create a new thread pool.
    pub fn new() -> Self {
        Self {
            runner_thread: None,
            kill_switch: Arc::new(AtomicBool::new(false)),
            t_table: Arc::new(TranspositionTable::new()),
        }
    }

    /// Try to spawn threads to run a search job. Returns an error if a search
    /// is already running.
    pub fn spawn(&mut self, job: SearchJob) -> Result<InfoReceiver, SearchRunningError> {
        let (tx, rx) = info_channel();
        self.spawn_with_channel(job, tx)?;
        Ok(rx)
    }

    /// Try to spawn threads to run a search job using a pre-constructed
    /// channel. Returns an error if a search is already running.
    pub fn spawn_with_channel(
        &mut self,
        job: SearchJob,
        info_tx: InfoSender,
    ) -> Result<(), SearchRunningError> {
        if self.runner_thread.is_some() {
            return Err(SearchRunningError);
        }
        let runner = JobRunner {
            job,
            info_tx,
            kill_switch: Arc::clone(&self.kill_switch),
            t_table: Arc::clone(&self.t_table),
        };
        self.runner_thread = Some(std::thread::spawn(move || runner.run()));
        Ok(())
    }

    /// Stop the currently running job and return the best move found so far.
    pub fn stop(&mut self) -> Option<Move> {
        self.kill_switch.store(true, Ordering::Relaxed);
        self.wait()
    }

    /// Wait for the currently running job to finish and return the best move
    /// found.
    pub fn wait(&mut self) -> Option<Move> {
        self.runner_thread
            .take()
            .and_then(|h| h.join().expect("runner thread shouldn't panic"))
    }

    /// Sets the size of the transposition table in MB. This can only be done
    /// when no search is running. Returns an error if a search is running.
    pub fn set_hash_size(&mut self, size: usize) -> Result<(), SearchRunningError> {
        let Some(t_table) = Arc::get_mut(&mut self.t_table) else {
            return Err(SearchRunningError);
        };
        assert!(self.runner_thread.is_none());
        *t_table = TranspositionTable::with_hash_size(size);
        Ok(())
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.stop();
    }
}

struct JobRunner {
    job: SearchJob,
    info_tx: InfoSender,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
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
            let hash_full = ((self.t_table.len() * 1000) / self.t_table.capacity()) as u32;
            let info = SearchInfo::from_result(
                merged_result,
                search_start,
                iteration_start,
                depth,
                hash_full,
            );
            if self.info_tx.send(info).is_err() {
                log::warn!("Info channel closed.");
            }
        }
        best_move
    }

    fn search_depth(&self, depth: i8, search_start: Instant) -> Option<SearchResult> {
        self.job.clone().search(
            depth,
            search_start,
            Arc::clone(&self.kill_switch),
            Arc::clone(&self.t_table),
        )
    }
}

/// Information about an ongoing search. One of these is created for each
/// iteration of the search.
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

/// An error indicating that a search is already running.
#[derive(Debug, thiserror::Error)]
#[error("search is running")]
pub struct SearchRunningError;
