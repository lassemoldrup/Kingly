use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::mem;

use intmap::IntMap;

use crate::tables::Tables;
use crate::types::{
    BoardVector, CastlingRights, Color, Move, MoveKind, Piece, PieceKind, Rank, Side, Square,
};
use crate::zobrist::ZobristKey;

use pieces::Pieces;

mod fen;
pub use fen::*;
mod pieces;
#[cfg(test)]
mod tests;

/// Represents a chess position.
#[derive(Clone)]
pub struct Position {
    pub pieces: Pieces,
    pub to_move: Color,
    pub castling: CastlingRights,
    pub en_passant_sq: Option<Square>,
    ply_clock: u8,
    pub move_number: u32,
    repetitions: IntMap<u8>,
    history: Vec<Unmake>,
    pub zobrist: u64,
    tables: &'static Tables,
}

impl Position {
    /// Creates default chess starting position.
    pub fn new() -> Self {
        Position::from_fen(STARTING_FEN).unwrap()
    }

    /// Sets the position to the given FEN string without changing the repetition history.
    pub fn set_fen(&mut self, fen: &str) -> Result<(), FenParseError> {
        let mut new_pos = Self::from_fen(fen)?;
        mem::swap(&mut self.repetitions, &mut new_pos.repetitions);
        *self = new_pos;
        Ok(())
    }

    /// Makes a move in the position.
    ///
    /// # Panics
    /// May panic if the move is invalid for the position.
    pub fn make_move(&mut self, mv: Move) {
        let mut unmake = Unmake {
            mv,
            capture: None,
            castling: self.castling,
            en_passant_sq: self.en_passant_sq,
            ply_clock: self.ply_clock,
        };

        self.toggle_zobrist(self.en_passant_sq);
        self.en_passant_sq = None;

        if !mv.is_null() {
            let from = mv.from();
            let to = mv.to();
            match mv.kind() {
                MoveKind::Regular => {
                    let pce = self
                        .pieces
                        .get(from)
                        .expect("there should be a piece at from");
                    let dest_pce = self.pieces.get(to);
                    unmake.capture = dest_pce;

                    self.unset_sq(from, pce);
                    match dest_pce {
                        Some(dest_pce) => {
                            self.unset_sq(to, dest_pce);
                            self.remove_castling_on_rook_capture(to);
                            self.ply_clock = 0;
                        }
                        None => self.ply_clock += 1,
                    }
                    self.set_sq(to, pce);

                    if pce.kind() == PieceKind::Pawn {
                        self.ply_clock = 0;

                        let (snd_rank, frth_rank, up) = match self.to_move {
                            Color::White => (Rank::Second, Rank::Fourth, BoardVector::NORTH),
                            Color::Black => (Rank::Seventh, Rank::Fifth, BoardVector::SOUTH),
                        };

                        if from.rank() == snd_rank && to.rank() == frth_rank {
                            self.en_passant_sq = Some(from + up);
                            self.toggle_zobrist(Some(from));
                        }
                    } else {
                        let (king_sq, king_rook_sq, queen_rook_sq) = match self.to_move {
                            Color::White => (Square::E1, Square::H1, Square::A1),
                            Color::Black => (Square::E8, Square::H8, Square::A8),
                        };

                        if from == king_rook_sq {
                            self.remove_castling_rights(self.to_move, 0b01);
                        } else if from == queen_rook_sq {
                            self.remove_castling_rights(self.to_move, 0b10);
                        } else if from == king_sq {
                            self.remove_castling_rights(self.to_move, 0b11);
                        }
                    }
                }
                MoveKind::Castling => {
                    let side = match to {
                        Square::G1 | Square::G8 => Side::KingSide,
                        _ => {
                            debug_assert!(to == Square::C1 || to == Square::C8);
                            Side::QueenSide
                        }
                    };
                    let rook_sq = Square::rook_starting(self.to_move, side);
                    let castling_rook_sq = Square::rook_castling_dest(self.to_move, side);

                    let rook_pce = Piece(PieceKind::Rook, self.to_move);
                    let king_pce = Piece(PieceKind::King, self.to_move);
                    self.unset_sq(from, king_pce);
                    self.unset_sq(rook_sq, rook_pce);
                    self.set_sq(to, king_pce);
                    self.set_sq(castling_rook_sq, rook_pce);

                    self.remove_castling_rights(self.to_move, 0b11);

                    self.ply_clock += 1;
                }
                MoveKind::Promotion(kind) => {
                    unmake.capture = self.pieces.get(to);

                    let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
                    let promotion_pce = Piece(kind, self.to_move);
                    if let Some(dest_pce) = unmake.capture {
                        self.unset_sq(to, dest_pce);
                        self.remove_castling_on_rook_capture(to);
                    }
                    self.set_sq(to, promotion_pce);
                    self.unset_sq(from, pawn_pce);

                    self.ply_clock = 0;
                }
                MoveKind::EnPassant => {
                    let down = match self.to_move {
                        Color::White => BoardVector::SOUTH,
                        Color::Black => BoardVector::NORTH,
                    };

                    let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
                    let enemy_pawn_pce = Piece(PieceKind::Pawn, !self.to_move);
                    self.set_sq(to, pawn_pce);
                    self.unset_sq(from, pawn_pce);
                    let captured_pawn_sq = to + down;
                    self.unset_sq(captured_pawn_sq, enemy_pawn_pce);

                    self.ply_clock = 0;
                }
            }
        }

        match self.to_move {
            Color::White => self.to_move = Color::Black,
            Color::Black => {
                self.to_move = Color::White;
                self.move_number += 1;
            }
        }
        self.toggle_zobrist(Color::White);

        match self.repetitions.get_mut(self.zobrist) {
            Some(count) => *count += 1,
            None => {
                self.repetitions.insert(self.zobrist, 1);
            }
        }

        self.history.push(unmake);
    }

