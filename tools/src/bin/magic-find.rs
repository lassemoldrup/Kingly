#![feature(portable_simd)]

use std::io::{self, Write};
use std::simd::cmp::SimdPartialEq;
use std::simd::num::SimdUint;
use std::simd::Simd;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread;
use std::time::Instant;

use clap::{Parser, Subcommand};
use crossbeam::deque::{Injector, Stealer, Worker};
use kingly_lib::types::{Bitboard, BoardVector, File, Rank, Square};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const ROOK_MAGIC_BITS: u64 = 11;
const LANES: usize = 4;

type SimdU64 = Simd<u64, LANES>;
type SimdUsize = Simd<usize, LANES>;

#[derive(Parser)]
struct App {
    /// The square to find rook magics for, e.g. "a1" or "e4".
    #[arg(long, global = true, default_value = "a1")]
    square: Square,
    #[clap(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Randomly sample sparse magics across all cores until one works.
    Random {
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
}

fn main() {
    let app = App::parse();
    let occ_sets = rook_occupancy_sets(app.square);
    let precheck = precheck_masks(app.square);

    match app.command {
        Command::Random { single_threaded } => run_random(&occ_sets, &precheck, single_threaded),
        Command::Dfs {
            max_bits,
            single_threaded,
        } => run_dfs(&occ_sets, &precheck, max_bits, single_threaded),
    }
}

fn run_random(occ_sets: &[OccupancySet], precheck: &[SimdU64], single_threaded: bool) {
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
                random_worker(worker_id, occ_sets, precheck, found, tried_counts, start);
            });
        }
    });
}

fn random_worker(
    worker_id: usize,
    occ_sets: &[OccupancySet],
    precheck: &[SimdU64],
    found: &AtomicBool,
    tried_counts: &[AtomicU64],
    start: Instant,
) {
    let mut rng = SmallRng::from_rng(&mut rand::rng());
    let mut idxs = [usize::MAX; 1 << ROOK_MAGIC_BITS];
    let is_reporter = worker_id == 0;
    let mut local_count = 0u64;

    while !found.load(Ordering::Relaxed) {
        let magic = rng.next_u64() & rng.next_u64() & rng.next_u64();
        local_count += 1;
        tried_counts[worker_id].fetch_add(1, Ordering::Relaxed);

        if try_magic(magic, occ_sets, precheck, &mut idxs) == occ_sets.len() {
            if !found.swap(true, Ordering::Relaxed) {
                eprintln!("\nFound magic: {:#x}", magic);
            }
            return;
        }

        if is_reporter && local_count % 100_000 == 0 {
            let count = tried_counts
                .iter()
                .map(|counter| counter.load(Ordering::Relaxed))
                .sum();
            report(count, start);
        }
    }
}

fn run_dfs(occ_sets: &[OccupancySet], precheck: &[SimdU64], max_bits: u32, single_threaded: bool) {
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
    precheck: &[SimdU64],
    max_bits: u32,
    found: &AtomicBool,
    active: &AtomicUsize,
    tried_counts: &[AtomicU64],
    start: Instant,
) {
    let mut idxs = [usize::MAX; 1 << ROOK_MAGIC_BITS];
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

        if try_magic(magic, occ_sets, precheck, &mut idxs) == occ_sets.len() {
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

fn try_magic(
    magic: u64,
    occ_sets: &[OccupancySet],
    precheck: &[SimdU64],
    idxs: &mut [usize; 1 << ROOK_MAGIC_BITS],
) -> usize {
    const SHIFT: SimdU64 = SimdU64::splat(64 - ROOK_MAGIC_BITS);
    const EMPTY: SimdUsize = SimdUsize::splat(usize::MAX);
    const ZERO: SimdU64 = SimdU64::splat(0);
    let magic = SimdU64::splat(magic);

    // Pre-check: every single-blocker occupancy has a different attack set than
    // the empty board, which always hashes to index 0, so each must hash to a
    // non-zero index. `precheck` holds one index-window mask per relevant
    // square; if `magic` misses any window, that occupancy would collide with
    // the empty board and the magic cannot work.
    for &mask in precheck {
        if (mask & magic).simd_eq(ZERO).any() {
            return 0;
        }
    }

    idxs.fill(usize::MAX);

    let mut hits = 0;
    for occ in occ_sets {
        let keys = ((occ.bitboards * magic) >> SHIFT).cast::<usize>();
        let atk_sets = Simd::gather_or_default(idxs, keys);
        // All non-empty entries in `atk_sets` must be equal to the attack set index for
        // this occupancy set.
        let hits_mask = atk_sets.simd_eq(EMPTY) | atk_sets.simd_eq(occ.attack_set_indices);
        if hits_mask.all() {
            occ.attack_set_indices.scatter(idxs, keys);
            hits += 1;
        } else {
            break;
        }
    }

    hits
}

/// Builds the pre-check index-window masks for a rook on `sq`, packed into SIMD
/// lanes. For a blocker on relevant square `k`, the hash index is the top
/// `ROOK_MAGIC_BITS` bits of `magic << k`, which is non-zero iff `magic` has a
/// bit set in the window `magic & (index_window >> k)`.
fn precheck_masks(sq: Square) -> Vec<SimdU64> {
    const SHIFT: u64 = 64 - ROOK_MAGIC_BITS;
    let index_window = ((1u64 << ROOK_MAGIC_BITS) - 1) << SHIFT;

    let mut masks: Vec<u64> = relevant_occupancy_squares(sq)
        .into_iter()
        .map(|bit| index_window >> bit)
        .collect();

    // Pad to a whole number of lanes by repeating a real mask, so the SIMD tail
    // re-checks an existing (necessary) condition instead of a bogus one.
    while masks.len() % LANES != 0 {
        masks.push(masks[0]);
    }

    masks
        .as_chunks::<LANES>()
        .0
        .iter()
        .map(|&chunk| SimdU64::from_array(chunk))
        .collect()
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

    #[test]
    fn test_precheck_masks_match_old_a1() {
        // The old hand-written A1 masks, as a reference for the general builder.
        let old: std::collections::HashSet<u64> = [1, 2, 3, 4, 5, 6, 8, 16, 24, 32, 40, 48]
            .into_iter()
            .map(|s: u64| ((1u64 << ROOK_MAGIC_BITS) - 1) << (64 - ROOK_MAGIC_BITS) >> s)
            .collect();

        let masks: std::collections::HashSet<u64> = precheck_masks(Square::A1)
            .iter()
            .flat_map(|chunk| chunk.to_array())
            .collect();

        // The general builder may repeat a mask to pad SIMD lanes, but the set
        // of distinct windows must be exactly the old ones.
        assert_eq!(masks, old);
    }
}
