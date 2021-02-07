use std::convert::TryFrom;

use crate::framework::{CastlingRights, PieceMap, Side};
use crate::framework::color::Color;
use crate::framework::fen::{FenParseError, STARTING_FEN};
use crate::framework::moves::Move;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::square::Square;
use crate::standard::piece_map::BitboardPieceMap;
use crate::standard::position::castling::StandardCastlingRights;

#[cfg(test)]
mod tests;
mod castling;

pub struct Position {
    pieces: BitboardPieceMap,
    to_move: Color,
    castling: StandardCastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u8,
    move_number: u32,
}

impl Position {
    /// Creates default chess starting `Position`
    pub fn new() -> Self {
        Position::from_fen(STARTING_FEN).unwrap()
    }

    /// Creates `Position` from `fen`
    pub fn from_fen(fen: &str) -> Result<Self, FenParseError> {
        let fields: Vec<&str> = fen.split(' ').collect();
        if fields.len() != 6 {
            return Err(FenParseError::from("Incorrect number of fields"));
        }

        // Piece placement
        let mut pieces = BitboardPieceMap::new();
        let rows: Vec<&str> = fields[0].split('/').rev().collect();
        if rows.len() != 8 {
            return Err(FenParseError::from("Incorrect number of rows"));
        }
        for (r, row) in rows.into_iter().enumerate() {
            let mut c = 0;
            for ch in row.chars() {
                if c >= 8 {
                    return Err(FenParseError::from("Row formatted incorrectly"));
                }
                if let Some(n) = ch.to_digit(10) {
                    c += n as usize;
                } else {
                    let sq = Square::try_from((8 * r + c) as u8).unwrap();
                    pieces.set_sq(sq, Piece::try_from(ch)?);
                    c += 1;
                }
            }
        }
        if pieces.get_bb(Piece(PieceKind::King, Color::White)).len() != 1
            || pieces.get_bb(Piece(PieceKind::King, Color::Black)).len() != 1 {
            return Err(FenParseError::from("Each player must have exactly one king"));
        }

        // Player to move
        let to_move = match fields[1] {
            "w" => Color::White,
            "b" => Color::Black,
            _ => return Err(FenParseError::from("Invalid color")),
        };

        // Castling rights
        let mut castling = StandardCastlingRights::new(false, false, false, false);
        for right in fields[2].chars() {
            match right {
                'K' => castling.set(Color::White, Side::KingSide, true),
                'Q' => castling.set(Color::White, Side::QueenSide, true),
                'k' => castling.set(Color::Black, Side::KingSide, true),
                'q' => castling.set(Color::Black, Side::QueenSide, true),
                '-' => break,
                _ => return Err(FenParseError::from("Invalid castling right")),
            }
        }

        // En passant square
        let en_passant_sq = match fields[3] {
            "-" => None,
            _ => Some(Square::try_from(fields[3])?),
        };

        // Ply clock
        let ply_clock = fields[4].parse()
            .map_err(|_| "Invalid ply clock")?;

        // Move number
        let move_number = fields[5].parse()
            .map_err(|_| "Invalid move number")?;

        Ok(Position {
            pieces,
            to_move,
            castling,
            en_passant_sq,
            ply_clock,
            move_number,
        })
    }

    pub fn pieces(&self) -> &BitboardPieceMap {
        &self.pieces
    }

    pub fn to_move(&self) -> Color {
        self.to_move
    }

    pub fn castling(&self) -> &StandardCastlingRights {
        &self.castling
    }

    pub fn en_passant_sq(&self) -> Option<Square> {
        self.en_passant_sq
    }

    /// Makes move `m`
    pub fn make_move(&mut self, m: Move) {
        unimplemented!()
    }

    /// Unmakes last move
    pub fn unmake_move(&mut self) {
        unimplemented!()
    }
}