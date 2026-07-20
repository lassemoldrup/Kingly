#![feature(portable_simd)]

use std::io::{self, Write};
use std::simd::cmp::SimdPartialEq;
use std::simd::num::{SimdInt, SimdUint};
use std::simd::{Select, Simd};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::Instant;

use clap::{Parser, Subcommand};
use crossbeam::deque::{Injector, Stealer, Worker};
use kingly_lib::types::{Bitboard, BoardVector, File, Rank, Square};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

/// The largest supported index width; tables are sized for this.
const MAX_MAGIC_BITS: usize = 13;
const LANES: usize = 4;
const LANES_U32: usize = 2 * LANES;

type SimdU64 = Simd<u64, LANES>;
type SimdUsize = Simd<usize, LANES>;
type SimdU32 = Simd<u32, LANES_U32>;

#[derive(Parser)]
struct App {
    /// The square to find rook magics for, e.g. "a1" or "e4".
    #[arg(long, global = true, default_value = "a1")]
    square: Square,
    /// The index bits of the magic to find. Defaults to the premask size
    /// (injective, no compression), except for `climb` which defaults to one
    /// less (compression by one bit).
    #[arg(long, global = true)]
    shift: Option<u32>,
    #[clap(subcommand)]
    command: Command,
}

/// Flags shaping how candidate magics are sampled; shared between `random`
/// (the search candidates) and `climb` (the seed candidates).
#[derive(clap::Args)]
struct SamplerArgs {
    /// The number of extra random bits sprinkled into the index windows.
    #[clap(long, default_value_t = 0, conflicts_with = "dense")]
    extra_bits: u32,
    /// Sample dense candidates with a per-bit density gradient across the
    /// live span instead of sparse window-based bits. Compressing squares
    /// need the carry chains that dense magics provide.
    #[clap(long)]
    dense: bool,
    /// The bit density at bit 0 with --dense.
    #[clap(
        long,
        default_value_t = 0.5,
        requires = "dense",
        conflicts_with = "density"
    )]
    density_low: f64,
    /// The bit density at the highest live bit with --dense.
    #[clap(
        long,
        default_value_t = 0.5,
        requires = "dense",
        conflicts_with = "density"
    )]
    density_high: f64,
    // The bit density across the live span with --dense.
    #[clap(long, requires = "dense")]
    density: Option<f64>,
    /// A hex mask of bits to force on in every candidate (a la Witek's
    /// per-square scaffolds).
    #[clap(long, value_parser = parse_hex, default_value = "0")]
    pin_ones: u64,
    /// A hex mask of bits to force off in every candidate; pinned ones win
    /// over pinned zeros.
    #[clap(long, value_parser = parse_hex, default_value = "0")]
    pin_zeros: u64,
}

impl SamplerArgs {
    fn style(&self) -> SampleStyle {
        if self.dense {
            let (density_low, density_high) = if let Some(density) = self.density {
                (density, density)
            } else {
                (self.density_low, self.density_high)
            };
            SampleStyle::Dense {
                density_low: density_low.clamp(0.0, 1.0),
                density_high: density_high.clamp(0.0, 1.0),
                pin_ones: self.pin_ones,
                pin_zeros: self.pin_zeros,
            }
        } else {
            SampleStyle::Sparse {
                extra_bits: self.extra_bits,
                pin_ones: self.pin_ones,
                pin_zeros: self.pin_zeros,
            }
        }
    }
}

#[derive(Subcommand)]
enum Command {
    /// Randomly sample magics across all cores until one works, shaping each
    /// candidate around the index-window structure (see `Sampler`).
    Random {
        #[clap(flatten)]
        sampler: SamplerArgs,
        #[clap(long)]
        single_threaded: bool,
    },
    /// Exhaustively search magics with at most `max_bits` bits set.
    Dfs {
        #[clap(long, default_value_t = 8)]
        max_bits: u32,
        #[clap(long)]
        single_threaded: bool,
    },
    /// Hill-climb from seed magics found at a higher (easier) index width
    /// toward a reduced-bit magic. Every valid `bits`-bit magic is also valid
    /// at `bits + 1`, so the seeds live in a provable superset of the target.
    /// The sampler flags configure the seed search.
    Climb {
        #[clap(flatten)]
        sampler: SamplerArgs,
        /// The index width to find seeds at; defaults to one more than the
        /// target bits.
        #[clap(long)]
        seed_bits: Option<u32>,
        /// Reject mutations that break seed-width validity, confining the
        /// walk to the superset that provably contains every target magic.
        #[clap(long)]
        ridge: bool,
        /// Evaluations without improvement before restarting from a fresh
        /// seed.
        #[clap(long, default_value_t = 100_000)]
        stagnation: u64,
        /// The number of bits to flip when restarting from a pool elite
        /// (basin hopping).
        #[clap(long, default_value_t = 8)]
        kick_flips: u32,
        /// The number of elite local optima kept for restarts and crossover.
        #[clap(long, default_value_t = 8)]
        elites: usize,
        /// Initial simulated-annealing temperature, in occupancies. 0 is pure
        /// hill climbing. A move `d` occupancies worse is accepted with
        /// probability `exp(-d / T)`; `T` cools geometrically each evaluation
        /// and resets on restart.
        #[clap(long, default_value_t = 0.0)]
        temperature: f64,
        /// The geometric factor applied to the temperature each evaluation.
        #[clap(long, default_value_t = 0.9999)]
        cooling: f64,
        /// Probability that a mutation flip may touch a pinned bit, escaping
        /// the scaffold. 0 keeps the pins fixed for the whole climb.
        #[clap(long, default_value_t = 0.2)]
        pin_break: f64,
        #[clap(long)]
        single_threaded: bool,
    },
}

fn main() {
    let app = App::parse();
    let occ_sets = rook_occupancy_sets(app.square);
    let natural_bits = relevant_occupancy_squares(app.square).len() as u32;

    match app.command {
        Command::Random {
            sampler,
            single_threaded,
        } => {
            let bits = resolve_bits(app.shift, natural_bits);
            eprintln!("searching {bits}-bit magics for {}", app.square);
            let precheck = Precheck::new(app.square, bits);
            let sampler = Sampler::new(app.square, sampler.style(), bits);
            run_random(&occ_sets, &precheck, &sampler, single_threaded)
        }
        Command::Dfs {
            max_bits,
            single_threaded,
        } => {
            let bits = resolve_bits(app.shift, natural_bits);
            eprintln!("searching {bits}-bit magics for {}", app.square);
            let precheck = Precheck::new(app.square, bits);
            run_dfs(&occ_sets, &precheck, max_bits, single_threaded)
        }
        Command::Climb {
            sampler,
            seed_bits,
            ridge,
            stagnation,
            kick_flips,
            elites,
            temperature,
            cooling,
            pin_break,
            single_threaded,
        } => {
            let bits = resolve_bits(app.shift, natural_bits.saturating_sub(1));
            let seed_bits = resolve_bits(seed_bits, natural_bits);
            assert!(
                seed_bits > bits,
                "seed bits ({seed_bits}) must exceed target bits ({bits})"
            );
            eprintln!(
                "climbing from {seed_bits}-bit seeds toward {bits}-bit magics for {}{}",
                app.square,
                if ridge { " (ridge mode)" } else { "" },
            );
            run_climb(
                &occ_sets,
                app.square,
                bits,
                seed_bits,
                sampler.style(),
                ridge,
                stagnation,
                kick_flips,
                elites.max(1),
                Anneal {
                    temperature: temperature.max(0.0),
                    cooling: cooling.clamp(0.0, 1.0),
                },
                pin_break.clamp(0.0, 1.0),
                single_threaded,
            )
        }
    }
}

/// Resolves an optional bit-count argument against a default, validating the
/// supported range.
fn resolve_bits(bits: Option<u32>, default: u32) -> u32 {
    let bits = bits.unwrap_or(default);
    assert!(
        (1..=MAX_MAGIC_BITS as u32).contains(&bits),
        "index bits must be in [1, {MAX_MAGIC_BITS}]"
    );
    bits
}

