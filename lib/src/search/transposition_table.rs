use std::fmt::Debug;
use std::iter;
use std::mem::{self, size_of};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use itertools::Itertools;
use log::info;

use crate::position::Position;
use crate::types::{Move, Value};

/// Fixed size hash table for positions. Some implementation details borrowed from the intmap crate
pub struct TranspositionTable {
    data: Vec<(AtomicU64, AtomicEntry)>,
    mod_mask: usize,
    count: AtomicUsize,
    capacity: usize,
}

impl TranspositionTable {
    const PROBE_DEPTH: usize = 2;

    /// Creates a table with 16 MB of space
    pub fn new() -> Self {
        Self::with_hash_size(16)
    }

    /// Creates a table with a given `hash_size` in MB rounded down to the nearest power of two
    pub fn with_hash_size(hash_size: usize) -> Self {
        let capacity = hash_size * (1 << 20) / size_of::<(AtomicU64, AtomicEntry)>();
        let actual_capacity = if capacity.is_power_of_two() {
            capacity
        } else {
            capacity.next_power_of_two() >> 1
        };

        info!(
            "Allocating trans. table with capacity {} (actual {})",
            capacity, actual_capacity
        );

        let data = iter::repeat_with(|| (AtomicU64::new(0), AtomicEntry::default()))
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

    /// Inserts an entry. In case no free spot is found using linear probing,
    /// a combination of entry depth and age is used to determine which gets replaced
    pub fn insert(&self, position: &Position, entry: Entry) {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        let mut min = i8::MAX;
        let mut min_idx = 0;
        let mut min_entry_u64 = 0;
        let mut min_key = 0;
        for i in self.probe(index) {
            // Safety: Modulo ensures that index is in bounds
            let (locked_key, locked_entry) = unsafe { self.data.get_unchecked(i) };
            let entry_u64 = locked_entry.get_u64();
            let entry_opt = Entry::from_u64(entry_u64);
            let k = locked_key.load(Ordering::Relaxed);
            match entry_opt {
                Some(e) if k == key => {
                    if entry.entry_score() > e.entry_score() {
                        locked_entry.insert(entry_u64, entry);
                    }
                    return;
                }
                Some(e) if e.entry_score() < min => {
                    min = e.entry_score();
                    min_idx = i;
                    min_entry_u64 = entry_u64;
                    min_key = k;
                }
                Some(_) => {}
                None => {
                    if let Ok(_) =
                        locked_key.compare_exchange(0, key, Ordering::Relaxed, Ordering::Relaxed)
                    {
                        locked_entry.insert(0, entry);
                        self.count.fetch_add(1, Ordering::Relaxed);
                    }
                    return;
                }
            }
        }

        // Determine which node to be replaced
        if entry.entry_score() > min {
            let (locked_key, locked_entry) = unsafe { self.data.get_unchecked(min_idx) };
            if let Ok(_) =
                locked_key.compare_exchange(min_key, key, Ordering::Relaxed, Ordering::Relaxed)
            {
                locked_entry.insert(min_entry_u64, entry);
            }
        }
    }

    pub fn get(&self, position: &Position) -> Option<Entry> {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        for i in self.probe(index) {
            // Safety: Modulo ensures that index is in bounds
            let (locked_key, locked_entry) = unsafe { self.data.get_unchecked(i) };
            let entry_opt = Entry::from_u64(locked_entry.get_u64());
            let k = locked_key.load(Ordering::Relaxed);
            match entry_opt {
                Some(e) if k == key => {
                    return Some(e);
                }
                Some(_) => {}
                None => return None,
            }
        }

        None
    }

    fn probe(&self, index: usize) -> impl Iterator<Item = usize> {
        let mod_mask = self.mod_mask;
        (0..=Self::PROBE_DEPTH).map(move |i| (index + i) & mod_mask)
    }

    pub fn clear(&mut self) {
        self.data
            .fill_with(|| (AtomicU64::new(0), AtomicEntry::default()));
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

#[derive(Default)]
struct AtomicEntry(AtomicU64);

impl AtomicEntry {
    fn insert(&self, old: u64, entry: Entry) -> Result<u64, u64> {
        let mut inner_u64 = 1;

        inner_u64 |= (entry.score.into_inner() as u16 as u64) << 1;
        inner_u64 |= (entry.best_move.into_inner() as u64) << (1 + 16);
        inner_u64 |= (entry.bound as u64) << (1 + 16 + 16);
        inner_u64 |= (entry.depth as u8 as u64) << (1 + 16 + 16 + 8);

        self.0
            .compare_exchange(old, inner_u64, Ordering::Relaxed, Ordering::Relaxed)
    }

    fn get_u64(&self) -> u64 {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Entry {
    pub score: Value,
    pub best_move: Move,
    pub bound: Bound,
    pub depth: i8,
    // entry_score: i8,
}

impl Entry {
    pub fn new(score: Value, best_move: Move, bound: Bound, depth: i8) -> Self {
        // let entry_score = depth
        //     + match bound {
        //         Bound::Exact => 1,
        //         Bound::Lower | Bound::Upper => 0,
        //     };

        Self {
            score,
            best_move,
            bound,
            depth,
            // entry_score,
        }
    }

    fn from_u64(mut value: u64) -> Option<Self> {
        if value & 1 == 0 {
            return None;
        }
        value >>= 1;

        let score = Value::centi_pawn((value & 0xFFFF) as i16);
        value >>= 16;

        let best_move = ((value & 0xFFFF) as u16).into();
        value >>= 16;

        let bound = unsafe { mem::transmute((value & 0xFF) as u8) };
        value >>= 8;

        let depth = (value & 0xFF) as i8;

        Some(Entry {
            score,
            best_move,
            bound,
            depth,
        })
    }

    fn entry_score(&self) -> i8 {
        self.depth
            + match self.bound {
                Bound::Exact => 1,
                Bound::Lower | Bound::Upper => 0,
            }
    }
}

#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}
