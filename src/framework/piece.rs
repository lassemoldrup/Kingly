use crate::framework::color::Color;
use std::convert::TryFrom;

#[derive(PartialEq, Debug, Copy, Clone)]
pub enum PieceKind {
    Pawn,
    Knight,
    Bishop,
    Rook,
    Queen,
    King,
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


#[derive(PartialEq, Debug, Copy, Clone)]
pub struct Piece(pub PieceKind, pub Color);

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