fn run_random(
    occ_sets: &[OccupancySet],
    precheck: &Precheck,
    sampler: &Sampler,
    single_threaded: bool,
) {
    let worker_count = if single_threaded {
        1
    } else {
        thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .unwrap_or(1)
    };

    let found = AtomicBool::new(false);
    let tried_counts: Vec<AtomicU64> = (0..worker_count).map(|_| AtomicU64::new(0)).collect();
    let start = Instant::now();

    thread::scope(|scope| {
        for worker_id in 0..worker_count {
            let found = &found;
            let tried_counts = &tried_counts;
            scope.spawn(move || {
                random_worker(
                    worker_id,
                    occ_sets,
                    precheck,
                    sampler,
                    found,
                    tried_counts,
                    start,
                );
            });
        }
    });
}

fn random_worker(
    worker_id: usize,
    occ_sets: &[OccupancySet],
    precheck: &Precheck,
    sampler: &Sampler,
    found: &AtomicBool,
    tried_counts: &[AtomicU64],
    start: Instant,
) {
    let mut rng = SmallRng::from_rng(&mut rand::rng());
    let mut idxs = [usize::MAX; 1 << MAX_MAGIC_BITS];
    let is_reporter = worker_id == 0;
    let mut local_count = 0u64;

    while !found.load(Ordering::Relaxed) {
        let magic = sampler.sample(&mut rng);
        local_count += 1;
        tried_counts[worker_id].fetch_add(1, Ordering::Relaxed);

        if is_magic(magic, occ_sets, precheck, &mut idxs) {
            if !found.swap(true, Ordering::Relaxed) {
                eprintln!("\nFound magic: {:#x}", magic);
            }
            return;
        }

        if is_reporter && local_count % 1_000_000 == 0 {
            let count = tried_counts
                .iter()
                .map(|counter| counter.load(Ordering::Relaxed))
                .sum();
            report(count, start);
        }
    }
}

fn run_dfs(occ_sets: &[OccupancySet], precheck: &Precheck, max_bits: u32, single_threaded: bool) {
    let worker_count = if single_threaded {
        1
    } else {
        thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .unwrap_or(1)
    };

    // The frontier starts as a single node (the empty magic) in the shared
    // injector; the first idle worker picks it up and fans it out locally.
    let injector = Injector::new();
    injector.push(0u64);
    let deques: Vec<Worker<u64>> = (0..worker_count).map(|_| Worker::new_lifo()).collect();
    let stealers: Vec<Stealer<u64>> = deques.iter().map(|deque| deque.stealer()).collect();

    let found = AtomicBool::new(false);
    // Number of workers still holding work; the search is done once it hits 0.
    let active = AtomicUsize::new(worker_count);
    let tried_counts: Vec<AtomicU64> = (0..worker_count).map(|_| AtomicU64::new(0)).collect();
    let start = Instant::now();

    thread::scope(|scope| {
        for (worker_id, local) in deques.into_iter().enumerate() {
            let injector = &injector;
            let stealers = &stealers;
            let found = &found;
            let active = &active;
            let tried_counts = &tried_counts;
            scope.spawn(move || {
                dfs_worker(
                    worker_id,
                    local,
                    injector,
                    stealers,
                    occ_sets,
                    precheck,
                    max_bits,
                    found,
                    active,
                    tried_counts,
                    start,
                );
            });
        }
    });

    if !found.load(Ordering::Relaxed) {
        eprintln!("\nExhausted search space up to {max_bits} bits, no magic found");
    }
}

#[allow(clippy::too_many_arguments)]
fn dfs_worker(
    worker_id: usize,
    local: Worker<u64>,
    injector: &Injector<u64>,
    stealers: &[Stealer<u64>],
    occ_sets: &[OccupancySet],
    precheck: &Precheck,
    max_bits: u32,
    found: &AtomicBool,
    active: &AtomicUsize,
    tried_counts: &[AtomicU64],
    start: Instant,
) {
    let mut idxs = [usize::MAX; 1 << MAX_MAGIC_BITS];
    let is_reporter = worker_id == 0;
    let mut local_count = 0u64;
    let mut is_active = true;

    loop {
        if found.load(Ordering::Relaxed) {
            break;
        }

        let Some(magic) = find_task(&local, injector, stealers) else {
            // No work right now: go idle, and only quit once every worker is
            // idle, which means the whole frontier is drained.
            if is_active {
                active.fetch_sub(1, Ordering::SeqCst);
                is_active = false;
            }
            if active.load(Ordering::SeqCst) == 0 {
                break;
            }
            thread::yield_now();
            continue;
        };

        if !is_active {
            active.fetch_add(1, Ordering::SeqCst);
            is_active = true;
        }

        local_count += 1;
        tried_counts[worker_id].fetch_add(1, Ordering::Relaxed);

        if is_magic(magic, occ_sets, precheck, &mut idxs) {
            if !found.swap(true, Ordering::Relaxed) {
                eprintln!("\nFound magic: {:#x}", magic);
            }
            break;
        }

        if magic.count_ones() < max_bits {
            // Only add higher bits so each magic is reached exactly once, and
            // push onto the local deque to keep the frontier depth-first.
            let next_bit = if magic == 0 {
                0
            } else {
                64 - magic.leading_zeros()
            };
            for bit in next_bit..64 {
                local.push(magic | (1 << bit));
            }
        }

        if is_reporter && local_count % 1_000_000 == 0 {
            let count = tried_counts
                .iter()
                .map(|counter| counter.load(Ordering::Relaxed))
                .sum();
            report(count, start);
        }
    }
}

fn find_task(
    local: &Worker<u64>,
    injector: &Injector<u64>,
    stealers: &[Stealer<u64>],
) -> Option<u64> {
    // Fast path: our own deque. Otherwise pull a batch from the injector or
    // steal one from a peer, retrying while any source reports contention.
    local.pop().or_else(|| {
        std::iter::repeat_with(|| {
            injector.steal_batch_and_pop(local).or_else(|| {
                stealers
                    .iter()
                    .map(|stealer| stealer.steal_batch_and_pop(local))
                    .collect()
            })
        })
        .find(|steal| !steal.is_retry())
        .and_then(|steal| steal.success())
    })
}

/// A shared pool of the best distinct local optima found by the climbers,
/// used as restart material: perturbed for basin hopping and recombined by
/// crossover. Different local optima tend to solve different subsets of the
/// conflicts, so mixing them can combine their strengths.
struct ElitePool {
    /// Entries sorted by descending score, capped at `capacity`.
    entries: Mutex<Vec<(u64, usize)>>,
    capacity: usize,
}

impl ElitePool {
    fn new(capacity: usize) -> Self {
        Self {
            entries: Mutex::new(Vec::with_capacity(capacity + 1)),
            capacity,
        }
    }

    /// Inserts `(magic, score)`, keeping the pool sorted and capped. Returns
    /// `true` if it became the new top entry (a new global best).
    fn insert(&self, magic: u64, score: usize) -> bool {
        let mut entries = self.entries.lock().unwrap();
        if entries.len() >= self.capacity && score <= entries.last().unwrap().1 {
            return false;
        }
        if let Some(i) = entries
            .iter()
            .position(|&(m, _)| (m ^ magic).count_ones() <= 4)
        {
            // Keep the pool diverse.
            if score <= entries[i].1 {
                return false;
            }
            // Replace the old entry with the new one, which is better and similar.
            entries.remove(i);
        }
        let pos = entries.partition_point(|&(_, s)| s >= score);
        entries.insert(pos, (magic, score));
        entries.truncate(self.capacity);
        pos == 0
    }

    /// A random elite magic, if any.
    fn sample(&self, rng: &mut impl Rng) -> Option<u64> {
        let entries = self.entries.lock().unwrap();
        (!entries.is_empty()).then(|| entries[rng.random_range(0..entries.len())].0)
    }