    fn remove_castling_on_rook_capture(&mut self, to: Square) {
        let opp = !self.to_move;
        if to == Square::rook_starting(opp, Side::KingSide) {
            self.remove_castling_rights(opp, 0b01);
        } else if to == Square::rook_starting(opp, Side::QueenSide) {
            self.remove_castling_rights(opp, 0b10);
        }
    }

    /// Unmakes the last made move in the position.
    ///
    /// # Panics
    /// Panics if there are no moves to unmake.
    pub fn unmake_move(&mut self) {
        let unmake = self
            .history
            .pop()
            .expect("there should be a move to unmake");
        let count = self
            .repetitions
            .get_mut(self.zobrist)
            .expect("the position we are unmaking is in the repetitions table");
        *count -= 1;
        if *count == 0 {
            self.repetitions.remove(self.zobrist);
        }

        self.toggle_zobrist(self.en_passant_sq);
        self.en_passant_sq = unmake.en_passant_sq;
        self.toggle_zobrist(self.en_passant_sq);
        self.ply_clock = unmake.ply_clock;

        match self.to_move {
            Color::White => {
                self.to_move = Color::Black;
                self.move_number -= 1;
            }
            Color::Black => self.to_move = Color::White,
        }
        self.toggle_zobrist(Color::White);

        if unmake.mv.is_null() {
            return;
        }

        let from = unmake.mv.from();
        let to = unmake.mv.to();
        match unmake.mv.kind() {
            MoveKind::Regular => {
                let pce = self
                    .pieces
                    .get(to)
                    .expect("the moved piece should be at to");

                self.set_sq(from, pce);
                self.unset_sq(to, pce);
                if let Some(cap_pce) = unmake.capture {
                    self.set_sq(to, cap_pce);
                }

                self.set_castling(unmake.castling);
            }
            MoveKind::Castling => {
                let side = match to {
                    Square::G1 | Square::G8 => Side::KingSide,
                    _ => {
                        debug_assert!(to == Square::C1 || to == Square::C8);
                        Side::QueenSide
                    }
                };
                let rook_sq = Square::rook_starting(self.to_move, side);
                let castling_rook_sq = Square::rook_castling_dest(self.to_move, side);

                let rook_pce = Piece(PieceKind::Rook, self.to_move);
                let king_pce = Piece(PieceKind::King, self.to_move);
                self.set_sq(from, king_pce);
                self.set_sq(rook_sq, rook_pce);
                self.unset_sq(to, king_pce);
                self.unset_sq(castling_rook_sq, rook_pce);

                self.set_castling(unmake.castling);
            }
            MoveKind::Promotion(kind) => {
                let promotion_pce = Piece(kind, self.to_move);

                let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
                self.set_sq(from, pawn_pce);
                self.unset_sq(to, promotion_pce);
                if let Some(pce) = unmake.capture {
                    self.set_sq(to, pce);
                }

                self.set_castling(unmake.castling);
            }
            MoveKind::EnPassant => {
                let down = match self.to_move {
                    Color::White => BoardVector::SOUTH,
                    Color::Black => BoardVector::NORTH,
                };
                let ep_pawn_sq = to + down;

                let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
                let enemy_pawn_pce = Piece(PieceKind::Pawn, !self.to_move);
                self.set_sq(from, pawn_pce);
                self.unset_sq(to, pawn_pce);
                self.set_sq(ep_pawn_sq, enemy_pawn_pce);
            }
        }
    }

