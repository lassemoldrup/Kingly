use super::{Square, bin_vec::*};
use std::convert::From;
use bitintr::{Tzcnt, Popcnt, Blsr, Andn};
use std::ops::{BitOr, BitOrAssign, BitAnd, BitAndAssign, Not, Shl, Shr, Mul, Add, BitXor};
use std::fmt;
use std::iter::FusedIterator;

pub const FILE_A_BB: Bitboard = Bitboard(0x0101_0101_0101_0101);
pub const FILE_H_BB: Bitboard = Bitboard(0x8080_8080_8080_8080);
pub const RANK_1_BB: Bitboard = Bitboard(0x0000_0000_0000_00FF);
pub const RANK_2_BB: Bitboard = Bitboard(0x0000_0000_0000_FF00);
pub const RANK_4_BB: Bitboard = Bitboard(0x0000_0000_FF00_0000);
pub const RANK_5_BB: Bitboard = Bitboard(0x0000_00FF_0000_0000);
pub const RANK_7_BB: Bitboard = Bitboard(0x00FF_0000_0000_0000);
pub const RANK_8_BB: Bitboard = Bitboard(0xFF00_0000_0000_0000);

pub fn shift<const VEC: BinVec>(bb: Bitboard) -> Bitboard {
    match VEC {
        NORTH => bb << VEC.0 as u64,
        EAST => (bb << VEC.0 as u64) & !FILE_A_BB,
        SOUTH => (bb >> NORTH.0 as u64),
        WEST => (bb >> EAST.0 as u64) & !FILE_H_BB,
        NORTH_EAST => (bb << VEC.0 as u64) & !FILE_A_BB,
        SOUTH_EAST => (bb >> NORTH_WEST.0 as u64) & !FILE_A_BB,
        SOUTH_WEST => (bb >> NORTH_EAST.0 as u64) & !FILE_H_BB,
        NORTH_WEST => (bb << VEC.0 as u64) & !FILE_H_BB,
        _ => panic!("Shift not implemented for {:?}", VEC),
    }
}

#[macro_export]
macro_rules! bb {
    ( $( $sq:expr ),* ) => {{
        Bitboard::EMPTY $(.add_sq($sq) )*
    }};
}

/// A chess bitboard
#[derive(Eq, PartialEq, Copy, Clone)]
pub struct Bitboard(u64);

impl Bitboard {
    pub const EMPTY: Bitboard = Bitboard(0u64);

    pub const fn from_sq(sq: Square) -> Self {
        Bitboard(1u64 << sq as u64)
    }
    pub fn set(&mut self, sq: Square) {
        self.0 |= 1u64 << sq as u64;
    }
    // TODO: Check if the compiler optimises this
    pub fn shift(self, amount: i32) -> Self {
        if amount < 0 {
            Bitboard(self.0 >> -amount as u64)
        } else {
            Bitboard(self.0 << amount as u64)
        }
    }
    pub fn toggle(&mut self, sq: Square) {
        self.0 ^= 1u64 << sq as u64;
    }
    pub fn is_set(self, sq: Square) -> bool {
       (self.0 >> sq as u64) & 1 == 1
    }
    pub fn first_sq(self) -> Option<Square> {
        if self == Self::EMPTY {
            None
        } else {
            unsafe { Some(self.first_sq_unchecked()) }
        }
    }
    /// Gets the square corresponding to the first set bit
    /// # Safety
    /// Must have a square
    pub unsafe fn first_sq_unchecked(self) -> Square {
        debug_assert_ne!(self, Self::EMPTY);
        Square::from_unchecked(self.0.tzcnt() as u8)
    }
    pub fn pop_count(self) -> u64 {
        self.0.popcnt()
    }
    pub fn set_diff(self, other: Bitboard) -> Bitboard {
        Bitboard(other.0.andn(self.0))
    }
    pub const fn add_sq(self, sq: Square) -> Self {
        Bitboard(self.0 | 1 << sq as u64)
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

impl const BitOr for Bitboard {
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

impl BitXor for Bitboard {
    type Output = Self;

    fn bitxor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 ^ rhs.0)
    }
}

impl Not for Bitboard {
    type Output = Self;

    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl Shl<u64> for Bitboard {
    type Output = Self;

    fn shl(self, rhs: u64) -> Self::Output {
        Bitboard(self.0 << rhs)
    }
}

impl Shr<u64> for Bitboard {
    type Output = Self;

    fn shr(self, rhs: u64) -> Self::Output {
        Bitboard(self.0 >> rhs)
    }
}

impl Mul<u64> for Bitboard {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Bitboard(self.0.wrapping_mul(rhs))
    }
}

impl Add<u64> for Bitboard {
    type Output = Self;

    fn add(self, rhs: u64) -> Self::Output {
        Bitboard(self.0 + rhs)
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Iter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Into<usize> for Bitboard {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl const From<u64> for Bitboard {
    fn from(val: u64) -> Self {
        Bitboard(val)
    }
}

impl From<Square> for Bitboard {
    fn from(sq: Square) -> Self {
        Bitboard(1u64 << sq as u64)
    }
}

impl fmt::Debug for Bitboard {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        writeln!(f)?;
        for row in Square::A1.range_to(Square::H8).collect::<Vec<_>>().rchunks_exact(8) {
            for sq in row {
                if self.is_set(*sq) {
                    write!(f, "x ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(Copy, Clone)]
pub struct Iter {
    board: Bitboard,
}

impl Iterator for Iter {
    type Item = Square;

    fn next(&mut self) -> Option<Self::Item> {
        if self.board == Bitboard::EMPTY {
            None
        } else {
            let sq = unsafe { self.board.first_sq_unchecked() };
            self.board.0 = self.board.0.blsr();
            Some(sq)
        }
    }
}

impl FusedIterator for Iter { }