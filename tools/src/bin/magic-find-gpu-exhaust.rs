//! Exhaustive GPU magic checker: for each popcount `m` in a range, enumerate
//! every magic with exactly `m` bits (chosen from positions [0, span_high]),
//! skip those failing the single-blocker window necessary condition on the GPU,
//! and validate the (rare) survivors exactly on the CPU.
//!
//! The search space is split with zero coordination via the combinatorial
//! number system: each work-item unranks its start index and walks a chunk.
//!
//! Note: valid reduced magics for the hard squares are *dense* (~40 bits), far
//! beyond any tractable popcount, so this tool cannot find them. It is for
//! proving non-existence of low-popcount magics, or finding magics for cases
//! where sparse ones exist.
//!
//! (Occupancy generation is duplicated from magic-find.rs / magic-find-gpu.rs;
//! worth consolidating into a shared module.)

use std::collections::HashMap;
use std::ptr;
use std::time::Instant;

use clap::Parser;
use kingly_lib::types::{Bitboard, BoardVector, File, Rank, Square};

use opencl3::command_queue::CommandQueue;
use opencl3::context::Context;
use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};
use opencl3::kernel::{ExecuteKernel, Kernel};
use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_READ_WRITE};
use opencl3::program::Program;
use opencl3::types::{cl_uint, cl_ulong, CL_BLOCKING};

const KERNEL_SRC: &str = include_str!("magic_exhaust.cl");

#[derive(Parser)]
struct App {
    /// The square to check rook magics for.
    #[arg(long, default_value = "d1")]
    square: Square,
    /// Target index bits. Defaults to the premask size minus one.
    #[arg(long)]
    bits: Option<u32>,
    /// Lowest popcount to enumerate.
    #[arg(long, default_value_t = 1)]
    min_bits: u32,
    /// Highest popcount to enumerate.
    #[arg(long, default_value_t = 7)]
    max_bits: u32,
    /// Enumerate the sparse *complement*: the popcounts are the number of
    /// cleared span positions, so magics have popcount (span_width - m). Use
    /// this to check dense magics (span_width - m ~ 40) via small m ~ 10.
    #[arg(long)]
    complement: bool,
    /// Candidates enumerated per work-item.
    #[arg(long, default_value_t = 64)]
    chunk: u32,
    /// Threads per workgroup for the phase-2 scoring kernel.
    #[arg(long, default_value_t = 64)]
    wg: usize,
    /// Survivor output buffer capacity (magics). Overflow => incomplete run.
    #[arg(long, default_value_t = 16 * 1024 * 1024)]
    out_cap: usize,
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
    let rel_k: Vec<cl_uint> = relevant_occupancy_squares(sq).iter().map(|&k| k as cl_uint).collect();
    let num_rel = rel_k.len();
    let span_high = live_span_high(sq, bits);
    let n = span_high + 1; // positions [0, span_high]

    // Binomial table C(c, k) for c in 0..=n, k in 0..=max_bits.
    let stride = (app.max_bits + 1) as usize;
    let mut binom = vec![0u64; (n as usize + 1) * stride];
    for c in 0..=n as usize {
        binom[c * stride] = 1; // C(c, 0)
        for k in 1..stride.min(c + 1) {
            binom[c * stride + k] =
                binom[(c - 1) * stride + k - 1] + binom[(c - 1) * stride + k];
        }
    }
    let binom_at = |c: u32, k: u32| binom[c as usize * stride + k as usize];

    eprintln!(
        "square {sq}: {bits} bits, {} occupancies, {num_rel} windows, positions [0, {span_high}]",
        occ.len()
    );

    // --- OpenCL setup -----------------------------------------------------
    let device_ids = get_all_devices(CL_DEVICE_TYPE_GPU).expect("no OpenCL GPU");
    for (i, &id) in device_ids.iter().enumerate() {
        eprintln!("  device {i}: {}", Device::new(id).name().unwrap_or_default());
    }
    let device = Device::new(device_ids[app.device.min(device_ids.len() - 1)]);
    let context = Context::from_device(&device).expect("context");
    let queue = CommandQueue::create_default(&context, 0).expect("queue");