    /// Two distinct random elite magics, if at least two exist.
    fn pair(&self, rng: &mut impl Rng) -> Option<(u64, u64)> {
        let entries = self.entries.lock().unwrap();
        if entries.len() < 2 {
            return None;
        }
        let i = rng.random_range(0..entries.len());
        let mut j = rng.random_range(0..entries.len() - 1);
        if j >= i {
            j += 1;
        }
        Some((entries[i].0, entries[j].0))
    }
}

/// Uniform crossover: each bit is taken from `a` or `b` at random. Where the
/// parents agree the child agrees, so only the bits on which they differ are
/// randomized.
fn crossover(a: u64, b: u64, rng: &mut impl Rng) -> u64 {
    let mask = rng.next_u64();
    (a & mask) | (b & !mask)
}

/// Simulated-annealing schedule for the climb's acceptance rule.
#[derive(Clone, Copy)]
struct Anneal {
    temperature: f64,
    cooling: f64,
}

impl Anneal {
    /// The lowest score to accept at temperature `t` given the current score.
    ///
    /// Annealing accepts a move `d` worse with probability `exp(-d / t)`.
    /// Drawing the acceptance random `u` up front turns that into a threshold:
    /// accept iff `candidate >= score + t * ln(u)` (with `ln(u) <= 0`). The
    /// threshold doubles as the evaluation's lower bound, so a worse move is
    /// scored exactly when it will be accepted and abandoned early otherwise.
    fn accept_floor(&self, score: usize, t: f64, rng: &mut impl Rng) -> usize {
        if t <= 0.0 {
            return score; // Pure hill climbing: only sideways-or-better moves.
        }
        let u = 1.0 - rng.random_range(0.0f64..1.0); // (0, 1]
        (score as f64 + t * u.ln()).max(0.0) as usize
    }
}

#[allow(clippy::too_many_arguments)]
fn run_climb(
    occ_sets: &[OccupancySet],
    sq: Square,
    bits: u32,
    seed_bits: u32,
    seed_style: SampleStyle,
    ridge: bool,
    stagnation: u64,
    kick_flips: u32,
    elites: usize,
    anneal: Anneal,
    pin_break: f64,
    single_threaded: bool,
) {
    let worker_count = if single_threaded {
        1
    } else {
        thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .unwrap_or(1)
    };

    let pinned = seed_style.pinned();
    let target_precheck = Precheck::new(sq, bits);
    let seed_precheck = Precheck::new(sq, seed_bits);
    let seed_sampler = Sampler::new(sq, seed_style, seed_bits);
    let (_, span_high) = live_span(sq, bits);

    let found = AtomicBool::new(false);
    let pool = ElitePool::new(elites);
    let best_score = AtomicU64::new(0);
    let tried_counts: Vec<AtomicU64> = (0..worker_count).map(|_| AtomicU64::new(0)).collect();
    let start = Instant::now();

    thread::scope(|scope| {
        for worker_id in 0..worker_count {
            let target_precheck = &target_precheck;
            let seed_precheck = &seed_precheck;
            let seed_sampler = &seed_sampler;
            let found = &found;
            let pool = &pool;
            let best_score = &best_score;
            let tried_counts = &tried_counts;
            scope.spawn(move || {
                climb_worker(ClimbWorker {
                    worker_id,
                    occ_sets,
                    target_precheck,
                    seed_precheck,
                    seed_sampler,
                    span_high,
                    ridge,
                    stagnation,
                    kick_flips,
                    anneal,
                    pinned,
                    pin_break,
                    found,
                    pool,
                    best_score,
                    tried_counts,
                    start,
                });
            });
        }
    });
}

struct ClimbWorker<'a> {
    worker_id: usize,
    occ_sets: &'a [OccupancySet],
    target_precheck: &'a Precheck,
    seed_precheck: &'a Precheck,
    seed_sampler: &'a Sampler,
    /// The top of the target-width live span; mutations stay within
    /// `[0, span_high]`.
    span_high: u32,
    ridge: bool,
    stagnation: u64,
    /// The number of bits to flip in a pool elite when basin-hopping.
    kick_flips: u32,
    anneal: Anneal,
    /// Bit positions the pins fix; mutations preserve them (mostly).
    pinned: u64,
    /// Probability a mutation flip may touch a pinned bit.
    pin_break: f64,
    found: &'a AtomicBool,
    pool: &'a ElitePool,
    /// The best score seen across workers, for live progress reporting.
    best_score: &'a AtomicU64,
    tried_counts: &'a [AtomicU64],
    start: Instant,
}

fn climb_worker(w: ClimbWorker) {
    let mut rng = SmallRng::from_rng(&mut rand::rng());
    let mut idxs = [usize::MAX; 1 << MAX_MAGIC_BITS];
    let is_reporter = w.worker_id == 0;
    let mut local_count = 0u64;
    let total = w.occ_sets.len() * LANES;

    let count_eval = |local_count: &mut u64| {
        *local_count += 1;
        w.tried_counts[w.worker_id].fetch_add(1, Ordering::Relaxed);
        if is_reporter && *local_count % 100_000 == 0 {
            let count = w
                .tried_counts
                .iter()
                .map(|counter| counter.load(Ordering::Relaxed))
                .sum();
            report(count, w.start);
        }
    };

    while !w.found.load(Ordering::Relaxed) {
        // Pick a restart point: ~40% crossover of two elites, ~40% a perturbed
        // elite (basin hopping), ~20% a fresh seed for diversity. Either pool
        // draw falls back to a fresh seed when the pool is too small.
        let roll = rng.random_range(0..10);
        let restart = if roll < 4 {
            w.pool
                .pair(&mut rng)
                .map(|(a, b)| crossover(a, b, &mut rng))
        } else if roll < 8 {
            w.pool.sample(&mut rng).map(|elite| {
                mutate(
                    elite,
                    w.kick_flips,
                    w.span_high,
                    w.pinned,
                    w.pin_break,
                    &mut rng,
                )
            })
        } else {
            None
        };
        let mut current = match restart {
            Some(magic) => magic,
            None => {
                // Stage A: find a seed that is valid at the (easier) seed width.
                loop {
                    if w.found.load(Ordering::Relaxed) {
                        return;
                    }
                    let candidate = w.seed_sampler.sample(&mut rng);
                    count_eval(&mut local_count);
                    if is_magic(candidate, w.occ_sets, w.seed_precheck, &mut idxs) {
                        break candidate;
                    }
                }
            }
        };
        let mut score = score_magic(current, w.occ_sets, w.target_precheck, &mut idxs, 0);
        // With annealing the walk can wander downhill, so track the run's best
        // separately; without it `run_best == score` throughout.
        let mut run_best = score;
        let mut run_best_magic = current;
        let mut temperature = w.anneal.temperature;

        // Stage B: climb at the target width until stagnation.
        let mut since_improvement = 0;
        while since_improvement < w.stagnation {
            if w.found.load(Ordering::Relaxed) {
                return;
            }

            let candidate = mutate(
                current,
                mutation_size(score, total),
                w.span_high,
                w.pinned,
                w.pin_break,
                &mut rng,
            );
            count_eval(&mut local_count);
            // Accept anything at or above the annealing floor (equal to the
            // current score without annealing). The floor doubles as the
            // evaluation's lower bound: an accepted move is scored exactly, a
            // rejected one abandoned early.
            let floor = w.anneal.accept_floor(score, temperature, &mut rng);
            temperature *= w.anneal.cooling;
            let candidate_score =
                score_magic(candidate, w.occ_sets, w.target_precheck, &mut idxs, floor);

            if candidate_score == total {
                // A valid target-width magic is automatically valid at the
                // seed width, so ridge mode needs no extra check here.
                if !w.found.swap(true, Ordering::Relaxed) {
                    eprintln!("\nFound magic: {candidate:#x}");
                }
                return;
            }

            if candidate_score >= floor {
                // In ridge mode only accept moves that stay valid at the seed
                // width, where every target-width magic provably lives.
                if w.ridge && !is_magic(candidate, w.occ_sets, w.seed_precheck, &mut idxs) {
                    since_improvement += 1;
                    continue;
                }
                current = candidate;
                score = candidate_score;

                if candidate_score > run_best {
                    since_improvement = 0;
                    run_best = candidate_score;
                    run_best_magic = candidate;
                    // Live progress: report as soon as a global best is beaten,
                    // not only when the run finally stagnates.
                    let previous = w
                        .best_score
                        .fetch_max(candidate_score as u64, Ordering::Relaxed);
                    if candidate_score as u64 > previous {
                        eprintln!("\nnew best {candidate_score}/{total}: {candidate:#x}");
                    }
                } else {
                    since_improvement += 1;
                }
            } else {
                since_improvement += 1;
            }
        }

        // Keep the run's best (not the possibly-downhill final `current`) as
        // restart material for basin hopping and crossover.
        w.pool.insert(run_best_magic, run_best);
    }
}

