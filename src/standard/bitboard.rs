use crate::framework::square::Square;
use crate::standard::bitboard::iter::BitboardIter;
use std::ops::{Shr, BitOr, BitAnd, Not, Sub};
use crate::framework::direction::Direction;
use bitintr::Andn;

mod iter;


#[macro_export]
macro_rules! bb {
    ( $( $sq:expr ),* ) => {{
        Bitboard::new() $(.add_sq($sq) )*
    }};
}


#[derive(Copy, Clone, PartialEq, Debug)]
pub struct Bitboard(u64);

impl Bitboard {
    /// Ranks from 1 to 8
    pub const RANKS: [Self; 8] = [
        Bitboard(0x0000_0000_0000_00FF), Bitboard(0x0000_0000_0000_FF00),
        Bitboard(0x0000_0000_00FF_0000), Bitboard(0x0000_0000_FF00_0000),
        Bitboard(0x0000_00FF_0000_0000), Bitboard(0x0000_FF00_0000_0000),
        Bitboard(0x00FF_0000_0000_0000), Bitboard(0xFF00_0000_0000_0000)
    ];

    /// Files from a to h
    pub const FILES: [Self; 8] = [
        Bitboard(0x0101_0101_0101_0101), Bitboard(0x0202_0202_0202_0202),
        Bitboard(0x0404_0404_0404_0404), Bitboard(0x0808_0808_0808_0808),
        Bitboard(0x1010_1010_1010_1010), Bitboard(0x2020_2020_2020_2020),
        Bitboard(0x4040_4040_4040_4040), Bitboard(0x8080_8080_8080_8080)
    ];

    /// Creates an empty `Bitboard`
    pub const fn new() -> Self {
        Bitboard(0)
    }

    /// Returns a new `Bitboard` with `Square` `sq` added
    pub const fn add_sq(self, sq: Square) -> Self {
        Bitboard(self.0 | (1 << sq as u64))
    }

    /// Returns whether the `Bitboard` is empty or not
    pub fn is_empty(self) -> bool {
        self == bb!()
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = BitboardIter;

    fn into_iter(self) -> Self::IntoIter {
        BitboardIter::new(self)
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl Shr<Direction> for Bitboard {
    type Output = Bitboard;

    // TODO: Optimize maybe? Use andn?
    fn shr(self, rhs: Direction) -> Self::Output {
        match rhs {
            Direction::North =>
                Bitboard(self.0 << 8),
            Direction::NorthEast | Direction::East =>
                Bitboard((self.0 & !Bitboard::FILES[7].0) << rhs as u64),
            Direction::SouthEast =>
                Bitboard((self.0 & !Bitboard::FILES[7].0) >> 7),
            Direction::South =>
                Bitboard(self.0 >> 8),
            Direction::SouthWest =>
                Bitboard((self.0 & !Bitboard::FILES[0].0) >> 9),
            Direction::West =>
                Bitboard((self.0 & !Bitboard::FILES[7].0) >> 1),
            Direction::NorthWest =>
                Bitboard((self.0 & !Bitboard::FILES[0].0) << 7),
        }
    }
}

impl Sub for Bitboard {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Self(rhs.0.andn(self.0))
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_test() {
        assert_eq!(bb!(Square::A1, Square::A2) - bb!(Square::A1, Square::H8), bb!(Square::A2));
    }
}