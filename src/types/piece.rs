use std::convert::TryFrom;
use std::fmt::{Display, Formatter};

use itertools::iproduct;

use super::Color;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum PieceKind {
    Pawn = 4,
    Knight = 0,
    Bishop = 1,
    Rook = 2,
    Queen = 3,
    King = 5,
}

impl TryFrom<char> for PieceKind {
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        match value.to_ascii_lowercase() {
            'p' => Ok(PieceKind::Pawn),
            'n' => Ok(PieceKind::Knight),
            'b' => Ok(PieceKind::Bishop),
            'r' => Ok(PieceKind::Rook),
            'q' => Ok(PieceKind::Queen),
            'k' => Ok(PieceKind::King),
            _ => Err("Invalid piece character"),
        }
    }
}

impl Display for PieceKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            PieceKind::Pawn => write!(f, "p"),
            PieceKind::Knight => write!(f, "n"),
            PieceKind::Bishop => write!(f, "b"),
            PieceKind::Rook => write!(f, "r"),
            PieceKind::Queen => write!(f, "q"),
            PieceKind::King => write!(f, "k"),
        }
    }
}

#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Piece(pub PieceKind, pub Color);

impl Piece {
    pub fn kind(self) -> PieceKind {
        self.0
    }

    pub fn color(self) -> Color {
        self.1
    }

    /// Iterates over all possibles value of `Piece`
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
    type Error = &'static str;

    fn try_from(value: char) -> Result<Self, Self::Error> {
        let kind = PieceKind::try_from(value)?;

        let color;
        color = if value.is_ascii_uppercase() {
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
            Color::White => write!(f, "{}", format!("{}", self.kind()).to_ascii_uppercase()),
            Color::Black => write!(f, "{}", self.kind()),
        }
    }
}
