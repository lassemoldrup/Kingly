use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};
use std::hint::unreachable_unchecked;
use std::mem;

use intmap::IntMap;

use crate::fen::{FenParseError, STARTING_FEN};
use crate::tables::Tables;
use crate::types::{
    CastlingRights, Color, Direction, Move, MoveKind, Piece, PieceKind, Side, Square,
};
use crate::util::{get_castling_rook_sq, get_rook_sq};
use crate::zobrist::ZobristKey;

use self::pieces::Pieces;

mod pieces;
#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Position {
    pub pieces: Pieces,
    pub to_move: Color,
    pub castling: CastlingRights,
    pub en_passant_sq: Option<Square>,
    ply_clock: u8,
    pub move_number: u32,
    history: Vec<Unmake>,
    repetitions: IntMap<u8>,
    pub zobrist: u64,
    tables: &'static Tables,
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
        let mut pieces = Pieces::new();
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
            || pieces.get_bb(Piece(PieceKind::King, Color::Black)).len() != 1
        {
            return Err(FenParseError::from(
                "Each player must have exactly one king",
            ));
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
                'K' => castling.set(Color::White, 0b01),
                'Q' => castling.set(Color::White, 0b10),
                'k' => castling.set(Color::Black, 0b01),
                'q' => castling.set(Color::Black, 0b10),
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
        let ply_clock = fields[4].parse().map_err(|_| "Invalid ply clock")?;

        // Move number
        let move_number = fields[5].parse().map_err(|_| "Invalid move number")?;

        // Zobrist hash
        let tables = Tables::get();
        let mut zobrist = 0;

        for pce in Piece::iter() {
            zobrist ^= (pce, pieces.get_bb(pce)).key(tables);
        }
        zobrist ^= to_move.key(tables);
        zobrist ^= castling.key(tables);
        zobrist ^= en_passant_sq.key(tables);

        // Repetition table
        let mut repetitions = IntMap::new();
        repetitions.insert(zobrist, 1);

        Ok(Position {
            pieces,
            to_move,
            castling,
            en_passant_sq,
            ply_clock,
            move_number,
            history: Vec::new(),
            repetitions,
            zobrist,
            tables,
        })
    }

    /// Sets the `fen` of the `Position` without changing the repetition history
    pub fn set_fen(&mut self, fen: &str) -> Result<(), FenParseError> {
        let mut new_pos = Self::from_fen(fen)?;
        mem::swap(&mut self.repetitions, &mut new_pos.repetitions);
        *self = new_pos;
        Ok(())
    }

    /// Makes move `m`
    /// # Safety
    /// `m` must be a legal move
    pub unsafe fn make_move(&mut self, mv: Move) {
        let mut unmake = Unmake {
            mv,
            capture: None,
            castling: self.castling,
            en_passant_sq: self.en_passant_sq,
            ply_clock: self.ply_clock,
        };

        self.toggle_zobrist(&self.en_passant_sq.clone());
        self.en_passant_sq = None;

        if !mv.is_null() {
            let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
            let enemy_pawn_pce = Piece(PieceKind::Pawn, !self.to_move);
            let rook_pce = Piece(PieceKind::Rook, self.to_move);
            let king_pce = Piece(PieceKind::King, self.to_move);

            let from = mv.from();
            let to = mv.to();
            match mv.kind() {
                MoveKind::Regular => {
                    debug_assert!(self.pieces.get(from).is_some());
                    let pce = self.pieces.get(from).unwrap_unchecked();
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
                            Color::White => (1, 3, Direction::North),
                            Color::Black => (6, 4, Direction::South),
                        };

                        if from.rank() == snd_rank && to.rank() == frth_rank {
                            self.en_passant_sq = Some(from.shift(up));
                            self.toggle_zobrist(&Some(from));
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
                        Square::C1 | Square::C8 => Side::QueenSide,
                        _ => unreachable_unchecked(),
                    };
                    let rook_sq = get_rook_sq(self.to_move, side);
                    let castling_rook_sq = get_castling_rook_sq(self.to_move, side);

                    self.unset_sq(from, king_pce);
                    self.unset_sq(rook_sq, rook_pce);
                    self.set_sq(to, king_pce);
                    self.set_sq(castling_rook_sq, rook_pce);

                    self.remove_castling_rights(self.to_move, 0b11);

                    self.ply_clock += 1;
                }
                MoveKind::Promotion => {
                    unmake.capture = self.pieces.get(to);

                    let promotion_pce = Piece(mv.promotion(), self.to_move);

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
                        Color::White => Direction::South,
                        Color::Black => Direction::North,
                    };

                    self.set_sq(to, pawn_pce);
                    self.unset_sq(from, pawn_pce);
                    let captured_pawn_sq = to.shift(down);
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
        self.toggle_zobrist(&Color::White);

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
        if to == get_rook_sq(opp, Side::KingSide) {
            self.remove_castling_rights(opp, 0b01);
        } else if to == get_rook_sq(opp, Side::QueenSide) {
            self.remove_castling_rights(opp, 0b10);
        }
    }

    /// Unmakes last move
    /// # Safety
    /// There must be a move to unmake
    pub unsafe fn unmake_move(&mut self) {
        debug_assert!(!self.history.is_empty());
        let unmake = self.history.pop().unwrap_unchecked();

        match self.repetitions.get_mut(self.zobrist) {
            Some(count) => {
                *count -= 1;
                if *count == 0 {
                    self.repetitions.remove(self.zobrist);
                }
            }
            None => {
                debug_assert!(false);
                unreachable_unchecked()
            }
        }

        self.toggle_zobrist(&self.en_passant_sq.clone());
        self.en_passant_sq = unmake.en_passant_sq;
        self.toggle_zobrist(&self.en_passant_sq.clone());
        self.ply_clock = unmake.ply_clock;

        match self.to_move {
            Color::White => {
                self.to_move = Color::Black;
                self.move_number -= 1;
            }
            Color::Black => self.to_move = Color::White,
        }
        self.toggle_zobrist(&Color::White);

        if unmake.mv.is_null() {
            return;
        }

        let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
        let enemy_pawn_pce = Piece(PieceKind::Pawn, !self.to_move);
        let rook_pce = Piece(PieceKind::Rook, self.to_move);
        let king_pce = Piece(PieceKind::King, self.to_move);

        let from = unmake.mv.from();
        let to = unmake.mv.to();
        match unmake.mv.kind() {
            MoveKind::Regular => {
                debug_assert!(self.pieces.get(to).is_some());
                let pce = self.pieces.get(to).unwrap_unchecked();

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
                    Square::C1 | Square::C8 => Side::QueenSide,
                    _ => unreachable_unchecked(),
                };
                let rook_sq = get_rook_sq(self.to_move, side);
                let castling_rook_sq = get_castling_rook_sq(self.to_move, side);

                self.set_sq(from, king_pce);
                self.set_sq(rook_sq, rook_pce);
                self.unset_sq(to, king_pce);
                self.unset_sq(castling_rook_sq, rook_pce);

                self.set_castling(unmake.castling);
            }
            MoveKind::Promotion => {
                let promotion_pce = Piece(unmake.mv.promotion(), self.to_move);

                self.set_sq(from, pawn_pce);
                self.unset_sq(to, promotion_pce);
                if let Some(pce) = unmake.capture {
                    self.set_sq(to, pce);
                }

                self.set_castling(unmake.castling);
            }
            MoveKind::EnPassant => {
                let down = match self.to_move {
                    Color::White => Direction::South,
                    Color::Black => Direction::North,
                };
                let ep_pawn_sq = to.shift(down);

                self.set_sq(from, pawn_pce);
                self.unset_sq(to, pawn_pce);
                self.set_sq(ep_pawn_sq, enemy_pawn_pce);
            }
        }
    }

    fn set_sq(&mut self, sq: Square, pce: Piece) {
        self.pieces.set_sq(sq, pce);
        self.toggle_zobrist(&(pce, sq));
    }

    fn unset_sq(&mut self, sq: Square, pce: Piece) {
        self.pieces.unset_sq(sq);
        self.toggle_zobrist(&(pce, sq));
    }

    fn set_castling(&mut self, castling: CastlingRights) {
        self.toggle_zobrist(&self.castling.clone());
        self.castling = castling;
        self.toggle_zobrist(&self.castling.clone());
    }

    fn remove_castling_rights(&mut self, color: Color, rights: usize) {
        self.toggle_zobrist(&self.castling.clone());
        self.castling.remove(color, rights);
        self.toggle_zobrist(&self.castling.clone());
    }

    fn toggle_zobrist(&mut self, key: &impl ZobristKey) {
        self.zobrist ^= key.key(self.tables);
    }

    pub fn last_move(&self) -> Option<Move> {
        self.history.last().map(|um| um.mv)
    }

    pub fn is_draw(&self) -> bool {
        let threefold = match self.repetitions.get(self.zobrist) {
            Some(&count) => count >= 3,
            // The current position is always in the table
            None => unsafe {
                debug_assert!(false);
                unreachable_unchecked()
            },
        };

        threefold || self.ply_clock >= 100
    }

    /// Returns a heuristic of whether a null move can be made
    /// without risking missing zugzwang
    pub fn null_move_heuristic(&self) -> bool {
        // Null moves are not considered if the player to move only has king and pawns
        let total_pieces = self.pieces.get_occ_for(self.to_move).len();
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
        write!(f, "{}to_move: {:?}\ncastling: {:?}\nen_passant_sq: {:?}\nply_clock: {:?}\nmove_number: {:?}\nrepetitions: {:?}\nhistory: {:?}\n",
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
