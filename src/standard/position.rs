use std::convert::TryFrom;

use crate::framework::Side;
use crate::framework::color::Color;
use crate::framework::fen::{FenParseError, STARTING_FEN};
use crate::framework::moves::Move;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::square::Square;
use crate::standard::piece_map::BitboardPieceMap;
use crate::standard::position::castling::CastlingRights;
use std::hint::unreachable_unchecked;
use crate::framework::direction::Direction;

#[cfg(test)]
mod tests;
mod castling;

pub struct Position {
    pieces: BitboardPieceMap,
    to_move: Color,
    castling: CastlingRights,
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
        let mut castling = CastlingRights::new(false, false, false, false);
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

    pub fn castling(&self) -> &CastlingRights {
        &self.castling
    }

    pub fn en_passant_sq(&self) -> Option<Square> {
        self.en_passant_sq
    }

    /// Makes move `m`
    /// # Safety
    /// `m` must be a legal move
    pub unsafe fn make_move(&mut self, m: Move) {
        match m {
            Move::Regular(from, to) => {
                debug_assert!(self.pieces.get(from).is_some());
                let pce = self.pieces.get(from)
                    .unwrap_or_else(|| unreachable_unchecked());
                let dest_pce = self.pieces.get(to);

                self.pieces.set_sq(to, pce);
                self.pieces.unset_sq(from);

                self.en_passant_sq = None;

                if pce.kind() == PieceKind::Pawn {
                    self.ply_clock = 0;

                    let (snd_rank, frth_rank, up) = match self.to_move {
                        Color::White => (1, 3, Direction::North),
                        Color::Black => (6, 4, Direction::South),
                    };

                    if from.rank() == snd_rank && to.rank() == frth_rank {
                        self.en_passant_sq = Some(Square::from_unchecked((from as i8 + up as i8) as u8));
                    }
                } else {
                    let (king_sq, king_rook_sq, queen_rook_sq) = match self.to_move {
                        Color::White => (Square::E1, Square::H1, Square::A1),
                        Color::Black => (Square::E1, Square::H1, Square::A1),
                    };

                    if from == king_rook_sq {
                        self.castling.set(self.to_move, Side::KingSide, false);
                    } else if from == queen_rook_sq {
                        self.castling.set(self.to_move, Side::QueenSide, false);
                    } else if from == king_sq {
                        self.castling.set(self.to_move, Side::KingSide, false);
                        self.castling.set(self.to_move, Side::QueenSide, false);
                    }

                    if dest_pce.is_some() {
                        self.ply_clock = 0;
                    } else {
                        self.ply_clock += 1;
                    }
                }
            }
            Move::Castling(_) => unimplemented!(),
            Move::Promotion(_, _, _) => unimplemented!(),
            Move::EnPassant(_, _) => unimplemented!(),
        }

        match self.to_move {
            Color::White => self.to_move = Color::Black,
            Color::Black => {
                self.to_move = Color::White;
                self.move_number += 1;
            }
        }
    }

    /// Unmakes last move
    pub fn unmake_move(&mut self) {
        unimplemented!()
    }
}