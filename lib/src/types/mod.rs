use std::ops::Not;

pub mod bitboard;
mod moves;
mod piece;
mod square;

pub use bitboard::Bitboard;
pub use moves::*;
pub use piece::*;
pub use square::*;

/// Represents a color (white or black) in chess.
#[derive(Clone, Copy, PartialEq, Debug, strum::Display)]
#[repr(u8)]
pub enum Color {
    White = 0,
    Black = 1,
}

impl Not for Color {
    type Output = Self;

    /// Returns the opposite color.
    #[inline]
    fn not(self) -> Self::Output {
        match self {
            Color::White => Color::Black,
            Color::Black => Color::White,
        }
    }
}

/// Represents the king or queen side of the board.
#[derive(Copy, Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum Side {
    KingSide = 0b01,
    QueenSide = 0b10,
}

/// Represents the castling rights of a position.
// Each combination of color and side is represented by a bit:
// 0b0001: White kingside
// 0b0010: White queenside
// 0b0100: Black kingside
// 0b1000: Black queenside
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct CastlingRights(u8);

impl CastlingRights {
    /// Creates a new `CastlingRights` instance from the given castling rights.
    #[inline]
    pub fn new(w_king: bool, w_queen: bool, b_king: bool, b_queen: bool) -> Self {
        CastlingRights(
            w_king as u8 | (w_queen as u8) << 1 | (b_king as u8) << 2 | (b_queen as u8) << 3,
        )
    }

    /// Returns the castling rights for a given color and side.
    #[inline]
    pub fn get(&self, color: Color, side: Side) -> bool {
        match color {
            Color::White => self.0 & side as u8 != 0,
            Color::Black => self.0 & ((side as u8) << 2) != 0,
        }
    }

    /// Sets king and queen castling rights for a given color based on a 2-bit number,
    /// e.g. 0b01 means giving kingside castling, 0b11 means giving both sided castling
    #[inline]
    pub fn set(&mut self, color: Color, rights: u8) {
        match color {
            Color::White => self.0 |= rights,
            Color::Black => self.0 |= rights << 2,
        };
    }

    /// Similar to set, except it removes castling rights,
    /// e.g. 0b10 removes queenside castling
    #[inline]
    pub fn remove(&mut self, color: Color, rights: u8) {
        match color {
            Color::White => self.0 &= !rights,
            Color::Black => self.0 &= !(rights << 2),
        };
    }
}

impl From<CastlingRights> for u8 {
    #[inline]
    fn from(castling: CastlingRights) -> Self {
        castling.0
    }
}

impl From<CastlingRights> for usize {
    #[inline]
    fn from(castling: CastlingRights) -> Self {
        castling.0 as usize
    }
}