/// The number of bits to flip, scaled to fitness: careful steps near the
/// summit, big jumps in the flatlands.
fn mutation_size(score: usize, total: usize) -> u32 {
    if 25 * score >= 24 * total {
        1
    } else if 9 * score >= 8 * total {
        2
    } else if 3 * score >= 2 * total {
        3
    } else {
        4
    }
}

/// Flips `flips` random bits of `magic` within `[0, span_high]`. Pinned bit
/// positions are normally left untouched to preserve a scaffold, but each flip
/// ignores the pins with probability `pin_break`.
fn mutate(
    magic: u64,
    flips: u32,
    span_high: u32,
    pinned: u64,
    pin_break: f64,
    rng: &mut impl Rng,
) -> u64 {
    let span_mask = u64::MAX >> (63 - span_high);
    let free_mask = span_mask & !pinned;
    let mut mutated = magic;
    for _ in 0..flips {
        // Restrict to non-pinned bits, except with probability `pin_break`
        // (or always, if every span bit is pinned).
        let mask = if free_mask == 0 || rng.random_range(0.0f64..1.0) < pin_break {
            span_mask
        } else {
            free_mask
        };
        mutated ^= 1 << random_set_bit_u64(rng, mask);
    }
    mutated
}

/// Picks a uniformly random set bit of `mask` and returns its index.
fn random_set_bit_u64(rng: &mut impl Rng, mask: u64) -> u32 {
    let n = rng.random_range(0..mask.count_ones());
    let mut remaining = mask;
    for _ in 0..n {
        remaining &= remaining - 1;
    }
    remaining.trailing_zeros()
}

fn report(tried: u64, start: Instant) {
    let elapsed = start.elapsed().as_secs_f64();
    let rate = if elapsed > 0.0 {
        tried as f64 / elapsed
    } else {
        0.0
    };
    print!("\rTried {tried} magics in {elapsed:.2} seconds ({rate:.2} magics/sec)");
    io::stdout().flush().ok();
}

/// Whether `magic` maps every occupancy consistently, breaking at the first
/// conflicting chunk.
fn is_magic(
    magic: u64,
    occ_sets: &[OccupancySet],
    precheck: &Precheck,
    idxs: &mut [usize],
) -> bool {
    const EMPTY: SimdUsize = SimdUsize::splat(usize::MAX);
    let shift = SimdU64::splat(64 - u64::from(precheck.bits));

    if !precheck.passes(magic) {
        return false;
    }

    let magic = SimdU64::splat(magic);
    let table = &mut idxs[..1 << precheck.bits];
    table.fill(usize::MAX);

    for occ in occ_sets {
        let keys = ((occ.bitboards * magic) >> shift).cast::<usize>();
        let atk_sets = Simd::gather_or_default(table, keys);
        // All non-empty entries in `atk_sets` must be equal to the attack set
        // index for this occupancy set.
        let hits_mask = atk_sets.simd_eq(EMPTY) | atk_sets.simd_eq(occ.attack_set_indices);
        if !hits_mask.all() {
            return false;
        }
        occ.attack_set_indices.scatter(table, keys);
    }
    true
}

/// Scores `magic` by the number of occupancies (lanes) it maps consistently,
/// greedily in order, committing the consistent lanes of conflicting chunks.
/// Aborts once the score provably cannot reach `min_score`, returning the
/// partial count (which is then below `min_score` by construction).
fn score_magic(
    magic: u64,
    occ_sets: &[OccupancySet],
    precheck: &Precheck,
    idxs: &mut [usize],
    min_score: usize,
) -> usize {
    const EMPTY: SimdUsize = SimdUsize::splat(usize::MAX);
    /// The number of chunks between bound checks, keeping horizontal
    /// reductions off the hot loop.
    const BOUND_INTERVAL: usize = 32;

    let shift = SimdU64::splat(64 - u64::from(precheck.bits));

    if !precheck.passes(magic) {
        return 0;
    }

    let magic = SimdU64::splat(magic);
    let table = &mut idxs[..1 << precheck.bits];
    table.fill(usize::MAX);

    let total = occ_sets.len() * LANES;
    let mut hits = 0;
    let mut processed = 0;

    for block in occ_sets.chunks(BOUND_INTERVAL) {
        // Count consistent lanes branchlessly: true lanes are -1, so
        // subtracting the mask adds one per consistent lane. One vector op
        // per chunk, one horizontal reduction per block.
        let mut consistent = SimdUsize::splat(0);
        for occ in block {
            let keys = ((occ.bitboards * magic) >> shift).cast::<usize>();
            let atk_sets = Simd::gather_or_default(table, keys);
            let hits_mask = atk_sets.simd_eq(EMPTY) | atk_sets.simd_eq(occ.attack_set_indices);
            consistent -= hits_mask.to_simd().cast::<usize>();
            if hits_mask.all() {
                occ.attack_set_indices.scatter(table, keys);
            } else {
                // Commit only the consistent lanes; conflicting lanes write
                // their slots' current values back.
                hits_mask
                    .select(occ.attack_set_indices, atk_sets)
                    .scatter(table, keys);
            }
        }
        hits += consistent.reduce_sum();
        processed += block.len() * LANES;

        // Bounded evaluation: stop once `min_score` is out of reach even if
        // every remaining occupancy were consistent.
        if hits + (total - processed) < min_score {
            break;
        }
    }
    hits
}

/// A cheap necessary condition on a magic, checked before the full table test.
///
/// The empty board always hashes to index 0, and every single-blocker
/// occupancy has an attack set that differs from the empty board *and* from
/// every other single-blocker occupancy. Their indices must therefore be
/// pairwise distinct and non-zero. A single-blocker product is carry-free
/// (`(1 << k) * magic == magic << k`), so its index is the bit window
/// `[53 - k, 63 - k]` of the magic itself, which can be extracted without
/// running the table.
///
/// A window is at most `bits` wide, so it fits entirely in at least one of
/// the 32-bit halves `[0, 31]`, `[32, 63]` or `[16, 47]` of the magic (a
/// window crossing the 32-bit boundary spans at most `[19, 44]` for 13-bit
/// windows). Grouping the windows by half lets us extract them with 32-bit
/// lane shifts, twice as many per SIMD op as with 64-bit lanes.
struct Precheck {
    /// Windows within magic bits `[0, 31]`.
    lo: Bucket,
    /// Windows within magic bits `[32, 63]`.
    hi: Bucket,
    /// Windows within magic bits `[16, 47]`.
    mid: Bucket,
    /// The index width this pre-check (and its windows) is built for.
    bits: u32,
}

