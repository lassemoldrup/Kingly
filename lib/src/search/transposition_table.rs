use std::fmt::Debug;
use std::iter;
use std::mem::size_of;
use std::sync::atomic::{AtomicUsize, Ordering};

use log::info;
use parking_lot::{RwLock, RwLockUpgradableReadGuard};

use crate::position::Position;
use crate::types::{Move, Value};

const CLUSTER_SIZE: usize = 3;
type Cluster = RwLock<[Option<(u64, Entry)>; CLUSTER_SIZE]>;

/// Fixed size hash table for positions. Some implementation details borrowed from the intmap crate
pub struct TranspositionTable {
    data: Vec<Cluster>,
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

        let num_clusters = hash_size * (1 << 20) / size_of::<Cluster>();
        let actual_num_clusters = if num_clusters.is_power_of_two() {
            num_clusters
        } else {
            num_clusters.next_power_of_two() / 2
        };
        let capacity = actual_num_clusters * CLUSTER_SIZE;

        info!(
            "Allocating trans. table with capacity {} (actual {capacity})",
            num_clusters * CLUSTER_SIZE
        );

        let data = iter::repeat_with(|| RwLock::new([None; CLUSTER_SIZE]))
            .take(actual_num_clusters)
            .collect();
        let mod_mask = actual_num_clusters - 1;
        let count = AtomicUsize::new(0);

        info!("Done allocating trans. table");

        Self {
            data,
            mod_mask,
            count,
            capacity,
        }
    }

    /// Inserts an entry. In case no free spot is found using linear probing
    /// a combination of entry depth and age is used to determine which gets replaced.
    pub fn insert(&self, position: &Position, entry: Entry) {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        let mut min = entry.entry_score();
        let mut min_idx = CLUSTER_SIZE;
        // Safety: Modulo ensures that index is in bounds
        let cluster = unsafe { self.data.get_unchecked(index).upgradable_read() };
        // Figure out which entry, if any, should be replaced
        for (i, entry_opt) in cluster.iter().enumerate() {
            match entry_opt {
                Some((k, _)) if *k == key => {
                    min_idx = i;
                    break;
                }
                Some((_, e)) if e.entry_score() < min => {
                    min = e.entry_score();
                    min_idx = i;
                }
                Some(_) => {}
                None => {
                    min_idx = i;
                    break;
                }
            }
        }

        if min_idx == CLUSTER_SIZE {
            // No replacement
            return;
        }

        let mut cluster = RwLockUpgradableReadGuard::upgrade(cluster);
        let entry_opt = unsafe { cluster.get_unchecked_mut(min_idx) };
        if let None = entry_opt {
            self.count.fetch_add(1, Ordering::Relaxed);
        }
        *entry_opt = Some((key, entry));
    }

    pub fn get(&self, position: &Position) -> Option<Entry> {
        let key = position.zobrist;
        let index = key as usize & self.mod_mask;

        // Safety: Modulo ensures that index is in bounds
        let cluster = unsafe { self.data.get_unchecked(index).read() };
        for &entry_opt in cluster.iter() {
            match entry_opt {
                Some((k, e)) if k == key => {
                    return Some(e);
                }
                Some(_) => {}
                None => return None,
            }
        }

        None
    }

    pub fn clear(&mut self) {
        self.data.fill_with(|| RwLock::new([None; CLUSTER_SIZE]));
        self.count.store(0, Ordering::Relaxed);
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

    fn entry_score(&self) -> i8 {
        self.depth
            + match self.bound {
                Bound::Exact => 1,
                Bound::Lower | Bound::Upper => 0,
            }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Bound {
    Exact,
    Lower,
    Upper,
}
