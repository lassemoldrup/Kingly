use std::fmt::{self, Display, Formatter};
use std::mem;
use std::ops::{Add, Mul, Neg, Sub};
use std::str::FromStr;

use super::{Color, Side};

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
    pub const fn rank(self) -> Rank {
        // Safety: self is in [0; 63], so this is safe.
        unsafe { mem::transmute((self as u8) / 8) }
    }

    /// Returns the file of the square.
    #[inline]
    pub const fn file(self) -> File {
        // Safety: We mod by 8, so this is safe.
        unsafe { mem::transmute((self as u8) % 8) }
    }

    /// Returns the square with the given rank and file.
    #[inline]
    pub const fn from_rank_file(rank: Rank, file: File) -> Self {
        // Safety: File and Rank are both in [0; 7], so the result is in [0; 63].
        unsafe { Self::from_unchecked(rank as u8 * 8 + file as u8) }
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

    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        (0..64).map(|i| unsafe { Self::from_unchecked(i) })
    }

    #[inline]
    pub fn add_checked(self, vector: BoardVector) -> Option<Self> {
        let res = (self as i8 + vector.0) as u8;
        let dest = Self::try_from(res).ok()?;
        let dist = self.dist(dest);
        (dist <= 4).then_some(dest)
    }

    #[inline]
    pub const fn dist(self, other: Self) -> u8 {
        let dr = (self.rank() as i8 - other.rank() as i8).abs();
        let df = (self.file() as i8 - other.file() as i8).abs();
        (dr + df) as u8
    }

    #[inline]
    pub const fn king_starting(color: Color) -> Self {
        match color {
            Color::White => Square::E1,
            Color::Black => Square::E8,
        }
    }

    #[inline]
    pub const fn king_castling_dest(color: Color, side: Side) -> Self {
        match color {
            Color::White => match side {
                Side::KingSide => Square::G1,
                Side::QueenSide => Square::C1,
            },
            Color::Black => match side {
                Side::KingSide => Square::G8,
                Side::QueenSide => Square::C8,
            },
        }
    }

    #[inline]
    pub const fn rook_starting(color: Color, side: Side) -> Square {
        match color {
            Color::White => match side {
                Side::KingSide => Square::H1,
                Side::QueenSide => Square::A1,
            },
            Color::Black => match side {
                Side::KingSide => Square::H8,
                Side::QueenSide => Square::A8,
            },
        }
    }

    #[inline]
    pub const fn rook_castling_dest(color: Color, side: Side) -> Square {
        match color {
            Color::White => match side {
                Side::KingSide => Square::F1,
                Side::QueenSide => Square::D1,
            },
            Color::Black => match side {
                Side::KingSide => Square::F8,
                Side::QueenSide => Square::D8,
            },
        }
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

impl TryFrom<u8> for Square {
    type Error = SquareFromU8Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 64 {
            // Safety: value is in [0; 63], so this is safe.
            Ok(unsafe { Self::from_unchecked(value) })
        } else {
            Err(SquareFromU8Error(value))
        }
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

#[derive(thiserror::Error, Debug)]
#[error("invalid square value: {0}")]
pub struct SquareFromU8Error(u8);

/// An error that can occur when parsing a square.
#[derive(thiserror::Error, Debug)]
pub enum ParseSquareError {
    #[error("invalid square length")]
    InvalidLength,
    #[error("invalid square: {0}")]
    ParseError(#[from] strum::ParseError),
}

/// Represents a rank on the chessboard.
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Debug,
    PartialOrd,
    Ord,
    strum::EnumIter,
    strum::Display,
    strum::EnumString,
    strum::FromRepr,
)]
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

impl Rank {
    pub fn iter_before(self) -> impl Iterator<Item = Self> {
        (0..self as u8).map(|i| unsafe { mem::transmute(i) }).rev()
    }

    pub fn iter_after(self) -> impl Iterator<Item = Self> {
        (self as u8 + 1..8).map(|i| unsafe { mem::transmute(i) })
    }
}

/// Represents a file on the chessboard.
#[derive(
    Copy,
    Clone,
    PartialEq,
    Eq,
    Debug,
    PartialOrd,
    Ord,
    strum::EnumIter,
    strum::Display,
    strum::EnumString,
    strum::FromRepr,
)]
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

impl File {
    pub fn iter_before(self) -> impl Iterator<Item = Self> {
        (0..self as u8).map(|i| unsafe { mem::transmute(i) }).rev()
    }

    pub fn iter_after(self) -> impl Iterator<Item = Self> {
        (self as u8 + 1..8).map(|i| unsafe { mem::transmute(i) })
    }
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
