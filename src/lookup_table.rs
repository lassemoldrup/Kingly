use std::cell::UnsafeCell;
use std::ops::{Index, IndexMut};
use std::sync::atomic::{AtomicBool, Ordering};

use parking_lot::{lock_api::RawMutex, Mutex};
use crate::types::Bitboard;

/// Speedy boi lookup table
pub struct Lookup<T> where {
    table: UnsafeCell<T>,
    lock: Mutex<()>,
    is_init: AtomicBool,
}

impl<T> Lookup<T> {
    pub const fn new(initial: T) -> Self {
        Lookup {
            table: UnsafeCell::new(initial),
            lock: Mutex::const_new(RawMutex::INIT, ()),
            is_init: AtomicBool::new(false),
        }
    }
    pub fn is_init(&self) -> bool {
        self.is_init.load(Ordering::Relaxed)
    }
    pub fn set<I>(&self, idx: I, val: T::Output)
        where
            T: Index<I>,
            T: IndexMut<I>,
            T::Output: Sized + Copy
    {
        let _lock = self.lock.lock();
        assert!(!self.is_init.load(Ordering::Relaxed));
        unsafe {
            (&mut *self.table.get())[idx] = val;
        }
    }
    pub fn set_init(&self) {
        let _lock = self.lock.lock();
        self.is_init.store(true, Ordering::Relaxed);
    }
    /// # Safety
    /// Call after calling `set_init()`
    pub unsafe fn get<I>(&self, idx: I) -> T::Output
        where
            T: Index<I>,
            T::Output: Sized + Copy
    {
        debug_assert!(self.is_init());

        (&*self.table.get())[idx]
    }
}

unsafe impl<T> Sync for Lookup<T> { }

impl Lookup<[Bitboard; 107_648]> {
    pub fn set_slice(&self, start: usize, end: usize, src: &[Bitboard]) -> &[Bitboard] {
        let _lock = self.lock.lock();
        assert!(!self.is_init.load(Ordering::Relaxed));
        unsafe {
            (&mut *self.table.get())[start..end].copy_from_slice(src);
            &(*self.table.get())[start..end]
        }
    }
}

