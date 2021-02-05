use crate::framework::square::Square;
use std::iter::FusedIterator;
use crate::standard::bitboard::Bitboard;
use bitintr::{Blsr, Tzcnt};

pub struct BitboardIter(Bitboard);

impl BitboardIter {
    pub fn new(bb: Bitboard) -> Self {
        BitboardIter(bb)
    }
}

impl Iterator for BitboardIter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.0.is_empty() {
            None
        } else {
            let sq = unsafe { Square::from_unchecked((self.0).0.tzcnt() as u8) };
            self.0 = Bitboard((self.0).0.blsr());
            Some(sq)
        }
    }
}

impl FusedIterator for BitboardIter { }