// GPU magic-bitboard hill climber. One workgroup per independent climber; the
// workgroup's threads cooperatively score a candidate into a hash table held in
// local (LDS) memory. Kicks pull from an elite buffer the host refreshes each
// launch; the host manages the elite pool and validates solutions on the CPU.
//
// Compile-time defines (set by the host): BITS, NUM_OCC, SPAN_HIGH, WG, STAG,
// KICK_BASE, KICK_GROW, KICK_MAX.

#define EMPTY 0xffffffffu
#define TABLE_SIZE (1u << BITS)
#define SHIFT (64u - BITS)

// xorshift64* PRNG, state advanced by the workgroup's leader only.
inline ulong rng_next(__local ulong *s) {
    ulong x = *s;
    x ^= x >> 12;
    x ^= x << 25;
    x ^= x >> 27;
    *s = x;
    return x * 0x2545F4914F6CDD1DUL;
}

inline uint rng_range(__local ulong *s, uint n) {
    return (uint)(rng_next(s) % (ulong)n);
}

// Flips scaled to fitness: careful near the summit, big jumps in the flatlands.
inline uint mutation_size(uint score) {
    if (25u * score >= 24u * NUM_OCC) return 1u;
    if (9u * score >= 8u * NUM_OCC) return 2u;
    if (3u * score >= 2u * NUM_OCC) return 3u;
    return 4u;
}

inline ulong do_mutate(ulong m, uint flips, __local ulong *s) {
    for (uint i = 0; i < flips; i++) {
        m ^= (1UL << rng_range(s, SPAN_HIGH + 1u));
    }
    return m;
}

// Cooperatively score `magic`: the number of occupancies that map consistently.
// A full score (NUM_OCC) means a valid magic. Two scoring modes, selected at
// build time; both are exact for *validity* (0 conflicts iff a valid magic),
// only the conflict count for non-solutions may differ. Which is faster is
// hardware-dependent: the atomic mode does the 64-bit multiply once but leans
// on LDS atomic throughput; RACE_FREE avoids atomics but multiplies twice.
inline uint score_magic(__global const ulong *occ_bb, __global const uint *occ_idx,
                        ulong magic, __local uint *table, __local uint *conflicts,
                        uint lid) {
#ifdef RACE_FREE
    // Atomic-free two-pass. Pass 1 plainly writes each occupancy's index to its
    // slot (a multi-way collision resolves to an arbitrary race winner); pass 2
    // recomputes the slot and counts an occupancy as a conflict if its slot no
    // longer holds its own index. A valid magic gives one index per slot, so
    // every occupancy reads its own index (no false negatives); any slot with
    // two distinct indices has a non-winner that reads a different index (no
    // false positives). No table clear is needed: pass 2 only reads slots pass
    // 1 wrote. Recomputing the slot beats caching it, which would spill a
    // dynamically-indexed private array to scratch memory.
    for (uint i = lid; i < NUM_OCC; i += WG) {
        table[(uint)((occ_bb[i] * magic) >> SHIFT)] = occ_idx[i];
    }
    if (lid == 0) *conflicts = 0u;
    barrier(CLK_LOCAL_MEM_FENCE);

    uint myc = 0u;
    for (uint i = lid; i < NUM_OCC; i += WG) {
        uint slot = (uint)((occ_bb[i] * magic) >> SHIFT);
        if (table[slot] != occ_idx[i]) myc++;
    }
    atomic_add(conflicts, myc); // one reduction atomic per thread, not per occupancy
    barrier(CLK_LOCAL_MEM_FENCE);
    return NUM_OCC - *conflicts;
#else
    // Single-pass with LDS atomics: first writer of a slot wins; a later,
    // different attack set is a conflict. Multiplies once but clears the table
    // and does one atomic per occupancy.
    for (uint i = lid; i < TABLE_SIZE; i += WG) {
        table[i] = EMPTY;
    }
    if (lid == 0) *conflicts = 0u;
    barrier(CLK_LOCAL_MEM_FENCE);

    uint myc = 0u;
    for (uint i = lid; i < NUM_OCC; i += WG) {
        uint slot = (uint)((occ_bb[i] * magic) >> SHIFT);
        uint idx = occ_idx[i];
        uint prev = atomic_cmpxchg(&table[slot], EMPTY, idx);
        if (prev != EMPTY && prev != idx) myc++;
    }
    atomic_add(conflicts, myc);
    barrier(CLK_LOCAL_MEM_FENCE);
    return NUM_OCC - *conflicts;
#endif
}

__kernel void climb(__global const ulong *occ_bb,
                    __global const uint *occ_idx,
                    __global ulong *cur_magic,
                    __global ulong *best_magic,
                    __global uint *best_score,
                    __global ulong *rng_state,
                    __global uint *stag_state,
                    __global uint *kick_state,
                    __global const ulong *elites,
                    uint num_elites,
                    uint iters) {
    uint g = get_group_id(0);
    uint lid = get_local_id(0);

    __local uint table[TABLE_SIZE];
    __local uint conflicts;
    __local ulong l_cur, l_cand, l_rng, l_best;
    __local uint l_cur_score, l_best_score, l_stag, l_kick;

    if (lid == 0) {
        l_cur = cur_magic[g];
        l_rng = rng_state[g];
        l_best = best_magic[g];
        l_best_score = best_score[g];
        l_stag = stag_state[g];
        l_kick = kick_state[g];
    }
    barrier(CLK_LOCAL_MEM_FENCE);

    uint cs = score_magic(occ_bb, occ_idx, l_cur, table, &conflicts, lid);
    if (lid == 0) l_cur_score = cs;
    barrier(CLK_LOCAL_MEM_FENCE);

    for (uint it = 0; it < iters; it++) {
        if (lid == 0) {
            ulong cand;
            if (l_stag >= STAG) {
                // Kick: perturb a random elite (or our own best early on) and
                // widen the kick, resetting the stagnation counter.
                ulong base = (num_elites > 0u) ? elites[rng_range(&l_rng, num_elites)] : l_best;
                cand = do_mutate(base, l_kick, &l_rng);
                l_stag = 0u;
                l_kick = min(l_kick + (uint)KICK_GROW, (uint)KICK_MAX);
            } else {
                cand = do_mutate(l_cur, mutation_size(l_cur_score), &l_rng);
            }
            l_cand = cand;
        }
        barrier(CLK_LOCAL_MEM_FENCE);

        uint sc = score_magic(occ_bb, occ_idx, l_cand, table, &conflicts, lid);

        if (lid == 0) {
            if (sc >= l_cur_score) {
                l_cur = l_cand;
                l_cur_score = sc;
            }
            if (sc > l_best_score) {
                l_best = l_cand;
                l_best_score = sc;
                l_stag = 0u;
                l_kick = (uint)KICK_BASE;
            } else {
                l_stag++;
            }
        }
        barrier(CLK_LOCAL_MEM_FENCE);
        if (l_best_score == NUM_OCC) break;
    }

    if (lid == 0) {
        cur_magic[g] = l_cur;
        rng_state[g] = l_rng;
        best_magic[g] = l_best;
        best_score[g] = l_best_score;
        stag_state[g] = l_stag;
        kick_state[g] = l_kick;
    }
}