    let options = format!(
        "-D BITS={bits} -D SHIFT={} -D NUM_REL={num_rel} -D STRIDE={stride} -D OUT_CAP={} \
         -D NUM_OCC={num_occ} -D WG={}",
        64 - bits,
        app.out_cap,
        app.wg,
    );
    let program = match Program::create_and_build_from_source(&context, KERNEL_SRC, &options) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("kernel build failed:\n{e}");
            std::process::exit(1);
        }
    };
    let exhaust = Kernel::create(&program, "exhaust").expect("exhaust kernel");
    let validate = Kernel::create(&program, "validate").expect("validate kernel");

    let binom_buf = write_ro(&context, &queue, &binom);
    let rel_k_buf = write_ro(&context, &queue, &rel_k);
    let occ_bb_buf = write_ro(&context, &queue, &occ_bb);
    let occ_idx_buf = write_ro(&context, &queue, &occ_idx);
    let mut out_buf = unsafe {
        Buffer::<cl_ulong>::create(&context, CL_MEM_READ_WRITE, app.out_cap, ptr::null_mut())
    }
    .expect("out buf");
    let mut count_buf =
        unsafe { Buffer::<cl_uint>::create(&context, CL_MEM_READ_WRITE, 1, ptr::null_mut()) }
            .expect("count buf");
    // Valid magics from phase 2 — expected to be near-empty.
    let valid_cap = 1 << 20;
    let mut valid_buf =
        unsafe { Buffer::<cl_ulong>::create(&context, CL_MEM_READ_WRITE, valid_cap, ptr::null_mut()) }
            .expect("valid buf");
    let mut valid_count_buf =
        unsafe { Buffer::<cl_uint>::create(&context, CL_MEM_READ_WRITE, 1, ptr::null_mut()) }
            .expect("valid count buf");

    // Self-check the enumeration on the smallest popcount against a CPU count.
    let span_full: cl_ulong = if span_high >= 63 { u64::MAX } else { (1u64 << (span_high + 1)) - 1 };
    let complement_arg: cl_uint = app.complement as cl_uint;

    // Self-check the enumeration on a small popcount against a CPU reference.
    {
        let m = app.min_bits.max(1).min(n);
        let total = binom_at(n, m);
        let cpu = cpu_precheck_count(m, span_high, bits, app.complement, span_full, &rel_k);
        let kind = if app.complement { "complement" } else { "direct" };
        eprintln!("self-check m={m} ({kind}): C({n},{m})={total}, CPU precheck survivors={cpu}");
    }

    // --- Enumerate each popcount ------------------------------------------
    let mut grand_valid = 0u64;
    // Tile so each tile's survivors (<= tile candidates) fit the buffer.
    let tile: u64 = app.out_cap as u64;

    for m in app.min_bits..=app.max_bits.min(n) {
        let total = binom_at(n, m);
        if total == 0 {
            continue;
        }
        let magic_bits = if app.complement { n - m } else { m };
        let t0 = Instant::now();
        let mut survivors_total = 0u64;
        let mut valid_all = Vec::new();

        let mut base = 0u64;
        while base < total {
            let tile_total = tile.min(total - base);

            // --- Phase 1: enumerate a tile + window precheck -> survivors. ---
            unsafe { queue.enqueue_write_buffer(&mut count_buf, CL_BLOCKING, 0, &[0u32], &[]) }.unwrap();
            let num_threads = tile_total.div_ceil(app.chunk as u64) as usize;
            let local = 64usize;
            let global = num_threads.div_ceil(local) * local;
            let (n_arg, m_arg, total_arg, base_arg, chunk_arg) =
                (n as cl_uint, m as cl_uint, total as cl_ulong, base as cl_ulong, app.chunk);
            let ev = unsafe {
                ExecuteKernel::new(&exhaust)
                    .set_arg(&binom_buf)
                    .set_arg(&n_arg)
                    .set_arg(&m_arg)
                    .set_arg(&total_arg)
                    .set_arg(&base_arg)
                    .set_arg(&chunk_arg)
                    .set_arg(&complement_arg)
                    .set_arg(&span_full)
                    .set_arg(&rel_k_buf)
                    .set_arg(&out_buf)
                    .set_arg(&count_buf)
                    .set_global_work_size(global)
                    .set_local_work_size(local)
                    .enqueue_nd_range(&queue)
                    .expect("launch")
            };
            ev.wait().expect("wait");

            let mut count = [0u32];
            unsafe { queue.enqueue_read_buffer(&mut count_buf, CL_BLOCKING, 0, &mut count, &[]) }.unwrap();
            let stored = count[0]; // <= tile_total <= out_cap, so no overflow
            survivors_total += stored as u64;

            // --- Phase 2: GPU full-scores survivors -> valid magics only. ---
            if stored > 0 {
                unsafe { queue.enqueue_write_buffer(&mut valid_count_buf, CL_BLOCKING, 0, &[0u32], &[]) }.unwrap();
                let vglobal = (stored as usize) * app.wg;
                let ev = unsafe {
                    ExecuteKernel::new(&validate)
                        .set_arg(&occ_bb_buf)
                        .set_arg(&occ_idx_buf)
                        .set_arg(&out_buf)
                        .set_arg(&stored)
                        .set_arg(&valid_buf)
                        .set_arg(&valid_count_buf)
                        .set_global_work_size(vglobal)
                        .set_local_work_size(app.wg)
                        .enqueue_nd_range(&queue)
                        .expect("validate launch")
                };
                ev.wait().expect("validate wait");

                let mut vcount = [0u32];
                unsafe { queue.enqueue_read_buffer(&mut valid_count_buf, CL_BLOCKING, 0, &mut vcount, &[]) }.unwrap();
                let nvalid = (vcount[0] as usize).min(valid_cap);
                if nvalid > 0 {
                    let mut vout = vec![0u64; nvalid];
                    unsafe { queue.enqueue_read_buffer(&mut valid_buf, CL_BLOCKING, 0, &mut vout, &[]) }.unwrap();
                    valid_all.extend(vout.into_iter().filter(|&mg| cpu_valid(mg, &occ, bits)));
                }
            }

            base += tile_total;
            if total > tile {
                let secs = t0.elapsed().as_secs_f64();
                eprint!(
                    "\rm={m} ({magic_bits}-bit magics): {:.1}% ({:.0} M/s), {} valid   ",
                    100.0 * base as f64 / total as f64,
                    base as f64 / secs / 1e6,
                    valid_all.len(),
                );
            }
        }

        grand_valid += valid_all.len() as u64;
        let secs = t0.elapsed().as_secs_f64();
        eprintln!(
            "\rm={m} ({magic_bits}-bit magics): {total} candidates in {secs:.1}s ({:.0} M/s), {survivors_total} survivors, {} valid          ",
            total as f64 / secs / 1e6,
            valid_all.len(),
        );
        for magic in valid_all {
            println!("valid magic ({magic_bits} bits): {magic:#x}");
        }
    }

    eprintln!("done: {grand_valid} valid magic(s) found");
}

