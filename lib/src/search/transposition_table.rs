use std::fmt::Debug;
use std::mem::size_of;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::{iter, mem};

use itertools::Itertools;
use log::info;

use crate::position::Position;
use crate::types::{Move, Value};

type KeyAndEntry = (AtomicU64, AtomicU64);

/// Fixed size hash table for positions. Some implementation details borrowed from the intmap crate
pub struct TranspositionTable {
    data: Vec<KeyAndEntry>,
    mod_mask: usize,
    count: AtomicUsize,
    capacity: usize,
}

impl TranspositionTable {
    /// Creates a table with 16 MB of space
    pub fn new() -> Self {
        Self::with_hash_size(16)
    }

    /// Creates a table with a given `hash_size` in MB rounded down to the nearest power of two
    pub fn with_hash_size(hash_size: usize) -> Self {
        if hash_size == 0 {
            panic!("Attempt to allocate empty transposition table");
        }

        let capacity = hash_size * (1 << 20) / size_of::<KeyAndEntry>();
        let actual_capacity = if capacity.is_power_of_two() {
            capacity
        } else {
            capacity.next_power_of_two() >> 1
        };

        info!(
            "Allocating trans. table with capacity {} (actual {})",
            capacity, actual_capacity
        );

        let data = iter::repeat_with(|| (AtomicU64::new(0), AtomicU64::new(0)))
            .take(actual_capacity)
            .collect_vec();
        let mod_mask = actual_capacity - 1;
        let count = AtomicUsize::new(0);

        info!("Done allocating trans. table");

        Self {
            data,
            mod_mask,
            count,
            capacity: actual_capacity,
        }
    }

    /// Inserts `entry` if its entry score is higher than that of the stored entry.
    pub fn insert(&self, position: &Position, entry: Entry) {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        // Safety: Modulo ensures that index is in bounds
        let (key_cell, entry_cell) = unsafe { self.data.get_unchecked(index) };
        // let masked_key = key_cell.load(Ordering::Acquire);
        let packed_entry = entry_cell.load(Ordering::Acquire);

        match Entry::from_u64(packed_entry) {
            Some(e) => {
                if entry.entry_score() > e.entry_score() {
                    let new_entry = entry.to_u64();
                    key_cell.store(key ^ new_entry, Ordering::Release);
                    entry_cell.store(new_entry, Ordering::Release);
                }
            }
            None => {
                let new_entry = entry.to_u64();
                key_cell.store(key ^ new_entry, Ordering::Release);
                if entry_cell
                    .compare_exchange(0, new_entry, Ordering::AcqRel, Ordering::Relaxed)
                    .is_ok()
                {
                    self.count.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
    }

    pub fn get(&self, position: &Position) -> Option<Entry> {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        let (key_cell, entry_cell) = unsafe { self.data.get_unchecked(index) };
        let masked_key = key_cell.load(Ordering::Acquire);
        let packed_entry = entry_cell.load(Ordering::Acquire);

        if masked_key ^ packed_entry == key {
            Entry::from_u64(packed_entry)
        } else {
            None
        }
    }

    pub fn clear(&mut self) {
        self.data
            .fill_with(|| (AtomicU64::new(0), AtomicU64::new(0)));
        self.count = AtomicUsize::new(0);
    }

    pub fn len(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

impl Default for TranspositionTable {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy)]
pub struct Entry {
    pub score: Value,
    pub best_move: Move,
    pub bound: Bound,
    pub depth: i8,
}

impl Entry {
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

        let score = ((val & 0xFFFF) as i16).into();
        let best_move = (((val >> 16) & 0xFFFF) as u16).into();
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
