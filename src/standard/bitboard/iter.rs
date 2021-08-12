use std::iter::FusedIterator;

use bitintr::Blsr;

use crate::framework::square::Square;
use crate::standard::bitboard::Bitboard;

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
            let sq = unsafe {
                self.0.first_sq_unchecked()
            };
            self.0 = Bitboard((self.0).0.blsr());
            Some(sq)
        }
    }
}

impl FusedIterator for BitboardIter { }