use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};
use std::iter::{Enumerate, FusedIterator, Map};
use std::ops::{Index, IndexMut};
use std::slice;

use crate::types::Square;

#[derive(Copy, Clone)]
pub struct SquareMap<T>([T; 64]);

impl<T> SquareMap<T> {
    pub fn new(map: [T; 64]) -> Self {
        SquareMap(map)
    }

    pub fn iter(&self) -> Iter<T> {
        Iter::new(self)
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut::new(self)
    }
}

impl<T: Default + Copy> Default for SquareMap<T> {
    fn default() -> Self {
        SquareMap([T::default(); 64])
    }
}

impl<T> Index<Square> for SquareMap<T> {
    type Output = T;

    fn index(&self, index: Square) -> &Self::Output {
        unsafe { self.0.get_unchecked(index as usize) }
    }
}

impl<T> IndexMut<Square> for SquareMap<T> {
    fn index_mut(&mut self, index: Square) -> &mut Self::Output {
        unsafe { self.0.get_unchecked_mut(index as usize) }
    }
}

impl<T: PartialEq> PartialEq for SquareMap<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0.iter().zip(other.0.iter()).all(|(x, y)| *x == *y)
    }
}

impl<T: Debug> Debug for SquareMap<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in (0..8).rev() {
            for file in 0..8 {
                let sq = Square::try_from(8 * rank + file).unwrap();
                write!(f, "{:?} ", self[sq])?;
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub struct Iter<'a, T> {
    inner_iter: Map<Enumerate<slice::Iter<'a, T>>, fn((usize, &'a T)) -> (Square, &'a T)>,
}

impl<'a, T> Iter<'a, T> {
    fn new(square_map: &'a SquareMap<T>) -> Self {
        Self {
            inner_iter: square_map
                .0
                .iter()
                .enumerate()
                .map(|(idx, item)| unsafe { (Square::from_unchecked(idx as u8), item) }),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (Square, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next()
    }
}

impl<'a, T> FusedIterator for Iter<'a, T> {}

pub struct IterMut<'a, T> {
    inner_iter:
        Map<Enumerate<slice::IterMut<'a, T>>, fn((usize, &'a mut T)) -> (Square, &'a mut T)>,
}

impl<'a, T> IterMut<'a, T> {
    fn new(square_map: &'a mut SquareMap<T>) -> Self {
        Self {
            inner_iter: square_map
                .0
                .iter_mut()
                .enumerate()
                .map(|(idx, item)| unsafe { (Square::from_unchecked(idx as u8), item) }),
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (Square, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner_iter.next()
    }
}

impl<'a, T> FusedIterator for IterMut<'a, T> {}