impl Precheck {
    fn new(sq: Square, bits: u32) -> Self {
        let shift = 64 - u64::from(bits);
        let index_window = ((1u64 << bits) - 1) << shift;

        let mut lo = Vec::new();
        let mut hi = Vec::new();
        let mut mid = Vec::new();
        for bit in relevant_occupancy_squares(sq) {
            let k = u64::from(bit);
            let mask = index_window >> k;
            // Each window gets a `(right, left)` shift pair such that its
            // index value is `((half >> right) << left) & index_mask`.
            if k > shift {
                // The window is truncated to `[0, 63 - k]`; the index has its
                // low `k - shift` bits at zero.
                debug_assert_eq!(mask & 0xFFFF_FFFF_0000_0000, 0);
                lo.push((0, (k - shift) as u32));
            } else if mask & 0xFFFF_FFFF_0000_0000 == 0 {
                lo.push(((shift - k) as u32, 0));
            } else if mask & 0x0000_0000_FFFF_FFFF == 0 {
                hi.push(((shift - k) as u32 - 32, 0));
            } else {
                debug_assert_eq!(mask & !(0xFFFF_FFFF << 16), 0);
                mid.push(((shift - k) as u32 - 16, 0));
            }
        }

        Self {
            lo: Bucket::new(lo, bits),
            hi: Bucket::new(hi, bits),
            mid: Bucket::new(mid, bits),
            bits,
        }
    }

    /// Returns false if `magic` cannot possibly be valid.
    fn passes(&self, magic: u64) -> bool {
        let lo = SimdU32::splat(magic as u32);
        let hi = SimdU32::splat((magic >> 32) as u32);
        let mid = SimdU32::splat((magic >> 16) as u32);

        // Cheap gate first: every window must be non-zero, i.e. distinct from
        // the empty board's reserved index 0. Only survivors pay for the
        // conflict check below.
        if !(self.hi.all_non_zero(hi) && self.mid.all_non_zero(mid) && self.lo.all_non_zero(lo)) {
            return false;
        }

        // Conflict check: a repeated index means two single-blocker
        // occupancies would collide despite having different attack sets.
        // With at most 12 windows, brute-force pairwise comparison beats any
        // set structure.
        let mut values = [0; 16];
        let mut count = 0;
        let all_values = (self.hi.values(hi))
            .chain(self.mid.values(mid))
            .chain(self.lo.values(lo));
        for value in all_values {
            if values[..count].contains(&value) {
                return false;
            }
            values[count] = value;
            count += 1;
        }
        true
    }
}

/// The pre-check windows contained in one 32-bit half of the magic, as
/// SIMD-chunked lane shifts.
struct Bucket {
    right_shifts: Vec<SimdU32>,
    left_shifts: Vec<SimdU32>,
    /// The number of real windows, excluding padding lanes.
    len: usize,
    /// `2^bits - 1`, splatted.
    index_mask: SimdU32,
}

impl Bucket {
    fn new(mut shifts: Vec<(u32, u32)>, bits: u32) -> Self {
        let len = shifts.len();
        // Pad to a whole number of lanes by repeating a real shift pair, so
        // the SIMD tail re-extracts an existing window. The duplicate values
        // pass the non-zero gate iff the original does and are excluded from
        // the conflict check by `len`.
        if let Some(&first) = shifts.first() {
            while shifts.len() % LANES_U32 != 0 {
                shifts.push(first);
            }
        }
        let (chunks, _rem) = shifts.as_chunks::<LANES_U32>();
        Self {
            right_shifts: chunks
                .iter()
                .map(|&chunk| SimdU32::from_array(chunk.map(|(right, _)| right)))
                .collect(),
            left_shifts: chunks
                .iter()
                .map(|&chunk| SimdU32::from_array(chunk.map(|(_, left)| left)))
                .collect(),
            len,
            index_mask: SimdU32::splat((1 << bits) - 1),
        }
    }

    /// Extracts the window index values for one chunk.
    fn extract(&self, chunk: usize, half: SimdU32) -> SimdU32 {
        ((half >> self.right_shifts[chunk]) << self.left_shifts[chunk]) & self.index_mask
    }

    fn all_non_zero(&self, half: SimdU32) -> bool {
        const ZERO: SimdU32 = SimdU32::splat(0);
        (0..self.right_shifts.len()).all(|chunk| self.extract(chunk, half).simd_ne(ZERO).all())
    }

    /// The window index values of the real (un-padded) windows.
    fn values(&self, half: SimdU32) -> impl Iterator<Item = u32> + '_ {
        (0..self.right_shifts.len())
            .flat_map(move |chunk| self.extract(chunk, half).to_array())
            .take(self.len)
    }
}

/// The index windows `[64 - bits - k, 63 - k]` of the magic (see
/// [`Precheck`]) as `(low, length)` pairs, truncated at bit 0 for blocker
/// bits above `64 - bits`.
fn index_windows(sq: Square, bits: u32) -> Vec<(u32, u32)> {
    relevant_occupancy_squares(sq)
        .into_iter()
        .map(|bit| {
            let k = u32::from(bit);
            let low = (64 - bits).saturating_sub(k);
            (low, 64 - k - low)
        })
        .collect()
}

/// How [`Sampler`] shapes candidate magics.
enum SampleStyle {
    /// Sparse window-aware candidates, suited for squares whose index does
    /// not compress the occupancies (an injective mapping exists):
    ///
    /// - Every window gets at least one bit, so no sample trivially collides
    ///   with the empty board.
    /// - A window holding a single bit at in-window offset `e` has the
    ///   power-of-two index `1 << e`, so two such windows with equal offsets
    ///   conflict. Bits are therefore placed on pairwise-distinct offsets.
    ///   Sharing one physical bit between overlapping windows stays possible
    ///   and is safe, since the offsets differ by construction.
    /// - A corner square has more windows than there are distinct offsets, so a
    ///   valid magic must hold at least two bits in some window; one randomly
    ///   chosen window always gets two.
    /// - Extra bits are drawn from the union of the windows, since bits outside
    ///   every window only act on the index through carries.
    ///
    /// `pin_ones`/`pin_zeros` force specific bits on/off (ones win): window
    /// bits and extras are never placed on pinned-zero positions, and pinned
    /// ones are added on top of the window placement.
    Sparse {
        extra_bits: u32,
        pin_ones: u64,
        pin_zeros: u64,
    },
    /// Dense candidates with a per-bit density gradient: the density
    /// interpolates linearly from `density_low` at bit 0 to `density_high` at
    /// the top of the live span. `pin_ones`/`pin_zeros` force specific bits
    /// on/off (ones win), overriding the gradient.
    ///
    /// A compressing square (more occupancies than index slots) needs shadow
    /// collisions: adding a far blocker behind a nearer one must leave the
    /// index unchanged. Carry-free products change the index by the far
    /// blocker's non-zero window contribution regardless of the other
    /// blockers, so only carry chains can cancel it conditionally — dense
    /// magics are carry fuel. Known reduced magics are ~78% dense within the
    /// live span, and the first-rank finds cluster their bits low: carries
    /// propagate upward, so density low radiates through all windows above.
    ///
    /// The gradient deliberately includes the carry-only bits *below* the
    /// live span: they can only act through carries, but that is exactly the
    /// mechanism — clearing bits 0-2 of Witek's d1 magic breaks it.
    Dense {
        density_low: f64,
        density_high: f64,
        pin_ones: u64,
        pin_zeros: u64,
    },
}

impl SampleStyle {
    /// The bit positions this style forces to a fixed value (`pin_ones` set,
    /// `pin_zeros` clear), which mutations should normally preserve.
    fn pinned(&self) -> u64 {
        let (SampleStyle::Sparse {
            pin_ones,
            pin_zeros,
            ..
        }
        | SampleStyle::Dense {
            pin_ones,
            pin_zeros,
            ..
        }) = *self;
        pin_ones | pin_zeros
    }
}

/// The number of bit-sliced threshold planes for dense sampling; densities
/// are quantized to `1 / 2^DENSITY_PLANES` steps.
const DENSITY_PLANES: usize = 8;

