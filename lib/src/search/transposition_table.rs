use std::fmt::{self, Debug, Formatter};
use std::mem::size_of;

use tracing::info;

use crate::position::Position;
use crate::types::{Move, Value};

/// Fixed size hash table for positions. Some implementation details borrowed from the IntMap crate
pub struct TranspositionTable {
    data: Vec<Option<(u64, Entry)>>,
    mod_mask: usize,
    count: usize,
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
        let capacity = hash_size * (1 << 20) / size_of::<Option<(u64, Entry)>>();
        let actual_capacity = if capacity.is_power_of_two() {
            capacity
        } else {
            capacity.next_power_of_two() >> 1
        };

        info!(
            "Allocating trans. table with capacity {} (actual {})",
            capacity, actual_capacity
        );

        let data = vec![None; actual_capacity];
        let mod_mask = actual_capacity - 1;
        let count = 0;

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
    pub fn insert(&mut self, position: &Position, entry: Entry) {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        let mut min = u8::MAX;
        let mut min_idx = 0;
        for i in self.probe(index) {
            // Safety: Modulo ensures that index is in bounds
            let entry_opt = unsafe { self.data.get_unchecked_mut(i) };
            match entry_opt {
                Some((k, e)) => {
                    if *k == key {
                        *e = entry;
                        return;
                    }

                    if e.entry_score < min {
                        min = e.entry_score;
                        min_idx = i;
                    }
                }
                None => {
                    *entry_opt = Some((key, entry));
                    self.count += 1;
                    return;
                }
            }
        }

        // Determine which node to be replaced
        if entry.entry_score >= min {
            unsafe {
                *self.data.get_unchecked_mut(min_idx) = Some((key, entry));
            }
        }
    }

    pub fn get(&self, position: &Position) -> Option<&Entry> {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        for i in self.probe(index) {
            // Safety: Modulo ensures that index is in bounds
            let entry_opt = unsafe { self.data.get_unchecked(i) };
            match entry_opt {
                Some((k, e)) => {
                    if *k == key {
                        return Some(e);
                    }
                }
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
        self.data.fill(None);
        self.count = 0;
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
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
    pub depth: u8,
    entry_score: u8,
}

impl Entry {
    pub fn new(score: Value, best_move: Move, bound: Bound, depth: u8) -> Self {
        let entry_score = depth
            + match bound {
                Bound::Exact => 1,
                Bound::Lower | Bound::Upper => 0,
            };

        Self {
            score,
            best_move,
            bound,
            depth,
            entry_score,
        }
    }
}

impl Debug for Entry {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        f.debug_struct("Entry")
            .field("score", &self.score)
            .field("best_move", &self.best_move)
            .field("bound", &self.bound)
            .field("depth", &self.depth)
            .finish()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}
