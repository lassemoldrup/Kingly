use crate::framework::square::Square;
use crate::framework::SquareSet;
use crate::standard::iter::BitboardIter;

#[derive(Copy, Clone)]
pub struct Bitboard(u64);

impl SquareSet for Bitboard {
    fn new() -> Self {
        Bitboard(0)
    }

    fn add(&mut self, sq: Square) {
        self.0 |= 1 << sq as u64;
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;

    fn into_iter(self) -> Self::IntoIter {
        unimplemented!()
    }
}