/// The live span `[low, high]` of the index windows: the lowest and highest
/// magic bits appearing in any window. Bits above `high` are shifted out of
/// every blocker's product and are completely inert; bits below `low` only
/// act through carries.
fn live_span(sq: Square, bits: u32) -> (u32, u32) {
    let mut union_mask = 0u64;
    for (low, len) in index_windows(sq, bits) {
        union_mask |= ((1 << len) - 1) << low;
    }
    (union_mask.trailing_zeros(), 63 - union_mask.leading_zeros())
}

/// Samples candidate magics for the random search, shaped by the structure of
/// the single-blocker index windows (see [`Precheck`] and [`SampleStyle`]).
struct Sampler {
    /// The index windows as `(low, length)` pairs, shortest first so that the
    /// greedy offset assignment in [`Self::sample_sparse`] serves the most
    /// constrained (truncated) windows before the offsets run out.
    windows: Vec<(u32, u32)>,
    /// Per window (parallel to `windows`), the offsets the sparse sampler may
    /// place a bit on: the offsets reachable by the window's length, minus
    /// positions pinned to zero.
    window_offsets: Vec<u16>,
    /// The bit positions covered by at least one window, minus positions
    /// pinned to zero.
    union_positions: Vec<u32>,
    /// Bit-sliced density thresholds for [`Self::sample_dense`]: bit `p` of
    /// `density_planes[j]` is bit `j` of position `p`'s threshold, so that
    /// position `p` is set with probability `threshold(p) / 2^DENSITY_PLANES`.
    /// Positions above the live span have threshold 0; positions below it are
    /// carry-active and take part in the gradient. All zeros for the sparse
    /// style.
    density_planes: [u64; DENSITY_PLANES],
    /// Positions with density exactly 1, which an 8-bit threshold cannot
    /// express.
    density_always: u64,
    /// The index width the candidates are aimed at.
    bits: u32,
    style: SampleStyle,
}

impl Sampler {
    fn new(sq: Square, style: SampleStyle, bits: u32) -> Self {
        let mut windows = index_windows(sq, bits);
        windows.sort_by_key(|&(_, len)| len);

        let (SampleStyle::Sparse { pin_zeros, .. } | SampleStyle::Dense { pin_zeros, .. }) = style;

        let mut union_mask = 0u64;
        for &(low, len) in &windows {
            union_mask |= ((1 << len) - 1) << low;
        }
        let union_positions: Vec<u32> = (0..64)
            .filter(|p| union_mask & !pin_zeros & (1 << p) != 0)
            .collect();

        let window_offsets = windows
            .iter()
            .map(|&(low, len)| {
                let min_offset = bits - len;
                (min_offset..bits)
                    .filter(|&offset| pin_zeros & (1 << (low + offset - min_offset)) == 0)
                    .fold(0u16, |mask, offset| mask | (1 << offset))
            })
            .collect();

        let mut density_planes = [0; DENSITY_PLANES];
        let mut density_always = 0;
        if let SampleStyle::Dense {
            density_low,
            density_high,
            pin_ones,
            pin_zeros,
        } = style
        {
            // The gradient runs from bit 0 (carry-only bits included — they
            // act through carries, which is the point of dense magics) up to
            // the top of the live span; everything above is dead.
            let (_, high) = live_span(sq, bits);
            for position in 0..=high {
                let fraction = if high == 0 {
                    0.0
                } else {
                    f64::from(position) / f64::from(high)
                };
                let density = density_low + (density_high - density_low) * fraction;
                let threshold = (density * f64::from(1u32 << DENSITY_PLANES)).round() as u64;
                if threshold >= 1 << DENSITY_PLANES {
                    density_always |= 1 << position;
                } else {
                    for (plane, bits) in density_planes.iter_mut().enumerate() {
                        *bits |= ((threshold >> plane) & 1) << position;
                    }
                }
            }

            // Fold the pins into the planes: a pinned bit costs nothing at
            // sampling time. Pinned ones win over pinned zeros.
            for bits in &mut density_planes {
                *bits &= !(pin_zeros | pin_ones);
            }
            density_always = (density_always & !pin_zeros) | pin_ones;
        }

        Self {
            windows,
            window_offsets,
            union_positions,
            density_planes,
            density_always,
            bits,
            style,
        }
    }

    fn sample(&self, rng: &mut impl Rng) -> u64 {
        match self.style {
            SampleStyle::Sparse {
                extra_bits,
                pin_ones,
                pin_zeros,
            } => self.sample_sparse(rng, extra_bits, pin_ones, pin_zeros),
            SampleStyle::Dense { .. } => self.sample_dense(rng),
        }
    }

    /// Draws an 8-bit uniform value per bit position (bit `j` of position
    /// `p`'s value is bit `p` of the `j`-th random word) and sets the magic
    /// bit wherever the value is below the position's density threshold. The
    /// bit-sliced comparison, most significant plane first, resolves all 64
    /// positions in parallel with a few bitwise ops per plane.
    fn sample_dense(&self, rng: &mut impl Rng) -> u64 {
        let mut less = 0;
        let mut equal = u64::MAX;
        for &plane in self.density_planes.iter().rev() {
            let random = rng.next_u64();
            less |= equal & !random & plane;
            equal &= !(random ^ plane);
        }
        less | self.density_always
    }

    fn sample_sparse(
        &self,
        rng: &mut impl Rng,
        extra_bits: u32,
        pin_ones: u64,
        pin_zeros: u64,
    ) -> u64 {
        let mut magic = 0;
        let doubled = rng.random_range(0..self.windows.len());
        let mut used_offsets = 0u16;

        for (i, &(low, len)) in self.windows.iter().enumerate() {
            // A truncated window of length `len` only reaches the offsets
            // `[bits - len, bits)`; positions pinned to zero are excluded.
            let min_offset = self.bits - len;
            let feasible = self.window_offsets[i];
            if feasible == 0 {
                // Fully pinned to zero; coverage must come from pinned ones.
                continue;
            }

            if i == doubled && feasible.count_ones() >= 2 {
                // Two distinct bits; a multi-bit window sidesteps the
                // power-of-two pigeonhole entirely, so it does not take part
                // in the offset assignment.
                let first = random_set_bit(rng, feasible);
                let second = random_set_bit(rng, feasible & !(1 << first));
                magic |= 1 << (low + first - min_offset);
                magic |= 1 << (low + second - min_offset);
                continue;
            }

            let unused = feasible & !used_offsets;
            let offset = if unused != 0 {
                random_set_bit(rng, unused)
            } else {
                random_set_bit(rng, feasible)
            };
            used_offsets |= 1 << offset;
            magic |= 1 << (low + offset - min_offset);
        }

        if !self.union_positions.is_empty() {
            for _ in 0..extra_bits {
                let position =
                    self.union_positions[rng.random_range(0..self.union_positions.len())];
                magic |= 1 << position;
            }
        }
        (magic & !pin_zeros) | pin_ones
    }
}

/// Parses a hex mask, with or without a `0x` prefix.
fn parse_hex(s: &str) -> Result<u64, std::num::ParseIntError> {
    let s = s.trim_start_matches("0x").trim_start_matches("0X");
    u64::from_str_radix(s, 16)
}

/// Picks a uniformly random set bit of `mask` and returns its index.
fn random_set_bit(rng: &mut impl Rng, mask: u16) -> u32 {
    let n = rng.random_range(0..mask.count_ones());
    let mut remaining = mask;
    for _ in 0..n {
        remaining &= remaining - 1;
    }
    remaining.trailing_zeros()
}

/// The bit indices of the squares a rook on `sq` slides over, excluding the
/// board-edge squares, i.e. the relevant occupancy mask.
fn relevant_occupancy_squares(sq: Square) -> Vec<u8> {
    let dirs: [(BoardVector, fn(Square) -> bool); 4] = [
        (BoardVector::NORTH, |s| s.rank() < Rank::Eighth),
        (BoardVector::SOUTH, |s| s.rank() > Rank::First),
        (BoardVector::EAST, |s| s.file() < File::H),
        (BoardVector::WEST, |s| s.file() > File::A),
    ];

    let mut squares = Vec::new();
    for (dir, can_step) in dirs {
        let mut ray_sq = sq;
        // Stop once the next square would be the edge, which is excluded.
        while can_step(ray_sq) && can_step(ray_sq + dir) {
            ray_sq = ray_sq + dir;
            squares.push(ray_sq as u8);
        }
    }
    squares
}

