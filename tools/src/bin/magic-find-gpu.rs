//! GPU hill-climbing rook-magic finder for AMD (and any OpenCL) GPUs.
//!
//! Each GPU workgroup is an independent hill climber with adaptive kicking
//! (see `magic_climb.cl`); the host manages the shared elite pool between
//! kernel launches and validates any solution on the CPU before trusting it.
//!
//! The occupancy/attack-set generation is duplicated from `magic-find.rs` (the
//! CPU tool) so the GPU searches over the exact same, already-tested data.

use std::collections::HashMap;
use std::ptr;
use std::time::Instant;

use clap::Parser;
use kingly_lib::types::{Bitboard, BoardVector, File, Rank, Square};
use rand::rngs::SmallRng;
use rand::{Rng, RngExt, SeedableRng};

use opencl3::command_queue::CommandQueue;
use opencl3::context::Context;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};
use opencl3::kernel::{ExecuteKernel, Kernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE};
use opencl3::program::Program;
use opencl3::types::{cl_uint, cl_ulong, CL_BLOCKING};

const KERNEL_SRC: &str = include_str!("magic_climb.cl");

#[derive(Parser)]
struct App {
    /// The square to find rook magics for, e.g. "d1" or "g1".
    #[arg(long, default_value = "d1")]
    square: Square,
    /// Target index bits. Defaults to the premask size minus one (a reduced
    /// magic, the interesting case).
    #[arg(long)]
    bits: Option<u32>,
    /// Number of parallel climbers (GPU workgroups).
    #[arg(long, default_value_t = 4096)]
    climbers: usize,
    /// Threads per workgroup.
    #[arg(long, default_value_t = 64)]
    wg: usize,
    /// Hill-climb steps per climber per kernel launch.
    #[arg(long, default_value_t = 3000)]
    iters: u32,
    /// Stagnation steps before a kick.
    #[arg(long, default_value_t = 300)]
    stagnation: u32,
    /// Base / grow / max kick size (bits flipped), adaptive like the CPU tool.
    #[arg(long, default_value_t = 8)]
    kick_flips: u32,
    #[arg(long, default_value_t = 2)]
    kick_grow: u32,
    #[arg(long, default_value_t = 40)]
    kick_max: u32,
    /// Elite pool size (top magics used as kick material).
    #[arg(long, default_value_t = 256)]
    elites: usize,
    /// Elite pool merge radius (Hamming) for diversity.
    #[arg(long, default_value_t = 6)]
    elite_dist: u32,
    /// Fraction of climbers re-seeded to fresh random magics each round.
    #[arg(long, default_value_t = 0.1)]
    reseed: f64,
    /// Stop after this many seconds (0 = run until found).
    #[arg(long, default_value_t = 0)]
    max_seconds: u64,
    /// GPU device index (see the startup listing).
    #[arg(long, default_value_t = 0)]
    device: usize,
}

