use std::fmt::{self, Display, Formatter};
use std::mem;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

/// Represents a square on the chessboard.
#[rustfmt::skip]
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Square {
    A1, B1, C1, D1, E1, F1, G1, H1,
    A2, B2, C2, D2, E2, F2, G2, H2,
    A3, B3, C3, D3, E3, F3, G3, H3,
    A4, B4, C4, D4, E4, F4, G4, H4,
    A5, B5, C5, D5, E5, F5, G5, H5,
    A6, B6, C6, D6, E6, F6, G6, H6,
    A7, B7, C7, D7, E7, F7, G7, H7,
    A8, B8, C8, D8, E8, F8, G8, H8,
}

impl Square {
    /// Returns the rank of the square.
    #[inline]
    pub fn rank(self) -> Rank {
        // Safety: self is in [0; 63], so this is safe.
        unsafe { mem::transmute((self as u8) / 8) }
    }

    /// Returns the file of the square.
    #[inline]
    pub fn file(self) -> File {
        // Safety: We mod by 8, so this is safe.
        unsafe { mem::transmute((self as u8) % 8) }
    }

    /// Returns the square with the given rank and file.
    #[inline]
    pub fn from_rank_file(rank: Rank, file: File) -> Self {
        // Safety: File and Rank are both in [0; 7], so the result is in [0; 63].
        unsafe { Self::from_unchecked(rank as u8 + 8 * file as u8) }
    }

    /// Returns the square with the given index.
    ///
    /// # Safety
    /// `index` should be in the interval `[0; 63]`.
    #[inline]
    pub const unsafe fn from_unchecked(index: u8) -> Self {
        debug_assert!(index < 64);
        mem::transmute(index)
    }
}

impl Add<BoardVector> for Square {
    type Output = Self;

    /// Adds a [`BoardVector`] to a [`Square`], yielding a [`Square`] and wrapping around
    /// the A and H files according to the little endian square numbering if necessary.
    ///
    /// # Panics
    /// Panics in debug mode if the result is not inside the board.
    /// In release mode, an undefined square is returned.
    ///
    /// # Examples
    /// ```
    /// use kingly_lib::types::{Square, BoardVector};
    ///
    /// assert_eq!(Square::A1 + BoardVector::NORTH, Square::A2);
    /// assert_eq!(Square::E4 + BoardVector::SOUTH_EAST * 2, Square::G2);
    /// assert_eq!(Square::G4 + BoardVector::NORTH_EAST * 2, Square::A7);
    /// ```
    #[inline]
    fn add(self, rhs: BoardVector) -> Self::Output {
        let res = (self as i8 + rhs.0) as u8;
        debug_assert!(res < 64);
        // Safety: We modulo the result by 64, so it's safe to convert to a square.
        unsafe { Self::from_unchecked(res % 64) }
    }
}

impl FromStr for Square {
    type Err = ParseSquareError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.len() != 2 {
            return Err(ParseSquareError::InvalidLength);
        }
        let rank = s[1..2].parse()?;
        let file = s[0..1].parse()?;
        Ok(Square::from_rank_file(rank, file))
    }
}

impl Display for Square {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}{}", self.file(), self.rank())
    }
}

/// An error that can occur when parsing a square.
#[derive(thiserror::Error, Debug)]
pub enum ParseSquareError {
    #[error("Invalid square: Invalid length")]
    InvalidLength,
    #[error("Invalid square: {0}")]
    ParseError(#[from] strum::ParseError),
}

/// Represents a rank on the chessboard.
#[derive(Copy, Clone, PartialEq, Eq, Debug, strum::EnumIter, strum::Display, strum::EnumString)]
#[repr(u8)]
pub enum Rank {
    #[strum(serialize = "1")]
    First,
    #[strum(serialize = "2")]
    Second,
    #[strum(serialize = "3")]
    Third,
    #[strum(serialize = "4")]
    Fourth,
    #[strum(serialize = "5")]
    Fifth,
    #[strum(serialize = "6")]
    Sixth,
    #[strum(serialize = "7")]
    Seventh,
    #[strum(serialize = "8")]
    Eighth,
}

/// Represents a file on the chessboard.
#[derive(Copy, Clone, PartialEq, Eq, Debug, strum::EnumIter, strum::Display, strum::EnumString)]
#[repr(u8)]
pub enum File {
    #[strum(serialize = "a")]
    A,
    #[strum(serialize = "b")]
    B,
    #[strum(serialize = "c")]
    C,
    #[strum(serialize = "d")]
    D,
    #[strum(serialize = "e")]
    E,
    #[strum(serialize = "f")]
    F,
    #[strum(serialize = "g")]
    G,
    #[strum(serialize = "h")]
    H,
}

/// Represents a vector on the chessboard.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BoardVector(pub i8);

impl BoardVector {
    pub const NORTH: Self = Self(8);
    pub const SOUTH: Self = Self(-8);
    pub const EAST: Self = Self(1);
    pub const WEST: Self = Self(-1);
    pub const NORTH_EAST: Self = Self(9);
    pub const NORTH_WEST: Self = Self(7);
    pub const SOUTH_EAST: Self = Self(-7);
    pub const SOUTH_WEST: Self = Self(-9);
}

impl Add for BoardVector {
    type Output = Self;

    #[inline]
    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl Sub for BoardVector {
    type Output = Self;

    #[inline]
    fn sub(self, rhs: Self) -> Self::Output {
        Self(self.0 - rhs.0)
    }
}

impl Neg for BoardVector {
    type Output = Self;

    #[inline]
    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Mul<i8> for BoardVector {
    type Output = Self;

    #[inline]
    fn mul(self, rhs: i8) -> Self::Output {
        Self(self.0 * rhs)
    }
}