fn rook_attack_sets(sq: Square) -> Vec<Bitboard> {
    let make_rays = |dir: BoardVector, pred: fn(Square) -> bool| {
        let mut rays = Vec::new();
        let mut ray = Bitboard::new();
        let mut ray_sq = sq;

        while pred(ray_sq) {
            ray_sq = ray_sq + dir;
            ray |= Bitboard::from(ray_sq);
            rays.push(ray);
        }

        // A direction with no available squares contributes one empty value
        // so that the Cartesian product is not empty.
        if rays.is_empty() {
            rays.push(Bitboard::new());
        }

        rays
    };

    let north_rays = make_rays(BoardVector::NORTH, |s| s.rank() < Rank::Eighth);
    let south_rays = make_rays(BoardVector::SOUTH, |s| s.rank() > Rank::First);
    let east_rays = make_rays(BoardVector::EAST, |s| s.file() < File::H);
    let west_rays = make_rays(BoardVector::WEST, |s| s.file() > File::A);

    itertools::iproduct!(north_rays, south_rays, east_rays, west_rays)
        .map(|(n, s, e, w)| n | s | e | w)
        .collect()
}

fn rook_occupancy_sets(sq: Square) -> Vec<OccupancySet> {
    fn make_sets(
        sq: Square,
        attack_set: Bitboard,
        dir: BoardVector,
        can_step: fn(Square) -> bool,
    ) -> Vec<Bitboard> {
        // Construct the complete geometric ray, including the edge square.
        let mut ray_squares = Vec::new();
        let mut ray_sq = sq;

        while can_step(ray_sq) {
            ray_sq = ray_sq + dir;
            ray_squares.push(ray_sq);
        }

        // The rook is already on the edge in this direction.
        if ray_squares.is_empty() {
            return vec![Bitboard::new()];
        }

        // Determine the length of the attacked prefix.
        let attacked_len = ray_squares
            .iter()
            .take_while(|&&s| attack_set.contains(s))
            .count();

        // A legal rook attack must contain the adjacent square whenever
        // there is a square available in this direction.
        assert!(
            attacked_len > 0,
            "Invalid rook attack set: adjacent square is missing"
        );

        // The full ray is attacked. Therefore, all relevant non-edge
        // occupancy squares in this direction must be empty.
        //
        // Occupancy on the edge square is ignored because edge squares
        // are normally excluded from the relevant occupancy mask.
        if attacked_len == ray_squares.len() {
            return vec![Bitboard::new()];
        }

        // The final attacked square is the mandatory blocker.
        let blocker = ray_squares[attacked_len - 1];
        let mut sets = vec![Bitboard::from(blocker)];

        // Squares beyond the blocker are arbitrary. Exclude the final
        // edge square because it cannot affect the rook attack.
        let beyond_blocker = &ray_squares[attacked_len..ray_squares.len() - 1];

        for &square in beyond_blocker {
            let previous_sets = sets.clone();

            for set in previous_sets {
                sets.push(set | Bitboard::from(square));
            }
        }

        sets
    }

    let attack_sets = rook_attack_sets(sq);
    let mut occupancy_sets = Vec::new();

    for (attack_set_index, attack_set) in attack_sets.into_iter().enumerate() {
        let north_sets = make_sets(sq, attack_set, BoardVector::NORTH, |s| {
            s.rank() < Rank::Eighth
        });
        let south_sets = make_sets(sq, attack_set, BoardVector::SOUTH, |s| {
            s.rank() > Rank::First
        });
        let east_sets = make_sets(sq, attack_set, BoardVector::EAST, |s| s.file() < File::H);
        let west_sets = make_sets(sq, attack_set, BoardVector::WEST, |s| s.file() > File::A);

        occupancy_sets.extend(
            itertools::iproduct!(north_sets, south_sets, east_sets, west_sets)
                .map(|(n, s, e, w)| (u64::from(n | s | e | w), attack_set_index)),
        );
    }

    occupancy_sets.sort_unstable_by_key(|&(bb, _)| bb.count_ones());

    assert!(
        occupancy_sets.len() % LANES == 0,
        "Occupancy sets length is always a multiple of LANES"
    );
    let mut occupancy_sets_simd = Vec::with_capacity(occupancy_sets.len() / LANES);
    let (chunks, _rem) = occupancy_sets.as_chunks::<LANES>();
    occupancy_sets_simd.extend(chunks.iter().map(|chunk: &[_; LANES]| OccupancySet {
        bitboards: SimdU64::from_array(chunk.map(|entry| entry.0)),
        attack_set_indices: SimdUsize::from_array(chunk.map(|entry| entry.1)),
    }));

    occupancy_sets_simd
}

struct OccupancySet {
    bitboards: SimdU64,
    attack_set_indices: SimdUsize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use kingly_lib::bb;

    #[test]
    fn test_rook_attack_sets() {
        let sq = Square::D4;
        let attack_sets = rook_attack_sets(sq);
        assert!(attack_sets.contains(&bb!(D5, D6, D7, D8, D3, D2, E4, C4, B4)));
        assert!(!attack_sets.contains(&bb!()));
    }

    #[test]
    fn test_a1_occupancy_sets() {
        let sets = rook_occupancy_sets(Square::A1);

        // A1 is a corner with 12 relevant squares, so there are 2^12 distinct
        // occupancies, packed LANES per set.
        let relevant = relevant_occupancy_squares(Square::A1).len();
        assert_eq!(relevant, 12);
        assert_eq!(sets.len(), (1usize << relevant) / LANES);

        let unique: std::collections::HashSet<_> = sets
            .iter()
            .flat_map(|set| set.bitboards.to_array())
            .collect();

        assert_eq!(unique.len(), 1 << relevant);
    }

    #[test]
    fn test_relevant_occupancy_squares() {
        use std::collections::HashSet;

        let a1: HashSet<u8> = relevant_occupancy_squares(Square::A1).into_iter().collect();
        assert_eq!(a1, HashSet::from([1, 2, 3, 4, 5, 6, 8, 16, 24, 32, 40, 48]));

        // Corners have 12 relevant squares, edges 11, interior 10.
        assert_eq!(relevant_occupancy_squares(Square::H8).len(), 12);
        assert_eq!(relevant_occupancy_squares(Square::A4).len(), 11);
        assert_eq!(relevant_occupancy_squares(Square::E4).len(), 10);
    }

    /// The index width used by tests that need a fixed one.
    const TEST_BITS: u32 = 11;

    #[test]
    fn test_precheck_a1_buckets() {
        // At 11 bits, A1 has 8 high windows (blocker bits 1-6, 8, 16), 3 low
        // (bits 32, 40, 48) and 1 crossing (a4's [29, 39]) — one chunk each
        // after padding.
        let precheck = Precheck::new(Square::A1, TEST_BITS);
        assert_eq!(precheck.hi.len, 8);
        assert_eq!(precheck.lo.len, 3);
        assert_eq!(precheck.mid.len, 1);
        assert_eq!(precheck.hi.right_shifts.len(), 1);
        assert_eq!(precheck.lo.right_shifts.len(), 1);
        assert_eq!(precheck.mid.right_shifts.len(), 1);
    }

