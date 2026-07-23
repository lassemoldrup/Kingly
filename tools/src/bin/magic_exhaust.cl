// Exhaustive GPU magic checker. The host splits the space of magics with a
// fixed popcount `m` (bits chosen from positions [0, N)) into contiguous index
// ranges via the combinatorial number system; each work-item unranks its start
// into a bitmask and walks a chunk with Gosper's hack (no stack, no arrays, no
// coordination). Candidates failing the single-blocker window necessary
// condition (zero or duplicate window) are skipped; survivors are emitted for
// the host to validate exactly on the CPU.
//
// Compile-time defines (host): BITS, SHIFT (= 64 - BITS), NUM_REL, STRIDE
// (binomial-table row stride), OUT_CAP (survivor buffer capacity).

#define TABLE_SIZE (1u << BITS)

inline ulong binom(__global const ulong *t, uint c, uint k) {
    return t[c * STRIDE + k];
}

// Unrank `idx` in [0, C(n, m)) to the m-subset bitmask, in colexicographic
// order (which equals increasing numeric order of the bitmask, so it lines up
// with Gosper's hack below).
inline ulong unrank(ulong idx, uint n, uint m, __global const ulong *t) {
    ulong magic = 0UL;
    ulong rem = idx;
    uint upper = n;
    for (uint k = m; k >= 1u; k--) {
        uint c = upper - 1u;
        while (binom(t, c, k) > rem) c--; // stops: C(c, k) = 0 for c < k
        magic |= (1UL << c);
        rem -= binom(t, c, k);
        upper = c;
    }
    return magic;
}

// Next bitmask with the same popcount, in increasing numeric order.
inline ulong gosper_next(ulong x) {
    ulong c = x & (~x + 1UL); // lowest set bit
    ulong r = x + c;
    uint sh = 63u - (uint)clz(c); // log2(c); c is a power of two
    return (((r ^ x) >> 2) >> sh) | r;
}

// Necessary condition: each single-blocker window value (top BITS bits of
// `magic << k`) must be non-zero and pairwise distinct.
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

// Phase 1: enumerate a chunk of the m-subset space and emit window-precheck
// survivors for phase 2.
__kernel void exhaust(__global const ulong *binom_t,
                      uint n,
                      uint m,
                      ulong total,
                      uint chunk,
                      __constant const uint *rel_k,
                      __global ulong *out,
                      __global uint *out_count) {
    ulong start = (ulong)get_global_id(0) * (ulong)chunk;
    if (start >= total) return;

    ulong magic = unrank(start, n, m, binom_t);
    ulong count = min((ulong)chunk, total - start);
    for (ulong i = 0; i < count; i++) {
        if (precheck(magic, rel_k)) {
            uint pos = atomic_inc(out_count);
            if (pos < OUT_CAP) out[pos] = magic;
        }
        magic = gosper_next(magic);
    }
}

// Phase 2: one workgroup per survivor cooperatively full-scores it into an LDS
// hash table (exact, via atomic_cmpxchg) and emits only valid magics, so the
// host validates ~nothing. Defines: TABLE_SIZE (= 1 << BITS), WG, NUM_OCC.
__kernel void validate(__global const ulong *occ_bb,
                       __global const uint *occ_idx,
                       __global const ulong *survivors,
                       uint num_survivors,
                       __global ulong *valid_out,
                       __global uint *valid_count) {
    uint g = get_group_id(0);
    if (g >= num_survivors) return;
    uint lid = get_local_id(0);
    ulong magic = survivors[g];

    __local uint table[TABLE_SIZE];
    __local uint conflicts;
    for (uint i = lid; i < TABLE_SIZE; i += WG) table[i] = 0xffffffffu;
    if (lid == 0) conflicts = 0u;
    barrier(CLK_LOCAL_MEM_FENCE);

    uint myc = 0u;
    for (uint i = lid; i < NUM_OCC; i += WG) {
        uint slot = (uint)((occ_bb[i] * magic) >> SHIFT);
        uint idx = occ_idx[i];
        uint prev = atomic_cmpxchg(&table[slot], 0xffffffffu, idx);
        if (prev != 0xffffffffu && prev != idx) myc++;
    }
    atomic_add(&conflicts, myc);
    barrier(CLK_LOCAL_MEM_FENCE);

    if (lid == 0 && conflicts == 0u) {
        uint pos = atomic_inc(valid_count);
        if (pos < OUT_CAP) valid_out[pos] = magic;
    }
}
