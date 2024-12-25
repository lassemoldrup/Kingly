use std::fmt::Debug;
use std::mem::size_of;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::{iter, mem};

use itertools::Itertools;

use crate::position::Position;
use crate::types::{Move, Value};

type KeyAndEntry = (AtomicU64, AtomicU64);

/// Fixed size hash table for transpositions between positions.
pub struct TranspositionTable {
    data: Vec<KeyAndEntry>,
    count: AtomicUsize,
    capacity: usize,
}

impl TranspositionTable {
    /// Creates a table with 16 MB of space.
    pub fn new() -> Self {
        Self::with_hash_size(16)
    }

    /// Creates a table with a given `hash_size` in MB
    pub fn with_hash_size(hash_size: usize) -> Self {
        if hash_size == 0 {
            panic!("Attempt to allocate empty transposition table");
        }

        log::info!("Allocating transposition table with {} MB...", hash_size);

        let capacity = hash_size * (1 << 20) / size_of::<KeyAndEntry>();
        let data = iter::repeat_with(|| (AtomicU64::new(0), AtomicU64::new(0)))
            .take(capacity)
            .collect_vec();
        let count = AtomicUsize::new(0);

        log::info!("Allocation finished.");

        Self {
            data,
            count,
            capacity,
        }
    }

    /// Inserts `entry` if its entry score is higher than that of the stored
    /// entry.
    #[inline]
    pub fn insert(&self, position: &Position, entry: Entry) {
        let key = position.zobrist;
        let index = key as usize % self.capacity;

        // Safety: Modulo ensures that index is in bounds
        let (key_cell, entry_cell) = unsafe { self.data.get_unchecked(index) };
        let packed_entry = entry_cell.load(Ordering::Relaxed);

        // Concurrency: The fact that the key cell is stored as `key ^ entry` means we
        // do not need to worry about the key and entry being out of sync.
        match Entry::from_u64(packed_entry) {
            Some(e) => {
                if entry.entry_score() > e.entry_score() {
                    let new_entry = entry.to_u64();
                    key_cell.store(key ^ new_entry, Ordering::Relaxed);
                    entry_cell.store(new_entry, Ordering::Relaxed);
                }
            }
            None => {
                let new_entry = entry.to_u64();
                key_cell.store(key ^ new_entry, Ordering::Relaxed);
                if entry_cell
                    .compare_exchange(0, new_entry, Ordering::Relaxed, Ordering::Relaxed)
                    .is_ok()
                {
                    self.count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    /// Retrieves the entry for a given position.
    #[inline]
    pub fn get(&self, position: &Position) -> Option<Entry> {
        let key = position.zobrist;
        let index = key as usize % self.capacity;

        // Safety: Modulo ensures that index is in bounds
        let (key_cell, entry_cell) = unsafe { self.data.get_unchecked(index) };
        let masked_key = key_cell.load(Ordering::Relaxed);
        let packed_entry = entry_cell.load(Ordering::Relaxed);

        // Check that the key matches the entry
        if masked_key ^ packed_entry == key {
            Entry::from_u64(packed_entry)
        } else {
            None
        }
    }

    pub fn clear(&self) {
        for (key, entry) in &self.data {
            key.store(0, Ordering::Relaxed);
            entry.store(0, Ordering::Relaxed);
        }
        // Concurrency: Release/acquire ensures that no other thread can observe
        // the count being set to 0 before the data is cleared.
        self.count.store(0, Ordering::Release);
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.count.load(Ordering::Acquire)
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
}
/// An entry in the transposition table.
#[derive(Clone, Copy)]
pub struct Entry {
    /// The score of the position.
    pub score: Value,
    /// The best move found for the position.
    pub best_move: Move,
    /// The type of bound for the score.
    pub bound: Bound,
    /// The depth of the search that produced the score.
    pub depth: i8,
}

impl Entry {
    #[inline]
    pub fn new(score: Value, best_move: Move, bound: Bound, depth: i8) -> Self {
        Self {
            score,
            best_move,
            bound,
            depth,
        }
    }

    fn from_u64(val: u64) -> Option<Self> {
        if val == 0 {
            return None;
        }

        // Safety: We know that the value is a valid score
        let score = unsafe { Value::from_inner((val & 0xFFFF) as i16) };
        // Safety: We know that the value is a valid move
        let best_move = unsafe { Move::from_inner(((val >> 16) & 0xFFFF) as u16) };
        let bound = (((val >> 32) & 0xFF) as u8).into();
        let depth = (val >> 40) as i8;
        Some(Entry::new(score, best_move, bound, depth))
    }

    fn to_u64(&self) -> u64 {
        let mut res = self.score.into_inner() as u16 as u64;
        res |= (self.best_move.into_inner() as u64) << 16;
        res |= (self.bound as u64) << 32;
        res |= (self.depth as u8 as u64) << 40;
        res
    }

    fn entry_score(&self) -> i8 {
        self.depth
            + match self.bound {
                Bound::Exact => 1,
                Bound::Lower | Bound::Upper => 0,
            }
    }
}

/// An enum indicating whether a score is exact, a lower bound, or an upper
/// bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Bound {
    Exact = 1,
    Lower = 2,
    Upper = 3,
}

impl From<u8> for Bound {
    fn from(value: u8) -> Self {
        unsafe { mem::transmute(value) }
    }
}