fn main() {
    let app = App::parse();
    let sq = app.square;
    let natural = relevant_occupancy_squares(sq).len() as u32;
    let bits = app.bits.unwrap_or(natural - 1);
    assert!((1..=13).contains(&bits), "bits must be in [1, 13]");

    let occ = rook_occupancy(sq);
    let num_occ = occ.len();
    let (occ_bb, occ_idx): (Vec<cl_ulong>, Vec<cl_uint>) =
        occ.iter().map(|&(bb, idx)| (bb, idx)).unzip();
    let span_high = live_span_high(sq, bits);
    let span_mask = if span_high >= 63 { u64::MAX } else { (1u64 << (span_high + 1)) - 1 };

    eprintln!(
        "square {sq}: natural={natural} target={bits} bits, {num_occ} occupancies, \
         span_high={span_high}, {} climbers x {} threads",
        app.climbers, app.wg
    );

    // --- OpenCL setup -----------------------------------------------------
    let device_ids = get_all_devices(CL_DEVICE_TYPE_GPU).expect("no OpenCL GPU devices");
    for (i, &id) in device_ids.iter().enumerate() {
        let d = Device::new(id);
        eprintln!("  device {i}: {}", d.name().unwrap_or_default());
    }
    let device = Device::new(device_ids[app.device.min(device_ids.len() - 1)]);
    let context = Context::from_device(&device).expect("context");
    let queue = CommandQueue::create_default(&context, 0).expect("queue");

    let options = format!(
        "-D BITS={bits} -D NUM_OCC={num_occ} -D SPAN_HIGH={span_high} \
         -D STAG={} -D KICK_BASE={} -D KICK_GROW={} -D KICK_MAX={}",
        app.stagnation, app.kick_flips, app.kick_grow, app.kick_max
    );
    let program = match Program::create_and_build_from_source(&context, KERNEL_SRC, &options) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("kernel build failed:\n{e}");
            std::process::exit(1);
        }
    };
    let kernel = Kernel::create(&program, "climb").expect("kernel");

    // --- Buffers ----------------------------------------------------------
    let n = app.climbers;
    let mut rng = SmallRng::from_rng(&mut rand::rng());

    // Fresh random starting magics (dense-ish within the span).
    let mut cur: Vec<cl_ulong> = (0..n).map(|_| fresh_magic(&mut rng, span_mask)).collect();
    let mut best_magic: Vec<cl_ulong> = vec![0; n];
    let mut best_score: Vec<cl_uint> = vec![0; n];
    let mut rng_state: Vec<cl_ulong> = (0..n).map(|_| rng.next_u64() | 1).collect();
    let mut stag: Vec<cl_uint> = vec![0; n];
    let mut kick: Vec<cl_uint> = vec![app.kick_flips; n];

    let occ_bb_buf = write_ro(&context, &queue, &occ_bb);
    let occ_idx_buf = write_ro(&context, &queue, &occ_idx);
    let mut cur_buf = write_rw(&context, &queue, &cur);
    let mut best_magic_buf = write_rw(&context, &queue, &best_magic);
    let mut best_score_buf = write_rw(&context, &queue, &best_score);
    let mut rng_buf = write_rw(&context, &queue, &rng_state);
    let mut stag_buf = write_rw(&context, &queue, &stag);
    let mut kick_buf = write_rw(&context, &queue, &kick);
    // Elite buffer is at least length 1 (OpenCL dislikes zero-size buffers).
    let mut elite_buf =
        unsafe { Buffer::<cl_ulong>::create(&context, CL_MEM_READ_ONLY, app.elites.max(1), ptr::null_mut()) }
            .expect("elite buf");

    // --- Host-side elite pool ---------------------------------------------
    let mut pool: Vec<(u64, u32)> = Vec::new(); // (magic, score) sorted desc
    let start = Instant::now();
    let mut round = 0u64;
    let mut global_best = 0u32;

    loop {
        round += 1;

        // Upload the current elites (magics only).
        let elite_magics: Vec<cl_ulong> = pool.iter().map(|&(m, _)| m).take(app.elites).collect();
        let num_elites = elite_magics.len() as cl_uint;
        if !elite_magics.is_empty() {
            unsafe { queue.enqueue_write_buffer(&mut elite_buf, CL_BLOCKING, 0, &elite_magics, &[]) }
                .expect("write elites");
        }

        // Launch: one workgroup per climber.
        let global = n * app.wg;
        let event = unsafe {
            ExecuteKernel::new(&kernel)
                .set_arg(&occ_bb_buf)
                .set_arg(&occ_idx_buf)
                .set_arg(&cur_buf)
                .set_arg(&best_magic_buf)
                .set_arg(&best_score_buf)
                .set_arg(&rng_buf)
                .set_arg(&stag_buf)
                .set_arg(&kick_buf)
                .set_arg(&elite_buf)
                .set_arg(&num_elites)
                .set_arg(&app.iters)
                .set_global_work_size(global)
                .set_local_work_size(app.wg)
                .enqueue_nd_range(&queue)
                .expect("launch")
        };
        event.wait().expect("kernel wait");

        // Read back per-climber best.
        unsafe { queue.enqueue_read_buffer(&mut best_magic_buf, CL_BLOCKING, 0, &mut best_magic, &[]) }
            .expect("read best_magic");
        unsafe { queue.enqueue_read_buffer(&mut best_score_buf, CL_BLOCKING, 0, &mut best_score, &[]) }
            .expect("read best_score");

        // Fold results into the host elite pool and check for a solution.
        for i in 0..n {
            let (m, s) = (best_magic[i], best_score[i]);
            if s as usize == num_occ {
                if cpu_valid(m, &occ, bits) {
                    println!("\nFound magic: {m:#x}");
                    println!("square {sq}, {bits} bits (shift {})", 64 - bits);
                    return;
                } else {
                    // Kernel false-positive guard; ignore and keep searching.
                    continue;
                }
            }
            pool_insert(&mut pool, m, s, app.elites, app.elite_dist);
        }

        if let Some(&(bm, bs)) = pool.first() {
            if bs > global_best {
                global_best = bs;
                eprintln!("\nnew best {bs}/{num_occ}: {bm:#x}");
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        eprint!(
            "\rround {round}  best {}/{num_occ} ({:.2}%)  {:.0} evals/s  {:.0}s",
            global_best,
            100.0 * global_best as f64 / num_occ as f64,
            (round as f64 * n as f64 * app.iters as f64) / elapsed,
            elapsed,
        );

        // Re-seed a fraction of climbers to fresh randoms for diversity, and
        // reset their per-climber best so a stale high score isn't re-reported.
        let reseeds = (n as f64 * app.reseed) as usize;
        for _ in 0..reseeds {
            let i = rng.random_range(0..n);
            cur[i] = fresh_magic(&mut rng, span_mask);
            best_magic[i] = 0;
            best_score[i] = 0;
            stag[i] = 0;
            kick[i] = app.kick_flips;
        }
        if reseeds > 0 {
            unsafe { queue.enqueue_write_buffer(&mut cur_buf, CL_BLOCKING, 0, &cur, &[]) }.unwrap();
            unsafe { queue.enqueue_write_buffer(&mut best_magic_buf, CL_BLOCKING, 0, &best_magic, &[]) }.unwrap();
            unsafe { queue.enqueue_write_buffer(&mut best_score_buf, CL_BLOCKING, 0, &best_score, &[]) }.unwrap();
            unsafe { queue.enqueue_write_buffer(&mut stag_buf, CL_BLOCKING, 0, &stag, &[]) }.unwrap();
            unsafe { queue.enqueue_write_buffer(&mut kick_buf, CL_BLOCKING, 0, &kick, &[]) }.unwrap();
        }

        if app.max_seconds > 0 && elapsed >= app.max_seconds as f64 {
            eprintln!("\nstopped after {elapsed:.0}s, best {global_best}/{num_occ}");
            return;
        }
    }
}

/// A fresh dense-ish starting magic within the live span.
fn fresh_magic(rng: &mut impl Rng, span_mask: u64) -> u64 {
    (rng.next_u64() | rng.next_u64()) & span_mask
}

/// Inserts `(magic, score)` into the descending, capped, Hamming-deduped pool.
fn pool_insert(pool: &mut Vec<(u64, u32)>, magic: u64, score: u32, cap: usize, merge_dist: u32) {
    if score == 0 {
        return;
    }
    if pool.len() >= cap && score <= pool.last().unwrap().1 {
        return;
    }
    if let Some(i) = pool
        .iter()
        .position(|&(m, _)| (m ^ magic).count_ones() <= merge_dist)
    {
        if score <= pool[i].1 {
            return;
        }
        pool.remove(i);
    }
    let pos = pool.partition_point(|&(_, s)| s >= score);
    pool.insert(pos, (magic, score));
    pool.truncate(cap);
}

/// Exact CPU re-check that `magic` maps every occupancy consistently.
fn cpu_valid(magic: u64, occ: &[(u64, u32)], bits: u32) -> bool {
    let shift = 64 - bits;
    let mut table: HashMap<u64, u32> = HashMap::with_capacity(occ.len());
    for &(bb, idx) in occ {
        let slot = bb.wrapping_mul(magic) >> shift;
        match table.get(&slot) {
            Some(&other) if other != idx => return false,
            _ => {
                table.insert(slot, idx);
            }
        }
    }
    true
}

fn write_ro<T: Copy>(ctx: &Context, q: &CommandQueue, data: &[T]) -> Buffer<T> {
    let mut buf = unsafe { Buffer::<T>::create(ctx, CL_MEM_READ_ONLY, data.len(), ptr::null_mut()) }
        .expect("ro buffer");
    unsafe { q.enqueue_write_buffer(&mut buf, CL_BLOCKING, 0, data, &[]) }.expect("write ro");
    buf
}

fn write_rw<T: Copy>(ctx: &Context, q: &CommandQueue, data: &[T]) -> Buffer<T> {
    let mut buf = unsafe { Buffer::<T>::create(ctx, CL_MEM_READ_WRITE, data.len(), ptr::null_mut()) }
        .expect("rw buffer");
    unsafe { q.enqueue_write_buffer(&mut buf, CL_BLOCKING, 0, data, &[]) }.expect("write rw");
    buf
}

// --- Occupancy generation (mirrors magic-find.rs) -------------------------

/// The relevant (non-edge) blocker squares for a rook on `sq`.
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
        while can_step(ray_sq) && can_step(ray_sq + dir) {
            ray_sq = ray_sq + dir;
            squares.push(ray_sq as u8);
        }
    }
    squares
}

