use crate::{standard::Position, framework::{value::Value, moves::Move}};

/// Fixed size hash table for positions. Some implementation details borrowed from the IntMap crate
pub struct TranspositionTable {
    data: Vec<Option<(u64, Entry)>>,
    mod_mask: usize,
    count: usize,
}

impl TranspositionTable {
    const PROBE_DEPTH: usize = 2;

    pub fn new() -> Self {
        Self::with_capacity(1 << 22)
    }

    /// Creates a table with a given `capacity` rounded up to the nearest power of two
    pub fn with_capacity(capacity: usize) -> Self {
        let capacity = capacity.next_power_of_two();

        let data = vec![None; capacity];
        let mod_mask = capacity - 1;
        let count = 0;

        Self {
            data,
            mod_mask,
            count,
        }
    }

    /// Inserts an entry. In case no free spot is found using linear probing,
    /// a combination of entry depth and age is used to determine which gets replaced
    pub fn insert(&mut self, position: &Position, entry: Entry) {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;
        
        let mut min = u32::MAX;
        let mut min_idx = 0;
        for i in self.probe(index) {
            // Safety: Modulo ensures that index is in bounds
            let entry_opt = unsafe { 
                self.data.get_unchecked_mut(i)
            };
            match entry_opt {
                Some((k, e)) =>  {
                    if *k == key {
                        *e = entry;
                        return;
                    }

                    if e.entry_score < min {
                        min = e.entry_score;
                        min_idx = i;
                    }
                },
                None => {
                    *entry_opt = Some((key, entry));
                    self.count += 1;
                    return;
                },
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
            let entry_opt = unsafe { 
                self.data.get_unchecked(i)
            };
            match entry_opt {
                Some((k, e)) =>  {
                    if *k == key {
                        return Some(e);
                    }
                },
                None => return None,
            }
        }

        None
    }

    fn probe(&self, index: usize) -> impl Iterator<Item = usize> {
        let mod_mask = self.mod_mask;
        (0..=Self::PROBE_DEPTH)
            .map(move |i| (index + i) & mod_mask)
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

#[derive(Clone)]
pub struct Entry {
    pub score: Value,
    pub best_move: Move,
    pub bound: Bound,
    pub depth: u32,
    entry_score: u32,
}

impl Entry {
    pub fn new(score: Value, best_move: Move, kind: Bound, depth: u32) -> Self {
        let entry_score = depth + match kind {
            Bound::Exact => 1,
            Bound::Lower | Bound::Upper => 0,
        };

        Self {
            score,
            best_move,
            bound: kind,
            depth,
            entry_score
        }
    }
}

#[derive(Clone, Copy)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
} 