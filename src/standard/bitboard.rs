use crate::framework::square::Square;
use crate::framework::SquareSet;
use crate::standard::iter::BitboardIter;
use std::ops::{Shr, BitOr, BitAnd, Not};
use crate::framework::direction::Direction;

#[macro_export]
macro_rules! bb {
    ( $( $sq:expr ),* ) => {{
        Bitboard(0) $(.add_sq($sq) )*
    }};
}


#[derive(Copy, Clone, PartialEq)]
pub struct Bitboard(pub u64);

impl Bitboard {
    pub const fn add_sq(self, sq: Square) -> Self {
        Bitboard(self.0 | (1 << sq as u64))
    }
}

impl SquareSet for Bitboard {
    const RANKS: [Self; 8] = [
        Bitboard(0x0000_0000_0000_00FF), Bitboard(0x0000_0000_0000_FF00),
        Bitboard(0x0000_0000_00FF_0000), Bitboard(0x0000_0000_FF00_0000),
        Bitboard(0x0000_00FF_0000_0000), Bitboard(0x0000_FF00_0000_0000),
        Bitboard(0x00FF_0000_0000_0000), Bitboard(0xFF00_0000_0000_0000)
    ];

    const FILES: [Self; 8] = [
        Bitboard(0x0101_0101_0101_0101), Bitboard(0x0202_0202_0202_0202),
        Bitboard(0x0404_0404_0404_0404), Bitboard(0x0808_0808_0808_0808),
        Bitboard(0x1010_1010_1010_1010), Bitboard(0x2020_2020_2020_2020),
        Bitboard(0x4040_4040_4040_4040), Bitboard(0x8080_8080_8080_8080)
    ];

    fn new() -> Self {
        Bitboard(0)
    }

    fn from_sq(sq: Square) -> Self {
        bb!(sq)
    }

    fn add(&mut self, sq: Square) {
        self.0 |= 1 << sq as u64;
    }

    fn is_empty(&self) -> bool {
        *self == bb!()
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

    // TODO: Optimize maybe?
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