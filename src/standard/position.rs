use crate::framework::{Position, PieceMap, SquareSet, CastlingRights, Side};
use crate::framework::moves::{Move, MoveList};
use crate::framework::square::Square;
use crate::framework::color::Color;
use crate::framework::fen::{FenParseError, STARTING_FEN};
use crate::framework::piece::Piece;
use std::convert::TryFrom;
use crate::standard::piece_map::SquareSetPieceMap;
use crate::standard::position::castling::StandardCastlingRights;
use arrayvec::ArrayVec;

#[cfg(test)]
mod tests;
mod castling;
mod move_gen;

pub struct StandardPosition<S: SquareSet + Copy> {
    pieces: SquareSetPieceMap<S>,
    to_move: Color,
    castling: StandardCastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u8,
    move_number: u32,
}

impl<S: SquareSet + Copy> Position for StandardPosition<S> {
    fn new() -> Self {
        StandardPosition::from_fen(STARTING_FEN).unwrap()
    }

    fn from_fen(fen: &str) -> Result<Self, FenParseError> {
        let fields: Vec<&str> = fen.split(' ').collect();
        if fields.len() != 6 {
            return Err(FenParseError::from("Incorrect number of fields"));
        }

        // Piece placement
        let mut pieces = SquareSetPieceMap::new();
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

        Ok(StandardPosition {
            pieces,
            to_move,
            castling,
            en_passant_sq,
            ply_clock,
            move_number,
        })
    }

    fn gen_moves(&self) -> MoveList {
        let mut moves = MoveList::new();



        moves
    }

    fn make_move(&mut self, m: Move) {
        unimplemented!()
    }

    fn unmake_move(&mut self) {
        unimplemented!()
    }

    fn evaluate(&self) -> i32 {
        unimplemented!()
    }
}