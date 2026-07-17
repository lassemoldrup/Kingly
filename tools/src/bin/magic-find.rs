#![feature(portable_simd)]

use std::simd::cmp::SimdPartialEq;
use std::simd::num::SimdUint;
use std::simd::Simd;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc,
};
use std::time::Instant;
use std::{process, thread};

use kingly_lib::types::{Bitboard, BoardVector, File, Rank, Square};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

const ROOK_MAGIC_BITS: u64 = 11;
const LANES: usize = 4;

type SimdU64 = Simd<u64, LANES>;
type SimdUsize = Simd<usize, LANES>;

fn main() {
    let single_threaded = std::env::args().any(|arg| arg == "--single-threaded");
    let occ_sets = Arc::new(rook_occupancy_sets(Square::A1));
    let found = Arc::new(AtomicBool::new(false));
    let worker_count = thread::available_parallelism()
        .map(|parallelism| parallelism.get())
        .unwrap_or(1);
    let tried_counts = Arc::new(
        (0..worker_count)
            .map(|_| AtomicU64::new(0))
            .collect::<Vec<_>>(),
    );
    let near_miss_counts = Arc::new(
        (0..worker_count)
            .map(|_| AtomicU64::new(0))
            .collect::<Vec<_>>(),
    );

    let start = Instant::now();

    if single_threaded {
        run_worker(
            0,
            Arc::clone(&occ_sets),
            Arc::clone(&found),
            Arc::clone(&tried_counts),
            Arc::clone(&near_miss_counts),
            start,
        );
    } else {
        let mut workers = Vec::with_capacity(worker_count);
        for worker_id in 0..worker_count {
            let occ_sets = Arc::clone(&occ_sets);
            let found = Arc::clone(&found);
            let tried_counts = Arc::clone(&tried_counts);
            let near_miss_counts = Arc::clone(&near_miss_counts);

            workers.push(thread::spawn(move || {
                run_worker(
                    worker_id,
                    occ_sets,
                    found,
                    tried_counts,
                    near_miss_counts,
                    start,
                );
            }));
        }

        for worker in workers {
            worker.join().unwrap();
        }
    }
}

fn run_worker(
    worker_id: usize,
    occ_sets: Arc<Vec<OccupancySet>>,
    found: Arc<AtomicBool>,
    tried_counts: Arc<Vec<AtomicU64>>,
    near_miss_counts: Arc<Vec<AtomicU64>>,
    start: Instant,
) {
    let mut rng = SmallRng::from_rng(&mut rand::rng());
    let mut local_count = 0u64;
    let is_reporter = worker_id == 0;
    let mut magic = rng.next_u64() & rng.next_u64() & rng.next_u64();

    loop {
        local_count += 1;
        tried_counts[worker_id].fetch_add(1, Ordering::Relaxed);

        let hits = try_magic(magic, &occ_sets);
        if hits == occ_sets.len() {
            if !found.swap(true, Ordering::Relaxed) {
                eprintln!("Found magic: {:#x}", magic);
            }
            break;
        // } else if 25 * hits >= 24 * occ_sets.len() {
        //     // really near miss, flip a few bits
        //     near_miss_counts[worker_id].fetch_add(1, Ordering::Relaxed);
        //     magic ^=
        //         rng.next_u64() & rng.next_u64() & rng.next_u64() &
        // rng.next_u64() & rng.next_u64(); } else if 9 * hits >= 8 *
        // occ_sets.len() {     // close miss, flip a few bits
        //     magic ^= rng.next_u64() & rng.next_u64() & rng.next_u64() &
        // rng.next_u64(); } else if hits * 3 >= 2 * occ_sets.len() {
        //     // near-ish miss, flip a few bits
        //     magic ^= rng.next_u64() & rng.next_u64() & rng.next_u64();
        // } else {
        } else {
            // generate a new magic
            magic = rng.next_u64() & rng.next_u64() & rng.next_u64();
        }

        if is_reporter && local_count % 100000 == 0 {
            let count = tried_counts
                .iter()
                .map(|counter| counter.load(Ordering::Relaxed))
                .sum::<u64>();
            let near_misses = near_miss_counts
                .iter()
                .map(|counter| counter.load(Ordering::Relaxed))
                .sum::<u64>();
            let elapsed = start.elapsed();
            let elapsed_secs = elapsed.as_secs_f64();
            let rate = count as f64 / elapsed_secs;
            let near_miss_rate = if count == 0 {
                0.0
            } else {
                near_misses as f64 / count as f64
            };
            print!(
                "\rTried {} magics in {:.2} seconds ({:.2} magics/sec, {} near misses, {:.2}% near-miss rate)",
                count,
                elapsed_secs,
                rate,
                near_misses,
                near_miss_rate * 100.0
            );
        }
    }

    process::exit(0);
}

fn try_magic(magic: u64, occ_sets: &[OccupancySet]) -> usize {
    const SHIFT: SimdU64 = SimdU64::splat(64 - ROOK_MAGIC_BITS);
    const EMPTY: SimdUsize = SimdUsize::splat(usize::MAX);
    let magic = SimdU64::splat(magic);

    let mut idxs = [usize::MAX; 1 << ROOK_MAGIC_BITS];
    let mut hits = 0;
    for occ in occ_sets {
        let keys = ((occ.bitboards * magic) >> SHIFT).cast::<usize>();
        let atk_sets = Simd::gather_or_default(&idxs, keys);
        // All non-empty entries in `atk_sets` must be equal to the attack set index for
        // this occupancy set.
        let hits_mask = atk_sets.simd_eq(EMPTY) | atk_sets.simd_eq(occ.attack_set_indices);
        if hits_mask.all() {
            occ.attack_set_indices.scatter(&mut idxs, keys);
            hits += 1;
        } else {
            break;
        }
    }

    hits
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

        assert_eq!(sets.len(), (1usize << ROOK_MAGIC_BITS as usize) / LANES);

        let unique: std::collections::HashSet<_> = sets
            .iter()
            .flat_map(|set| set.bitboards.to_array())
            .collect();

        assert_eq!(unique.len(), 1 << 12);
    }
}