    fn set_sq(&mut self, sq: Square, pce: Piece) {
        self.pieces.set_sq(sq, pce);
        self.toggle_zobrist((pce, sq));
    }

    fn unset_sq(&mut self, sq: Square, pce: Piece) {
        self.pieces.unset_sq(sq);
        self.toggle_zobrist((pce, sq));
    }

    fn set_castling(&mut self, castling: CastlingRights) {
        self.toggle_zobrist(self.castling);
        self.castling = castling;
        self.toggle_zobrist(self.castling);
    }

    fn remove_castling_rights(&mut self, color: Color, rights: u8) {
        self.toggle_zobrist(self.castling);
        self.castling.remove(color, rights);
        self.toggle_zobrist(self.castling);
    }

    fn toggle_zobrist(&mut self, key: impl ZobristKey) {
        self.zobrist ^= key.key(self.tables);
    }

    /// Returns the last move made in the position.
    #[inline]
    pub fn last_move(&self) -> Option<Move> {
        self.history.last().map(|um| um.mv)
    }

    /// Returns whether the position is a draw by threefold repetition or the fifty-move rule.
    #[inline]
    pub fn is_draw(&self) -> bool {
        debug_assert!(self.repetitions.contains_key(self.zobrist));
        let count = *self.repetitions.get(self.zobrist).unwrap_or(&0);
        let threefold = count >= 3;
        threefold || self.ply_clock >= 100
    }

    /// Returns a heuristic of whether a null move can be made
    /// without risking missing zugzwang
    #[inline]
    pub fn null_move_heuristic(&self) -> bool {
        // Null moves are not considered if the player to move only has king and pawns
        let total_pieces = self.pieces.occupied_for(self.to_move).len();
        let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
        let pawns = self.pieces.get_bb(pawn_pce).len();

        total_pieces - pawns > 1
    }

    /// Returns `true` if the current position matches all the fields of `fen`
    pub fn matches_fen(&self, fen: &str) -> Result<bool, FenParseError> {
        let other = Self::from_fen(fen)?;
        Ok(self.zobrist == other.zobrist
            && self.ply_clock == other.ply_clock
            && self.move_number == other.move_number)
    }
}

impl Default for Position {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        self.zobrist == other.zobrist
            && self.ply_clock == other.ply_clock
            && self.move_number == other.move_number
            && self.repetitions == other.repetitions
            && self.history == other.history
    }
}

impl Debug for Position {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        writeln!(f, "{}to_move: {:?}\ncastling: {:?}\nen_passant_sq: {:?}\nply_clock: {:?}\nmove_number: {:?}\nrepetitions: {:?}\nhistory: {:?}",
               self.pieces, self.to_move, self.castling, self.en_passant_sq, self.ply_clock, self.move_number, self.repetitions, self.history)
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        for r in (0..8).rev() {
            for c in 0..8 {
                let sq = Square::try_from(r * 8 + c).unwrap();
                match self.pieces.get(sq) {
                    Some(pce) => write!(f, "{} ", pce)?,
                    None => write!(f, ". ")?,
                }
            }
            writeln!(f)?;
        }
        writeln!(f, "To move: {}", self.to_move)
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
struct Unmake {
    mv: Move,
    capture: Option<Piece>,
    castling: CastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u8,
}
