use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam::channel::{self, Receiver, Sender};

use crate::search::SearchStats;
use crate::types::Move;
use crate::MoveGen;

use super::{SearchEvaluation, SearchJob, SearchResult, TranspositionTable};

/// The sending side of a channel that sends search info.
pub type InfoSender = Sender<SearchInfo>;
/// The receiving side of a channel that sends search info.
pub type InfoReceiver = Receiver<SearchInfo>;
/// Create a new channel for sending search info.
pub fn info_channel() -> (InfoSender, InfoReceiver) {
    channel::unbounded()
}

// Right now it seems that more than 6 threads is detrimental.
/// The default number of threads to use for the search.
pub const DEFAULT_THREADS: usize = 6;

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
/// let rx = thread_pool.run(job).expect("search is not running");
/// let res = thread_pool.wait().expect("search is running");
/// assert!(res.evaluation.is_some());
/// ```
pub struct ThreadPool {
    runner_thread: Option<std::thread::JoinHandle<SearchResult>>,
    worker_threads: Vec<std::thread::JoinHandle<()>>,
    worker_txs: Arc<[Sender<WorkerJob>]>,
    result_rx: Receiver<SearchResult>,
    result_tx: Sender<SearchResult>,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
    num_threads: usize,
}

impl ThreadPool {
    /// Create a new thread pool, spawning threads.
    pub fn new() -> Self {
        let num_threads = num_cpus::get().min(DEFAULT_THREADS);
        let kill_switch = Arc::new(AtomicBool::new(false));
        let t_table = Arc::new(TranspositionTable::new());
        let (result_tx, result_rx) = channel::unbounded();
        let mut worker_txs = Vec::with_capacity(num_threads);
        let mut worker_threads = Vec::with_capacity(num_threads);
        for id in 0..num_threads {
            let (job_tx, job_rx) = channel::unbounded();
            worker_txs.push(job_tx);
            let result_tx = result_tx.clone();
            let kill_switch = Arc::clone(&kill_switch);
            let t_table = Arc::clone(&t_table);
            let worker =
                std::thread::spawn(move || worker(job_rx, result_tx, kill_switch, t_table, id));
            worker_threads.push(worker);
        }
        Self {
            runner_thread: None,
            worker_threads,
            worker_txs: worker_txs.into(),
            result_rx,
            result_tx,
            kill_switch,
            t_table,
            num_threads,
        }
    }

    /// Try to spawn threads to run a search job. Returns an error if a search
    /// is already running.
    pub fn run(&mut self, job: SearchJob) -> Result<InfoReceiver, SearchRunningError> {
        let (tx, rx) = info_channel();
        self.run_with_channel(job, tx)?;
        Ok(rx)
    }

    /// Try to spawn threads to run a search job using a pre-constructed
    /// channel. Returns an error if a search is already running.
    pub fn run_with_channel(
        &mut self,
        job: SearchJob,
        info_tx: InfoSender,
    ) -> Result<(), SearchRunningError> {
        if self.is_running() {
            return Err(SearchRunningError);
        }

        let worker_txs = Arc::clone(&self.worker_txs);
        let result_rx = self.result_rx.clone();
        let kill_switch = Arc::clone(&self.kill_switch);
        let t_table = Arc::clone(&self.t_table);
        let num_threads = self.num_threads;
        let runner = std::thread::spawn(move || {
            job.search_threaded(
                info_tx,
                worker_txs,
                result_rx,
                kill_switch,
                t_table,
                num_threads,
            )
        });
        self.runner_thread = Some(runner);
        Ok(())
    }

    /// Stop the currently running job and return the result of the search.
    /// Returns `None` if no search is running.
    pub fn stop(&mut self) -> Option<SearchResult> {
        self.kill_switch.store(true, Ordering::Relaxed);
        let res = self.wait();
        self.kill_switch.store(false, Ordering::Relaxed);
        res
    }

