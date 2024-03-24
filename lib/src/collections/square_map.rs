use std::fmt::{self, Debug, Formatter};
use std::mem::MaybeUninit;
use std::ops::{Index, IndexMut};
use std::{array, iter};

use strum::IntoEnumIterator;

use crate::types::{File, Rank, Square};

/// A map of values indexed by [`Square`]. The map is represented as a fixed-size
/// array of length 64.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct SquareMap<T>([T; 64]);

impl<T> SquareMap<T> {
    #[inline]
    pub const fn new(map: [T; 64]) -> Self {
        SquareMap(map)
    }

    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Square, &T)> {
        self.0
            .iter()
            .enumerate()
            .map(|(i, val)| unsafe { (Square::from_unchecked(i as u8), val) })
    }

    #[inline]
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (Square, &mut T)> {
        self.0
            .iter_mut()
            .enumerate()
            .map(|(i, val)| unsafe { (Square::from_unchecked(i as u8), val) })
    }

    #[inline]
    pub fn from_fn(f: impl Fn(Square) -> T) -> Self {
        let mut map: [MaybeUninit<T>; 64] = unsafe { MaybeUninit::uninit().assume_init() };
        for sq in Square::iter() {
            map[sq as usize] = MaybeUninit::new(f(sq));
        }
        Self(map.map(|v| unsafe { v.assume_init() }))
    }
}

impl<T: Default + Copy> Default for SquareMap<T> {
    #[inline]
    fn default() -> Self {
        SquareMap([T::default(); 64])
    }
}

impl<T> Index<Square> for SquareMap<T> {
    type Output = T;

    #[inline]
    fn index(&self, index: Square) -> &Self::Output {
        &self.0[index as usize]
    }
}

impl<T> IndexMut<Square> for SquareMap<T> {
    #[inline]
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        &mut self.0[index as usize]
    }
}

impl<T> IntoIterator for SquareMap<T> {
    type Item = (Square, T);
    type IntoIter = IntoIter<T>;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        IntoIter::new(self.0)
    }
}

impl<T: Debug> Debug for SquareMap<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        writeln!(f)?;
        for rank in Rank::iter().rev() {
            for file in File::iter() {
                let sq = Square::from_rank_file(rank, file);
                write!(f, "{:?} ", self[sq])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub struct IntoIter<T> {
    inner_iter: iter::Enumerate<array::IntoIter<T, 64>>,
}

impl<T> IntoIter<T> {
    fn new(map: [T; 64]) -> Self {
        Self {
            inner_iter: map.into_iter().enumerate(),
        }
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = (Square, T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter
            .next()
            .map(|(i, val)| unsafe { (Square::from_unchecked(i as u8), val) })
    }
}