/// The top of the live span (highest magic bit any single-blocker window
/// reaches), for the `bits`-bit index.
fn live_span_high(sq: Square, bits: u32) -> u32 {
    let mut union = 0u64;
    for k in relevant_occupancy_squares(sq).into_iter().map(u32::from) {
        let low = (64 - bits).saturating_sub(k);
        let high = 63 - k;
        for b in low..=high {
            union |= 1u64 << b;
        }
    }
    63 - union.leading_zeros()
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
        if rays.is_empty() {
            rays.push(Bitboard::new());
        }
        rays
    };
    let north = make_rays(BoardVector::NORTH, |s| s.rank() < Rank::Eighth);
    let south = make_rays(BoardVector::SOUTH, |s| s.rank() > Rank::First);
    let east = make_rays(BoardVector::EAST, |s| s.file() < File::H);
    let west = make_rays(BoardVector::WEST, |s| s.file() > File::A);
    itertools::iproduct!(north, south, east, west)
        .map(|(n, s, e, w)| n | s | e | w)
        .collect()
}

/// Every occupancy of `sq`'s relevant mask paired with its attack-set index.
fn rook_occupancy(sq: Square) -> Vec<(u64, u32)> {
    fn make_sets(
        sq: Square,
        attack_set: Bitboard,
        dir: BoardVector,
        can_step: fn(Square) -> bool,
    ) -> Vec<Bitboard> {
        let mut ray_squares = Vec::new();
        let mut ray_sq = sq;
        while can_step(ray_sq) {
            ray_sq = ray_sq + dir;
            ray_squares.push(ray_sq);
        }
        if ray_squares.is_empty() {
            return vec![Bitboard::new()];
        }
        let attacked_len = ray_squares
            .iter()
            .take_while(|&&s| attack_set.contains(s))
            .count();
        assert!(attacked_len > 0, "adjacent square missing from attack set");
        if attacked_len == ray_squares.len() {
            return vec![Bitboard::new()];
        }
        let blocker = ray_squares[attacked_len - 1];
        let mut sets = vec![Bitboard::from(blocker)];
        let beyond = &ray_squares[attacked_len..ray_squares.len() - 1];
        for &square in beyond {
            for set in sets.clone() {
                sets.push(set | Bitboard::from(square));
            }
        }
        sets
    }

    let attack_sets = rook_attack_sets(sq);
    let mut out = Vec::new();
    for (idx, attack_set) in attack_sets.into_iter().enumerate() {
        let north = make_sets(sq, attack_set, BoardVector::NORTH, |s| s.rank() < Rank::Eighth);
        let south = make_sets(sq, attack_set, BoardVector::SOUTH, |s| s.rank() > Rank::First);
        let east = make_sets(sq, attack_set, BoardVector::EAST, |s| s.file() < File::H);
        let west = make_sets(sq, attack_set, BoardVector::WEST, |s| s.file() > File::A);
        out.extend(
            itertools::iproduct!(north, south, east, west)
                .map(|(n, s, e, w)| (u64::from(n | s | e | w), idx as u32)),
        );
    }
    out
}
