use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

use itertools::iproduct;

use super::Color;

/// Represents a kind of piece in chess (pawns, knights, etc.).
#[derive(PartialEq, Debug, Copy, strum::Display, Clone)]
pub enum PieceKind {
    #[strum(serialize = "p")]
    Pawn = 4, // 0-3 are used for promotion pieces
    #[strum(serialize = "n")]
    Knight = 0,
    #[strum(serialize = "b")]
    Bishop = 1,
    #[strum(serialize = "r")]
    Rook = 2,
    #[strum(serialize = "q")]
    Queen = 3,
    #[strum(serialize = "k")]
    King = 5,
}

impl TryFrom<char> for PieceKind {
    type Error = PieceFromCharError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase() {
            'p' => Ok(PieceKind::Pawn),
            'n' => Ok(PieceKind::Knight),
            'b' => Ok(PieceKind::Bishop),
            'r' => Ok(PieceKind::Rook),
            'q' => Ok(PieceKind::Queen),
            'k' => Ok(PieceKind::King),
            _ => Err(PieceFromCharError(value)),
        }
    }
}

#[derive(thiserror::Error, Debug)]
#[error("invalid piece character '{0}'")]
pub struct PieceFromCharError(char);

/// Represents a piece in chess, i.e. a piece kind and a color.
#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Piece(pub PieceKind, pub Color);

impl Piece {
    #[inline]
    pub const fn kind(self) -> PieceKind {
        self.0
    }

    #[inline]
    pub const fn color(self) -> Color {
        self.1
    }

    /// Iterates over all possibles value of `Piece`.
    #[inline]
    pub fn iter() -> impl Iterator<Item = Self> {
        iproduct!(
            [Color::White, Color::Black],
            [
                PieceKind::Pawn,
                PieceKind::Knight,
                PieceKind::Bishop,
                PieceKind::Rook,
                PieceKind::Queen,
                PieceKind::King
            ]
        )
        .map(|(color, kind)| Self(kind, color))
    }
}

impl TryFrom<char> for Piece {
    type Error = PieceFromCharError;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let kind = PieceKind::try_from(value)?;
        let color = if value.is_ascii_uppercase() {
            Color::White
        } else {
            Color::Black
        };
        Ok(Piece(kind, color))
    }
}

impl Display for Piece {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.color() {
            Color::White => write!(f, "{}", self.kind().to_string().to_ascii_uppercase()),
            Color::Black => write!(f, "{}", self.kind()),
        }
    }
}