/// CPU reference: count magics of popcount `m` (bits in [0, span_high]) that
/// pass the window precheck. Only used for small `m` (self-check).
fn cpu_precheck_count(
    m: u32,
    span_high: u32,
    bits: u32,
    complement: bool,
    span_full: u64,
    rel_k: &[cl_uint],
) -> u64 {
    #[allow(clippy::too_many_arguments)]
    fn rec(start: u32, span_high: u32, left: u32, mask: u64, bits: u32, comp: u64, rel_k: &[cl_uint], count: &mut u64) {
        if left == 0 {
            if precheck_cpu(mask ^ comp, bits, rel_k) {
                *count += 1;
            }
            return;
        }
        for b in start..=span_high {
            rec(b + 1, span_high, left - 1, mask | (1 << b), bits, comp, rel_k, count);
        }
    }
    let comp = if complement { span_full } else { 0 };
    let mut count = 0;
    rec(0, span_high, m, 0, bits, comp, rel_k, &mut count);
    count
}

/// The same window necessary condition the kernel checks: each single-blocker
/// window value is non-zero and pairwise distinct.
fn precheck_cpu(magic: u64, bits: u32, rel_k: &[cl_uint]) -> bool {
    let shift = 64 - bits;
    let mut vals = [0u64; 16];
    for (i, &k) in rel_k.iter().enumerate() {
        let v = (magic << k) >> shift;
        if v == 0 || vals[..i].contains(&v) {
            return false;
        }
        vals[i] = v;
    }
    true
}

/// Exact validity: does `magic` map every occupancy consistently at `bits`?
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

// --- Occupancy generation (mirrors magic-find.rs) -------------------------

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
