use crate::framework::square::Square;
use crate::framework::SquareSet;
use crate::standard::iter::BitboardIter;

#[derive(Copy, Clone)]
pub struct Bitboard(u64);

impl SquareSet for Bitboard { }

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;

    fn into_iter(self) -> Self::IntoIter {
        unimplemented!()
    }
}