    /// Wait for the currently running job to finish and return the result of
    /// the search. Returns `None` if no search is running.
    pub fn wait(&mut self) -> Option<SearchResult> {
        self.runner_thread
            .take()
            .map(|h| h.join().expect("runner thread shouldn't panic"))
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

    /// Sets the number of threads to use for the search. This can only be done
    /// when no search is running. Returns an error if a search is running.
    pub fn set_num_threads(&mut self, num_threads: usize) -> Result<(), SearchRunningError> {
        assert!(num_threads > 0);
        if self.is_running() {
            return Err(SearchRunningError);
        }

        if num_threads <= self.num_threads {
            self.worker_txs = self.worker_txs[..num_threads].into();
            self.worker_threads.truncate(num_threads);
        } else {
            let mut worker_txs = self.worker_txs.to_vec();
            for id in self.num_threads..num_threads {
                let (job_tx, job_rx) = channel::unbounded();
                worker_txs.push(job_tx);
                let result_tx = self.result_tx.clone();
                let kill_switch = Arc::clone(&self.kill_switch);
                let t_table = Arc::clone(&self.t_table);
                let worker =
                    std::thread::spawn(move || worker(job_rx, result_tx, kill_switch, t_table, id));
                self.worker_threads.push(worker);
            }
            self.worker_txs = worker_txs.into();
        }

        self.num_threads = num_threads;
        Ok(())
    }

    /// Returns true if a search is currently running.
    pub fn is_running(&self) -> bool {
        self.runner_thread
            .as_ref()
            .map(|h| !h.is_finished())
            .unwrap_or(false)
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.stop();
    }
}

impl SearchJob {
    fn search_threaded(
        self,
        info_tx: InfoSender,
        worker_txs: Arc<[Sender<WorkerJob>]>,
        result_rx: Receiver<SearchResult>,
        kill_switch: Arc<AtomicBool>,
        t_table: Arc<TranspositionTable>,
        num_threads: usize,
    ) -> SearchResult {
        let search_start = Instant::now();

        log::info!("Starting search with {:?}", self.limits);
        let max_depth = self.limits.depth.unwrap_or(i8::MAX);

        let mut result = SearchResult::default();
        let mut nodes = vec![0; num_threads];
        // Iterative deepening
        for depth in 1..=max_depth {
            let iteration_start = Instant::now();

            for (i, tx) in worker_txs.iter().enumerate() {
                let mut search_job = self.clone();
                search_job.limits.depth = Some(depth);
                search_job.limits.nodes =
                    search_job.limits.nodes.map(|n| n.saturating_sub(nodes[i]));
                let job = WorkerJob {
                    search_job,
                    search_start,
                };
                tx.send(job).expect("worker channel shouldn't close");
            }

            let mut iter_evaluation = None;
            for i in 0..num_threads {
                let res = result_rx.recv().expect("result channel shouldn't close");
                if let Some(e) = res.evaluation {
                    iter_evaluation = Some(e);
                    kill_switch.store(true, Ordering::Relaxed);
                }
                result.stats = result.stats.combine(res.stats);
                nodes[i] += res.stats.nodes;
            }
            kill_switch.store(false, Ordering::Relaxed);
            let Some(iter_evaulation) = iter_evaluation else {
                break;
            };
            result.evaluation = Some(iter_evaulation.clone());

            let hash_full = ((t_table.len() * 1000) / t_table.capacity()) as u32;
            let info = SearchInfo::new_depth(
                iter_evaulation,
                result.stats,
                search_start,
                iteration_start,
                depth,
                hash_full,
            );
            if info_tx.send(info).is_err() {
                log::warn!("Info channel closed.");
            }
        }

        log::info!("Search finished, clearing t-table.");
        let best_move = result
            .evaluation
            .as_ref()
            .map(|r| r.pv[0])
            .unwrap_or_else(|| {
                log::warn!("No best move found, returning first move.");
                MoveGen::init().gen_all_moves(&self.position)[0]
            });
        let info = SearchInfo::Finished(best_move);
        if info_tx.send(info).is_err() {
            log::warn!("Info channel closed.");
        }

        t_table.clear();

        result
    }
}

struct WorkerJob {
    search_job: SearchJob,
    search_start: Instant,
}

fn worker(
    job_rx: Receiver<WorkerJob>,
    result_tx: Sender<SearchResult>,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
    id: usize,
) {
    while let Ok(job) = job_rx.recv() {
        let res = job.search_job.search(
            job.search_start,
            Arc::clone(&kill_switch),
            Arc::clone(&t_table),
        );
        let Ok(()) = result_tx.send(res) else {
            log::info!("Result channel closed, stopping worker {id}.");
            return;
        };
    }
    log::info!("Job channel closed, stopping worker {id}.");
}

/// Information about an ongoing search. One of these is created for each
/// iteration of the search, and one is created at the end of the search.
#[derive(Debug, Clone)]
pub enum SearchInfo {
    NewDepth {
        /// The depth of the last completed iteration.
        depth: i8,
        /// The search evaluation of the last completed iteration.
        evaluation: SearchEvaluation,
        /// The stats of the search so far
        stats: SearchStats,
        /// The number of nodes per second searched in the last iteration.
        nps: u64,
        /// The total duration of the search so far.
        total_duration: Duration,
        /// The fullness of the hash table as a per mille value.
        hash_full: u32,
    },
    /// The best move found by the search.
    Finished(Move),
}

impl SearchInfo {
    fn new_depth(
        evaluation: SearchEvaluation,
        stats: SearchStats,
        search_start: Instant,
        iteration_start: Instant,
        depth: i8,
        hash_full: u32,
    ) -> Self {
        let total_duration = search_start.elapsed();
        let elapsed_nanos = iteration_start.elapsed().as_nanos();
        let nps = (stats.nodes as u128 * 1_000_000_000 / elapsed_nanos) as u64;
        Self::NewDepth {
            depth,
            evaluation,
            stats,
            nps,
            total_duration,
            hash_full,
        }
    }
}

/// An error indicating that a search is already running.
#[derive(Debug, thiserror::Error)]
#[error("search is running")]
pub struct SearchRunningError;
