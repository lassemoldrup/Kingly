use crate::framework::{Position, PieceMap};
use crate::framework::moves::Move;
use crate::framework::square::Square;
use crate::framework::color::Color;
use crate::framework::fen::FenParseError;
use crate::framework::piece::Piece;
use std::convert::TryFrom;

#[cfg(test)]
mod tests;

pub struct StandardPosition<P: PieceMap> {
    pieces: P,
    to_move: Color,
    en_passant_sq: Option<Square>,
    ply_clock: u32,
    move_number: u32,
}

impl<P: PieceMap> StandardPosition<P> {
    fn get_piece_map(&self) -> &P {
        &self.pieces
    }
}

impl<P: PieceMap> Position for StandardPosition<P> {
    fn new() -> Self {
        unimplemented!()
    }

    fn from_fen(fen: &str) -> Result<Self, FenParseError> {
        let fields: Vec<&str> = fen.split(' ').collect();
        if fields.len() != 6 {
            return Err(FenParseError::new("Incorrect number of fields"));
        }

        let mut pieces = P::new();
        let rows: Vec<&str> = fields[0].split('/').rev().collect();
        if rows.len() != 8 {
            return Err(FenParseError::new("Incorrect number of rows"));
        }
        for (r, row) in rows.into_iter().enumerate() {
            let mut c = 0;
            for ch in row.chars() {
                if c >= 8 {
                    return Err(FenParseError::new("Row formatted incorrectly"));
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


        Ok(StandardPosition {
            pieces,
            to_move: Color::White,
            en_passant_sq: None,
            ply_clock: 0,
            move_number: 0,
        })
    }

    fn gen_moves(&self) -> Vec<Move> {
        unimplemented!()
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