use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};
use std::hint::unreachable_unchecked;

use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::fen::{FenParseError, STARTING_FEN};
use crate::framework::moves::Move;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::Side;
use crate::framework::square::Square;
use crate::standard::piece_map::BitboardPieceMap;
use crate::standard::position::castling::CastlingRights;

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
    history: Vec<Unmake>,
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
            history: Vec::new(),
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

    pub fn last_move(&self) -> Option<Move> {
        self.history.last()
            .map(|um| um.mv)
    }

    /// Makes move `m`
    /// # Safety
    /// `m` must be a legal move
    pub unsafe fn make_move(&mut self, m: Move) {
        let mut unmake = Unmake {
            mv: m,
            capture: None,
            castling: self.castling,
            en_passant_sq: self.en_passant_sq,
            ply_clock: self.ply_clock,
        };

        self.en_passant_sq = None;
        match m {
            Move::Regular(from, to) => {
                debug_assert!(self.pieces.get(from).is_some());
                let pce = self.pieces.get(from)
                    .unwrap_or_else(|| unreachable_unchecked());
                let dest_pce = self.pieces.get(to);

                unmake.capture = dest_pce;

                self.pieces.unset_sq(from);

                self.remove_castling_on_rook_capture(to);

                if pce.kind() == PieceKind::Pawn {
                    self.pieces.unset_sq(to);
                    self.pieces.set_sq(to, pce);

                    self.ply_clock = 0;

                    let (snd_rank, frth_rank, up) = match self.to_move {
                        Color::White => (1, 3, Direction::North),
                        Color::Black => (6, 4, Direction::South),
                    };

                    if from.rank() == snd_rank && to.rank() == frth_rank {
                        self.en_passant_sq = Some(from.shift(up));
                    }
                } else {
                    if dest_pce.is_some() {
                        self.pieces.unset_sq(to);
                        self.pieces.set_sq(to, pce);

                        self.ply_clock = 0;
                    } else {
                        self.pieces.set_sq(to, pce);

                        self.ply_clock += 1;
                    }

                    let (king_sq, king_rook_sq, queen_rook_sq) = match self.to_move {
                        Color::White => (Square::E1, Square::H1, Square::A1),
                        Color::Black => (Square::E8, Square::H8, Square::A8),
                    };

                    if from == king_rook_sq {
                        self.castling.set(self.to_move, Side::KingSide, false);
                    } else if from == queen_rook_sq {
                        self.castling.set(self.to_move, Side::QueenSide, false);
                    } else if from == king_sq {
                        self.castling.set(self.to_move, Side::KingSide, false);
                        self.castling.set(self.to_move, Side::QueenSide, false);
                    }
                }
            }
            Move::Castling(side) => {
                let king_sq = match self.to_move {
                    Color::White => Square::E1,
                    Color::Black => Square::E8,
                };
                let castling_sq = CastlingRights::get_castling_sq(self.to_move, side);
                let rook_sq = CastlingRights::get_rook_sq(self.to_move, side);
                let castling_rook_sq = CastlingRights::get_castling_rook_sq(self.to_move, side);

                self.pieces.set_sq(castling_sq, Piece(PieceKind::King, self.to_move));
                self.pieces.set_sq(castling_rook_sq, Piece(PieceKind::Rook, self.to_move));
                self.pieces.unset_sq(king_sq);
                self.pieces.unset_sq(rook_sq);

                self.castling.set(self.to_move, Side::KingSide, false);
                self.castling.set(self.to_move, Side::QueenSide, false);

                self.ply_clock += 1;
            },
            Move::Promotion(from, to, kind) => {
                unmake.capture = self.pieces.get(to);

                self.pieces.unset_sq(to);
                self.pieces.set_sq(to, Piece(kind, self.to_move));
                self.pieces.unset_sq(from);

                self.remove_castling_on_rook_capture(to);

                self.ply_clock = 0;
            },
            Move::EnPassant(from, to) => {
                let down = match self.to_move {
                    Color::White => Direction::South,
                    Color::Black => Direction::North,
                };

                self.pieces.set_sq(to, Piece(PieceKind::Pawn, self.to_move));
                self.pieces.unset_sq(from);
                let ep_pawn_sq = to.shift(down);
                self.pieces.unset_sq(ep_pawn_sq);

                self.ply_clock = 0;
            },
        }

        match self.to_move {
            Color::White => self.to_move = Color::Black,
            Color::Black => {
                self.to_move = Color::White;
                self.move_number += 1;
            },
        }

        self.history.push(unmake);
    }

    fn remove_castling_on_rook_capture(&mut self, to: Square) {
        let opp = !self.to_move;
        if to == CastlingRights::get_rook_sq(opp, Side::KingSide) {
            self.castling.set(opp, Side::KingSide, false);
        } else if to == CastlingRights::get_rook_sq(opp, Side::QueenSide) {
            self.castling.set(opp, Side::QueenSide, false);
        }
    }

    /// Unmakes last move
    /// # Safety
    /// There must be a move to unmake
    pub unsafe fn unmake_move(&mut self) {
        debug_assert!(!self.history.is_empty());
        let unmake = self.history.pop()
            .unwrap_or_else(|| unreachable_unchecked());

        self.en_passant_sq = unmake.en_passant_sq;
        self.ply_clock = unmake.ply_clock;

        match self.to_move {
            Color::White => {
                self.to_move = Color::Black;
                self.move_number -= 1;
            },
            Color::Black => self.to_move = Color::White,
        }

        match unmake.mv {
            Move::Regular(from, to) => {
                debug_assert!(self.pieces.get(to).is_some());
                let pce = self.pieces.get(to)
                    .unwrap_or_else(|| unreachable_unchecked());

                self.pieces.set_sq(from, pce);
                self.pieces.unset_sq(to);
                if let Some(cap_pce) = unmake.capture {
                    self.pieces.set_sq(to, cap_pce);
                }

                self.castling = unmake.castling;

            },
            Move::Castling(side) => {
                let king_sq = match self.to_move {
                    Color::White => Square::E1,
                    Color::Black => Square::E8,
                };
                let castling_sq = CastlingRights::get_castling_sq(self.to_move, side);
                let rook_sq = CastlingRights::get_rook_sq(self.to_move, side);
                let castling_rook_sq = CastlingRights::get_castling_rook_sq(self.to_move, side);

                self.pieces.set_sq(king_sq, Piece(PieceKind::King, self.to_move));
                self.pieces.set_sq(rook_sq, Piece(PieceKind::Rook, self.to_move));
                self.pieces.unset_sq(castling_sq);
                self.pieces.unset_sq(castling_rook_sq);

                self.castling = unmake.castling;
            },
            Move::Promotion(from, to, _) => {
                self.pieces.set_sq(from, Piece(PieceKind::Pawn, self.to_move));
                self.pieces.unset_sq(to);
                if let Some(pce) = unmake.capture {
                    self.pieces.set_sq(to, pce);
                }

                self.castling = unmake.castling;
            },
            Move::EnPassant(from, to) => {
                let down = match self.to_move {
                    Color::White => Direction::South,
                    Color::Black => Direction::North,
                };
                let ep_pawn_sq = to.shift(down);

                self.pieces.set_sq(from, Piece(PieceKind::Pawn, self.to_move));
                self.pieces.unset_sq(to);
                self.pieces.set_sq(ep_pawn_sq, Piece(PieceKind::Pawn, !self.to_move));
            }
        }
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.pieces == other.pieces
        && self.castling == other.castling
        && self.to_move == other.to_move
        && self.en_passant_sq == other.en_passant_sq
        && self.ply_clock == other.ply_clock
        && self.move_number == other.move_number
    }
}

impl Debug for Position {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}\nto_move: {:?}\ncastling: {:?}\nen_passant_sq: {:?}\nply_clock: {:?}\nmove_number: {:?}",
               self.pieces, self.to_move, self.castling, self.en_passant_sq, self.ply_clock, self.move_number)
    }
}


struct Unmake {
    mv: Move,
    capture: Option<Piece>,
    castling: CastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u8,
}