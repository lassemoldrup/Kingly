use super::Square;
use bitintr::{Tzcnt, Popcnt, Blsr};
use std::ops::{BitOr, BitOrAssign, BitAnd, BitAndAssign};

/// A chess bitboard
#[derive(Eq, PartialEq, Debug, Copy, Clone)]
pub struct Bitboard(u64);

impl Bitboard {
    const EMPTY: Bitboard = Bitboard(0u64);

    pub fn new(val: u64) -> Self {
        Bitboard(val)
    }
    pub fn set(&mut self, sq: Square) {
        self.0 |= 1u64 << sq as u64;
    }
    pub fn toggle(&mut self, sq: Square) {
        self.0 ^= 1u64 << sq as u64;
    }
    pub fn first_sq(self) -> Option<Square> {
        if self == Self::EMPTY {
            None
        } else {
            unsafe {
                Some(self.first_sq_unchecked())
            }
        }
    }
    pub unsafe fn first_sq_unchecked(self) -> Square {
        Square::from_unchecked(self.0.tzcnt() as u8)
    }
    pub fn pop_count(self) -> u64 {
        self.0.popcnt()
    }
    pub fn iter(self) -> Iter {
        Iter { board: self }
    }
}

impl Default for Bitboard {
    fn default() -> Self {
        Self::EMPTY
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Iter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct Iter {
    board: Bitboard,
}

impl Iterator for Iter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.board == Bitboard::EMPTY {
            None
        } else {
            unsafe{
                let sq = self.board.first_sq_unchecked();
                self.board.0 = self.board.0.blsr();
                Some(sq)
            }
        }
    }
}