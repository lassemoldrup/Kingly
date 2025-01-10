use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam::channel::{self, Receiver, Sender};

use crate::eval::{Eval, StandardEval};
use crate::search::SearchStats;
use crate::types::{value, Move, Value};
use crate::MoveGen;

use super::trace::{EmptyObserver, SearchObserver};
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

/// A thread pool for running search jobs. Dropping the thread pool will stop
/// the currently running search, and block until it finishes.
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
pub struct ThreadPool<E = StandardEval, O = EmptyObserver> {
    runner_thread: Option<std::thread::JoinHandle<SearchResult>>,
    worker_threads: Vec<std::thread::JoinHandle<()>>,
    worker_txs: Arc<[Sender<WorkerJob<E, O>>]>,
    result_rx: Receiver<SearchResult>,
    result_tx: Sender<SearchResult>,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
    num_threads: usize,
}

impl<E, O> ThreadPool<E, O> {
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
}

impl<E, O> ThreadPool<E, O>
where
    E: Eval + 'static,
    O: SearchObserver + Clone + Send + 'static,
{
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
    pub fn run(&mut self, job: SearchJob<E, O>) -> Result<InfoReceiver, SearchRunningError> {
        let (tx, rx) = info_channel();
        self.run_with_channel(job, tx)?;
        Ok(rx)
    }

    /// Try to spawn threads to run a search job using a pre-constructed
    /// channel. Returns an error if a search is already running.
    pub fn run_with_channel(
        &mut self,
        job: SearchJob<E, O>,
        info_tx: InfoSender,
    ) -> Result<(), SearchRunningError> {
        if self.is_running() {
            return Err(SearchRunningError);
        }

        let runner = ThreadedRunner::new(
            job,
            info_tx,
            Arc::clone(&self.worker_txs),
            self.result_rx.clone(),
            Arc::clone(&self.kill_switch),
            Arc::clone(&self.t_table),
            self.num_threads,
        );
        let runner_thread = std::thread::spawn(move || runner.run());
        self.runner_thread = Some(runner_thread);
        Ok(())
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

impl<E, O> Drop for ThreadPool<E, O> {
    fn drop(&mut self) {
        self.stop();
    }
}

impl Default for ThreadPool {
    fn default() -> Self {
        Self::new()
    }
}

struct ThreadedRunner<E, O> {
    job: SearchJob<E, O>,
    info_tx: InfoSender,
    worker_txs: Arc<[Sender<WorkerJob<E, O>>]>,
    result_rx: Receiver<SearchResult>,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
    num_threads: usize,
    search_start: Instant,
    result: SearchResult,
    nodes: Vec<u64>,
}

impl<E, O> ThreadedRunner<E, O>
where
    E: Eval,
    O: SearchObserver + Clone + Send + 'static,
{
    fn new(
        job: SearchJob<E, O>,
        info_tx: InfoSender,
        worker_txs: Arc<[Sender<WorkerJob<E, O>>]>,
        result_rx: Receiver<SearchResult>,
        kill_switch: Arc<AtomicBool>,
        t_table: Arc<TranspositionTable>,
        num_threads: usize,
    ) -> Self {
        Self {
            job,
            info_tx,
            worker_txs,
            result_rx,
            kill_switch,
            t_table,
            num_threads,
            search_start: Instant::now(),
            result: SearchResult::default(),
            nodes: vec![0; num_threads],
        }
    }

    fn run(mut self) -> SearchResult {
        // TODO: Pretty print
        log::info!("Starting search with {:?}", self.job.limits);
        let max_depth = self.job.limits.depth.unwrap_or(i8::MAX);

        // Iterative deepening
        for depth in 1..=max_depth {
            log::trace!("Starting iteration with depth {depth}");
            let iteration_start = Instant::now();
            self.job.observer.on_depth(depth);

            // Aspiration window search
            let evaluation = if let Some(entry) = self.t_table.get(&self.job.position) {
                let mut delta = 25i32;
                let mut alpha = entry.score - Value::centipawn(delta as i16);
                let mut beta = entry.score + Value::centipawn(delta as i16);

                loop {
                    log::trace!("Attempting aspiration window ({alpha:?}, {beta:?})");
                    self.job.observer.on_aspiration_window(alpha, beta);
                    let evaluation = self.search_threaded(depth, alpha, beta);
                    if let Some(e) = evaluation {
                        delta *= 4;
                        if e.score <= alpha {
                            log::trace!("Fail low: {:?}", e.score);
                            if alpha == value::NEG_INF {
                                break Some(e);
                            }
                            alpha =
                                Value::from_i32_saturating(entry.score.into_inner() as i32 - delta);
                        } else if e.score >= beta {
                            log::trace!("Fail high: {:?}", e.score);
                            if beta == value::INF {
                                break Some(e);
                            }
                            beta =
                                Value::from_i32_saturating(entry.score.into_inner() as i32 + delta);
                        } else {
                            break Some(e);
                        }
                    } else {
                        break None;
                    }
                }
            } else {
                log::trace!("No t-table entry, searching with full width");
                self.search_threaded(depth, value::NEG_INF, value::INF)
            };
            // let evaluation = self.search_threaded(depth, value::NEG_INF, value::INF);

            let Some(evaluation) = evaluation else {
                break;
            };
            self.result.evaluation = Some(evaluation.clone());
            let hash_full = ((self.t_table.len() * 1000) / self.t_table.capacity()) as u32;
            let info = SearchInfo::new_depth(
                evaluation,
                self.result.stats,
                self.search_start,
                iteration_start,
                depth,
                hash_full,
            );
            if self.info_tx.send(info).is_err() {
                log::warn!("Info channel closed.");
            }

            // Decide if we should stop early
            if self.job.limits.allow_early_stop {
                let moves = MoveGen::init().gen_all_moves(&self.job.position);
                if moves.len() == 1 {
                    log::trace!("Only one move, stopping early.");
                    break;
                }
                if self
                    .job
                    .limits
                    .time
                    .is_some_and(|d| self.search_start.elapsed() * 2 >= d)
                {
                    log::trace!("Unlikely to finish next iteration, stopping early.");
                    break;
                }
            }
        }

        log::info!("Search finished, clearing t-table.");
        let best_move = self
            .result
            .evaluation
            .as_ref()
            .map(|r| r.pv[0])
            .unwrap_or_else(|| {
                log::warn!("No best move found, returning first move.");
                MoveGen::init().gen_all_moves(&self.job.position)[0]
            });
        let info = SearchInfo::Finished(best_move);
        if self.info_tx.send(info).is_err() {
            log::warn!("Info channel closed.");
        }

        self.t_table.clear();
        self.result
    }

    fn search_threaded(
        &mut self,
        depth: i8,
        alpha: Value,
        beta: Value,
    ) -> Option<SearchEvaluation> {
        for (i, tx) in self.worker_txs.iter().enumerate() {
            let mut search_job = self.job.clone();
            search_job.limits.depth = Some(depth);
            search_job.limits.nodes = search_job
                .limits
                .nodes
                .map(|n| n.saturating_sub(self.nodes[i]));
            search_job.worker_id = i;
            let job = WorkerJob {
                alpha,
                beta,
                search_job,
                search_start: self.search_start,
            };
            tx.send(job).expect("worker channel shouldn't close");
        }

        let mut iter_evaluation = None;
        for i in 0..self.num_threads {
            let res = self
                .result_rx
                .recv()
                .expect("result channel shouldn't close");
            if let Some(e) = res.evaluation {
                iter_evaluation = Some(e);
                self.kill_switch.store(true, Ordering::Relaxed);
            }
            self.result.stats = self.result.stats.combine(res.stats);
            self.nodes[i] += res.stats.nodes;
        }
        self.kill_switch.store(false, Ordering::Relaxed);
        iter_evaluation
    }
}

struct WorkerJob<E, O> {
    alpha: Value,
    beta: Value,
    search_job: SearchJob<E, O>,
    search_start: Instant,
}

fn worker<E, O>(
    job_rx: Receiver<WorkerJob<E, O>>,
    result_tx: Sender<SearchResult>,
    kill_switch: Arc<AtomicBool>,
    t_table: Arc<TranspositionTable>,
    id: usize,
) where
    E: Eval,
    O: SearchObserver,
{
    while let Ok(job) = job_rx.recv() {
        let res = job.search_job.search(
            job.alpha,
            job.beta,
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
