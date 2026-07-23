// GPU magic-bitboard hill climber. One workgroup per independent climber; the
// workgroup's threads cooperatively score a candidate into a hash table held in
// local (LDS) memory. Kicks pull from an elite buffer the host refreshes each
// launch; the host manages the elite pool and validates solutions on the CPU.
//
// Compile-time defines (set by the host): BITS, NUM_OCC, SPAN_HIGH, WG, NUM_REL,
// STAG, KICK_BASE, KICK_GROW, KICK_MAX.

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

// A cheap necessary condition, checked by the leader before the full score:
// each single-blocker occupancy `1 << k` (k a relevant square) has index equal
// to the top BITS bits of `magic << k`, and these must be pairwise distinct and
// non-zero (distinct from the empty board's index 0). A magic failing this
// cannot be valid, so the workgroup can skip the expensive cooperative score.
//
// `rel_k` is in constant memory (cached/broadcast). NUM_REL is tiny (<= 12), so
// the loops unroll; the window values are cached in a small register array.
inline bool precheck(ulong magic, __constant const uint *rel_k) {
    uint vals[NUM_REL];
    for (uint i = 0; i < NUM_REL; i++) {
        uint v = (uint)((magic << rel_k[i]) >> SHIFT);
        if (v == 0u) return false;
        for (uint j = 0; j < i; j++) {
            if (vals[j] == v) return false;
        }
        vals[i] = v;
    }
    return true;
}

// Cooperatively score `magic`: the number of occupancies that map consistently
// (a slot's first writer wins; a later, different attack set is a conflict).
// A full score (NUM_OCC) means a valid magic.
inline uint score_magic(__global const ulong *occ_bb, __global const uint *occ_idx,
                        ulong magic, __local uint *table, __local uint *conflicts,
                        uint lid) {
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
                    __constant const uint *rel_k,
                    uint do_precheck,
                    uint iters) {
    uint g = get_group_id(0);
    uint lid = get_local_id(0);

    __local uint table[TABLE_SIZE];
    __local uint conflicts;
    __local ulong l_cur, l_cand, l_rng, l_best;
    __local uint l_cur_score, l_best_score, l_stag, l_kick, l_ok;

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
            // Fold the precheck into the leader's serial section (no extra
            // barrier): a doomed candidate skips the whole cooperative score.
            l_ok = (do_precheck == 0u || precheck(cand, rel_k)) ? 1u : 0u;
        }
        barrier(CLK_LOCAL_MEM_FENCE);

        // Uniform branch: all threads score together or all skip together, so
        // score_magic's internal barriers are never reached divergently.
        uint sc = l_ok ? score_magic(occ_bb, occ_idx, l_cand, table, &conflicts, lid) : 0u;

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