    #[test]
    fn test_precheck_matches_naive() {
        let mut rng = SmallRng::seed_from_u64(0xC0FFEE);

        // A4 puts six windows across the 32-bit boundary, H8 exercises the
        // truncated windows, and multiple widths exercise the window math.
        for sq in [Square::A1, Square::A4, Square::E4, Square::H8] {
            for bits in [9, 10, 11, 12] {
                let precheck = Precheck::new(sq, bits);
                // The definition: the empty board's index 0 and all
                // single-blocker indices must be pairwise distinct.
                let naive = |magic: u64| {
                    let mut seen = std::collections::HashSet::from([0u64]);
                    relevant_occupancy_squares(sq)
                        .into_iter()
                        .all(|bit| seen.insert((magic << bit) >> (64 - bits)))
                };

                assert!(!precheck.passes(0));
                // All-ones makes every non-truncated window equal — a conflict.
                assert!(!precheck.passes(u64::MAX));
                for _ in 0..10_000 {
                    // Sparse enough that both outcomes occur frequently.
                    let magic = rng.next_u64() & rng.next_u64();
                    assert_eq!(
                        precheck.passes(magic),
                        naive(magic),
                        "magic {magic:#x} on {sq} at {bits} bits"
                    );
                }
            }
        }
    }

    #[test]
    fn test_sampler_covers_windows() {
        let mut rng = SmallRng::seed_from_u64(42);

        for sq in [Square::A1, Square::A4, Square::H8] {
            let sampler = Sampler::new(
                sq,
                SampleStyle::Sparse {
                    extra_bits: 3,
                    pin_ones: 0,
                    pin_zeros: 0,
                },
                TEST_BITS,
            );
            let squares = relevant_occupancy_squares(sq);
            for _ in 0..1_000 {
                let magic = sampler.sample(&mut rng);
                let mut multi_bit_window = false;
                for &bit in &squares {
                    // Every single-blocker index is non-zero by construction.
                    let index = (magic << bit) >> (64 - TEST_BITS);
                    assert_ne!(index, 0);
                    multi_bit_window |= index.count_ones() >= 2;
                }
                // The doubled window guarantees a multi-bit index somewhere.
                assert!(multi_bit_window);
            }
        }
    }

    #[test]
    fn test_known_reduced_magics() {
        // Known reduced-bit rook magics: Grant Osborne's from
        // https://www.chessprogramming.org/Best_Magics_so_far and Witek's
        // first/second-rank finds from
        // https://www.talkchess.com/forum/viewtopic.php?t=64578
        let known: [(Square, u64, u32); 4] = [
            (Square::A8, 0xEBFFFFB9FF9FC526, 11),
            (Square::H8, 0x7645FFFECBFEA79E, 11),
            (Square::D1, 0xc6000b13534dffff, 10),
            (Square::C2, 0xcae2002cac99fffa, 9),
        ];
        for (sq, magic, bits) in known {
            let occ_sets = rook_occupancy_sets(sq);
            let precheck = Precheck::new(sq, bits);
            let mut idxs = [usize::MAX; 1 << MAX_MAGIC_BITS];
            assert!(
                is_magic(magic, &occ_sets, &precheck, &mut idxs),
                "known magic {magic:#x} rejected for {sq}"
            );
            // The score of a valid magic is exact and maximal regardless of
            // the bound.
            let total = occ_sets.len() * LANES;
            assert_eq!(
                score_magic(magic, &occ_sets, &precheck, &mut idxs, total),
                total,
                "known magic {magic:#x} misscored for {sq}"
            );

            // Bits above the live span are shifted out of every blocker's
            // product, so masking them off must leave the magic valid. (Bits
            // *below* the span are carry-active and must be kept: clearing
            // d1's bits 0-2 breaks its magic.)
            let (_, high) = live_span(sq, bits);
            let masked = magic & (u64::MAX >> (63 - high));
            assert_ne!(masked, magic, "expected dead bits in the known magic");
            assert!(
                is_magic(masked, &occ_sets, &precheck, &mut idxs),
                "span-masked magic {masked:#x} rejected for {sq}"
            );
        }
    }

    #[test]
    fn test_sample_dense_within_live_bits() {
        let mut rng = SmallRng::seed_from_u64(7);

        for sq in [Square::A1, Square::H8] {
            let sampler = Sampler::new(
                sq,
                SampleStyle::Dense {
                    density_low: 0.75,
                    density_high: 0.75,
                    pin_ones: 0,
                    pin_zeros: 0,
                },
                TEST_BITS,
            );
            let (_, high) = live_span(sq, TEST_BITS);
            // Everything from bit 0 to the top of the live span may be set
            // (low bits are carry-active); bits above are dead.
            let live_mask = u64::MAX >> (63 - high);
            for _ in 0..1_000 {
                let magic = sampler.sample(&mut rng);
                assert_ne!(magic, 0);
                assert_eq!(magic & !live_mask, 0, "bits above the live span");
            }
        }
    }

    #[test]
    fn test_sample_sparse_pins() {
        let mut rng = SmallRng::seed_from_u64(17);
        let sampler = Sampler::new(
            Square::A1,
            SampleStyle::Sparse {
                extra_bits: 3,
                pin_ones: 0x30,
                pin_zeros: 0xf00,
            },
            TEST_BITS,
        );
        let squares = relevant_occupancy_squares(Square::A1);
        for _ in 0..1_000 {
            let magic = sampler.sample(&mut rng);
            assert_eq!(magic & 0x30, 0x30, "pinned ones missing");
            assert_eq!(magic & 0xf00, 0, "pinned zeros set");
            // Window coverage survives the pins: placement avoids pinned-zero
            // positions instead of losing bits to them.
            for &bit in &squares {
                assert_ne!((magic << bit) >> (64 - TEST_BITS), 0);
            }
        }
    }

    #[test]
    fn test_mutate_respects_pins() {
        let mut rng = SmallRng::seed_from_u64(23);
        let pinned = 0x30 | 0xf00; // pin_ones | pin_zeros from the sparse test
        let base = 0x30; // pinned ones set, pinned zeros clear
        let span_high = 20;

        // pin_break = 0: pinned bits are never disturbed.
        for _ in 0..10_000 {
            let m = mutate(base, 4, span_high, pinned, 0.0, &mut rng);
            assert_eq!(m & pinned, base & pinned, "pins broken at pin_break=0");
            assert_eq!(m & !(u64::MAX >> (63 - span_high)), 0, "flip out of span");
        }

        // pin_break = 1: pins are ignored, so they do get flipped sometimes.
        let mut disturbed = false;
        for _ in 0..10_000 {
            let m = mutate(base, 4, span_high, pinned, 1.0, &mut rng);
            disturbed |= m & pinned != base & pinned;
        }
        assert!(disturbed, "pins never touched at pin_break=1");
    }

    #[test]
    fn test_sample_dense_pins() {
        let mut rng = SmallRng::seed_from_u64(13);
        let sampler = Sampler::new(
            Square::D1,
            SampleStyle::Dense {
                density_low: 0.75,
                density_high: 0.75,
                pin_ones: 0x7f0,
                pin_zeros: 0x1e00_0000_0000,
            },
            TEST_BITS,
        );
        for _ in 0..1_000 {
            let magic = sampler.sample(&mut rng);
            assert_eq!(magic & 0x7f0, 0x7f0, "pinned ones missing");
            assert_eq!(magic & 0x1e00_0000_0000, 0, "pinned zeros set");
        }
    }

    #[test]
    fn test_sample_dense_gradient() {
        let mut rng = SmallRng::seed_from_u64(11);
        let sampler = Sampler::new(
            Square::A1,
            SampleStyle::Dense {
                density_low: 0.9,
                density_high: 0.3,
                pin_ones: 0,
                pin_zeros: 0,
            },
            TEST_BITS,
        );
        let (_, high) = live_span(Square::A1, TEST_BITS);

        let n = 10_000;
        let mut low_hits = 0u32;
        let mut high_hits = 0u32;
        for _ in 0..n {
            let magic = sampler.sample(&mut rng);
            low_hits += (magic & 1) as u32;
            high_hits += (magic >> high & 1) as u32;
        }

        // Bit 0 should be set ~90% of the time, the highest live bit ~30%;
        // ±5% leaves lots of statistical slack.
        assert!((8_500..=9_500).contains(&low_hits), "low: {low_hits}");
        assert!((2_500..=3_500).contains(&high_hits), "high: {high_hits}");
    }
}
