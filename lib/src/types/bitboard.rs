use std::fmt::Debug;
use std::iter::FusedIterator;
use std::ops::{BitAnd, BitAndAssign, BitOr, BitOrAssign, Not, Sub, SubAssign};

use strum::IntoEnumIterator;

use super::{File, Rank, Square};

#[macro_export]
macro_rules! bb {
    ( $( $sq:expr ),* $(,)? ) => { {
        #[allow(unused_imports)]
        use Square::*;
        $crate::types::Bitboard::new() $(.with_sq($sq) )*
    }};
}

/// Ranks from 1 to 8.
pub const RANKS: [Bitboard; 8] = [
    Bitboard(0x0000_0000_0000_00FF),
    Bitboard(0x0000_0000_0000_FF00),
    Bitboard(0x0000_0000_00FF_0000),
    Bitboard(0x0000_0000_FF00_0000),
    Bitboard(0x0000_00FF_0000_0000),
    Bitboard(0x0000_FF00_0000_0000),
    Bitboard(0x00FF_0000_0000_0000),
    Bitboard(0xFF00_0000_0000_0000),
];

/// Files from A to H.
pub const FILES: [Bitboard; 8] = [
    Bitboard(0x0101_0101_0101_0101),
    Bitboard(0x0202_0202_0202_0202),
    Bitboard(0x0404_0404_0404_0404),
    Bitboard(0x0808_0808_0808_0808),
    Bitboard(0x1010_1010_1010_1010),
    Bitboard(0x2020_2020_2020_2020),
    Bitboard(0x4040_4040_4040_4040),
    Bitboard(0x8080_8080_8080_8080),
];

/// Diagonals from h1 to a8 (index is 7 + rank - file).
pub const DIAGS: [Bitboard; 15] = [
    Bitboard(0x0000_0000_0000_0080),
    Bitboard(0x0000_0000_0000_8040),
    Bitboard(0x0000_0000_0080_4020),
    Bitboard(0x0000_0000_8040_2010),
    Bitboard(0x0000_0080_4020_1008),
    Bitboard(0x0000_8040_2010_0804),
    Bitboard(0x0080_4020_1008_0402),
    Bitboard(0x8040_2010_0804_0201),
    Bitboard(0x4020_1008_0402_0100),
    Bitboard(0x2010_0804_0201_0000),
    Bitboard(0x1008_0402_0100_0000),
    Bitboard(0x0804_0201_0000_0000),
    Bitboard(0x0402_0100_0000_0000),
    Bitboard(0x0201_0000_0000_0000),
    Bitboard(0x0100_0000_0000_0000),
];

/// Anti-diagonals from a1 to h8 (index is rank + file).
pub const ANTI_DIAGS: [Bitboard; 15] = [
    Bitboard(0x0000_0000_0000_0001),
    Bitboard(0x0000_0000_0000_0102),
    Bitboard(0x0000_0000_0001_0204),
    Bitboard(0x0000_0000_0102_0408),
    Bitboard(0x0000_0001_0204_0810),
    Bitboard(0x0000_0102_0408_1020),
    Bitboard(0x0001_0204_0810_2040),
    Bitboard(0x0102_0408_1020_4080),
    Bitboard(0x0204_0810_2040_8000),
    Bitboard(0x0408_1020_4080_0000),
    Bitboard(0x0810_2040_8000_0000),
    Bitboard(0x1020_4080_0000_0000),
    Bitboard(0x2040_8000_0000_0000),
    Bitboard(0x4080_0000_0000_0000),
    Bitboard(0x8000_0000_0000_0000),
];

#[derive(Copy, Clone, PartialEq, Eq, Default)]
pub struct Bitboard(u64);

impl Bitboard {
    /// Creates an empty `Bitboard`.
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Returns a new `Bitboard` with `sq` added.
    #[inline]
    pub const fn with_sq(self, sq: Square) -> Self {
        Bitboard(self.0 | (1 << sq as u64))
    }

    /// Adds `sq` to the `Bitboard`.
    #[inline]
    pub fn add_sq(&mut self, sq: Square) {
        self.0 |= 1 << sq as u64;
    }

    /// Returns whether the `Bitboard` is empty or not.
    #[inline]
    pub fn is_empty(self) -> bool {
        self == Self::new()
    }

    /// Returns the number of set squares.
    #[inline]
    pub const fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    /// Returns whether the `Bitboard` contains `sq`.
    #[inline]
    pub fn contains(self, sq: Square) -> bool {
        (self.0 >> sq as u64) & 1 == 1
    }
}

impl IntoIterator for Bitboard {
    type Item = Square;
    type IntoIter = Iter;

    #[inline]
    fn into_iter(self) -> Self::IntoIter {
        Iter { bb: self }
    }
}

impl FromIterator<Square> for Bitboard {
    #[inline]
    fn from_iter<T: IntoIterator<Item = Square>>(iter: T) -> Self {
        let mut bb = Bitboard::new();
        for sq in iter {
            bb.add_sq(sq);
        }
        bb
    }
}

impl BitOr for Bitboard {
    type Output = Self;

    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 | rhs.0)
    }
}

impl BitOrAssign for Bitboard {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

impl BitAnd for Bitboard {
    type Output = Self;

    #[inline]
    fn bitand(self, rhs: Self) -> Self::Output {
        Bitboard(self.0 & rhs.0)
    }
}

impl BitAndAssign for Bitboard {
    #[inline]
    fn bitand_assign(&mut self, rhs: Self) {
        self.0 &= rhs.0;
    }
}

impl Not for Bitboard {
    type Output = Self;

    #[inline]
    fn not(self) -> Self::Output {
        Bitboard(!self.0)
    }
}

impl Sub for Bitboard {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 & !rhs.0)
    }
}

impl SubAssign for Bitboard {
    #[inline]
    fn sub_assign(&mut self, rhs: Self) {
        *self = *self - rhs;
    }
}

impl From<Bitboard> for u64 {
    #[inline]
    fn from(bb: Bitboard) -> Self {
        bb.0
    }
}

impl From<u64> for Bitboard {
    #[inline]
    fn from(val: u64) -> Self {
        Self(val)
    }
}

impl From<Square> for Bitboard {
    #[inline]
    fn from(sq: Square) -> Self {
        Self::new().with_sq(sq)
    }
}

impl From<Rank> for Bitboard {
    #[inline]
    fn from(rank: Rank) -> Self {
        RANKS[rank as usize]
    }
}

impl From<File> for Bitboard {
    #[inline]
    fn from(file: File) -> Self {
        FILES[file as usize]
    }
}

impl Debug for Bitboard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in Rank::iter().rev() {
            for file in File::iter() {
                let sq = Square::from_rank_file(rank, file);
                if self.contains(sq) {
                    write!(f, "# ")?;
                } else {
                    write!(f, ". ")?;
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

pub struct Iter {
    bb: Bitboard,
}

impl Iterator for Iter {
    type Item = Square;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        let count = self.bb.0.trailing_zeros();
        if count >= 64 {
            return None;
        }
        // Safety: count is in [0; 63] so it's safe to convert to a square.
        let sq = unsafe { Square::from_unchecked(count as u8) };
        self.bb -= sq.into();
        Some(sq)
    }
}

impl FusedIterator for Iter {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sub_test() {
        assert_eq!(bb!(A1, A2) - bb!(A1, H8), bb!(A2));
    }
}
