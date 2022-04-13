use std::ops::{Deref, DerefMut, Index};
use std::slice::{Iter, SliceIndex};

use arrayvec::{ArrayVec, IntoIter};

use crate::types::Move;

#[derive(Debug)]
pub struct MoveList(ArrayVec<Move, 256>);

impl MoveList {
    pub fn new() -> Self {
        MoveList(ArrayVec::new())
    }

    pub fn push(&mut self, m: Move) {
        /*unsafe {
            self.0.push_unchecked(m);
        }*/
        self.0.push(m);
    }

    pub fn contains(&self, m: Move) -> bool {
        self.0.contains(&m)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> Iter<Move> {
        self.0.iter()
    }

    pub fn get(&self, index: usize) -> Option<&Move> {
        self.0.get(index)
    }

    pub fn into_vec(self) -> Vec<Move> {
        self.0.as_slice().to_vec()
    }
}

impl IntoIterator for MoveList {
    type Item = Move;
    type IntoIter = IntoIter<Move, 256>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a MoveList {
    type Item = &'a Move;
    type IntoIter = Iter<'a, Move>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<Move> for MoveList {
    fn from_iter<I: IntoIterator<Item = Move>>(iter: I) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<I> Index<I> for MoveList
where
    I: SliceIndex<[Move]>,
{
    type Output = <I as SliceIndex<[Move]>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.0[index]
    }
}

impl AsRef<[Move]> for MoveList {
    fn as_ref(&self) -> &[Move] {
        self.0.as_slice()
    }
}

impl AsMut<[Move]> for MoveList {
    fn as_mut(&mut self) -> &mut [Move] {
        self.0.as_mut()
    }
}

impl Deref for MoveList {
    type Target = [Move];

    fn deref(&self) -> &Self::Target {
        self.as_ref()
    }
}

impl DerefMut for MoveList {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut()
    }
}

impl Default for MoveList {
    fn default() -> Self {
        Self::new()
    }
}
