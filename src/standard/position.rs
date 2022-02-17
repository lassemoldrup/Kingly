use std::convert::TryFrom;
use std::fmt::{Debug, Formatter};
use std::hint::unreachable_unchecked;

use crate::framework::{PieceMap, Side};
use crate::framework::castling::CastlingRights;
use crate::framework::color::Color;
use crate::framework::direction::Direction;
use crate::framework::fen::{FenParseError, STARTING_FEN};
use crate::framework::moves::Move;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::square::Square;
use crate::framework::util::{get_castling_rook_sq, get_rook_sq};
use crate::standard::piece_map::BitboardPieceMap;
use crate::standard::tables::Tables;
use crate::standard::zobrist::ZobristKey;

#[cfg(test)]
mod tests;

#[derive(Clone)]
pub struct Position {
    pieces: BitboardPieceMap,
    to_move: Color,
    castling: CastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u8,
    move_number: u32,
    history: Vec<Unmake>,
    zobrist: u64,
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
        let ply_clock = fields[4].parse()
            .map_err(|_| "Invalid ply clock")?;

        // Move number
        let move_number = fields[5].parse()
            .map_err(|_| "Invalid move number")?;

        // Zobrist hash
        let tables = Tables::get();
        let mut zobrist = 0;

        use PieceKind::*;
        for kind in [Pawn, Knight, Bishop, Rook, Queen, King] {
            for color in [Color::White, Color::Black] {
                let pce = Piece(kind, color);
                zobrist ^= (pce, pieces.get_bb(pce)).key(tables);
            }
        }
        zobrist ^= to_move.key(tables);
        zobrist ^= castling.key(tables);
        zobrist ^= en_passant_sq.key(tables);

        Ok(Position {
            pieces,
            to_move,
            castling,
            en_passant_sq,
            ply_clock,
            move_number,
            history: Vec::new(),
            zobrist,
            tables,
        })
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

        let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
        let enemy_pawn_pce = Piece(PieceKind::Pawn, !self.to_move);
        let rook_pce = Piece(PieceKind::Rook, self.to_move);
        let king_pce = Piece(PieceKind::King, self.to_move);

        self.toggle_zobrist(&self.en_passant_sq.clone());
        self.en_passant_sq = None;
        match m {
            Move::Regular(from, to) => {
                debug_assert!(self.pieces.get(from).is_some());
                let pce = self.pieces.get(from)
                    .unwrap_or_else(|| unreachable_unchecked());
                let dest_pce = self.pieces.get(to);

                unmake.capture = dest_pce;

                self.unset_sq(from, pce);
                match dest_pce {
                    Some(dest_pce) => {
                        self.unset_sq(to, dest_pce);
                        
                        self.ply_clock = 0;
                    },
                    None => self.ply_clock += 1,
                }
                self.set_sq(to, pce);

                self.remove_castling_on_rook_capture(to);

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
            Move::Castling(from, to) => {
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
            },
            Move::Promotion(from, to, kind) => {
                unmake.capture = self.pieces.get(to);

                let promotion_pce = Piece(kind, self.to_move);

                if let Some(dest_pce) = unmake.capture {
                    self.unset_sq(to, dest_pce);
                }
                self.set_sq(to, promotion_pce);
                self.unset_sq(from, pawn_pce);

                self.remove_castling_on_rook_capture(to);

                self.ply_clock = 0;
            },
            Move::EnPassant(from, to) => {
                let down = match self.to_move {
                    Color::White => Direction::South,
                    Color::Black => Direction::North,
                };

                self.set_sq(to, pawn_pce);
                self.unset_sq(from, pawn_pce);
                let captured_pawn_sq = to.shift(down);
                self.unset_sq(captured_pawn_sq, enemy_pawn_pce);

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
        self.toggle_zobrist(&Color::White);

        self.history.push(unmake);

        println!("{:#066b}", self.zobrist);
    }

    fn remove_castling_on_rook_capture(&mut self, to: Square) {
        let opp = !self.to_move;
        if to == get_rook_sq(opp, Side::KingSide) {
            self.remove_castling_rights(opp, 0b01);
        } else if to == get_rook_sq(opp, Side::QueenSide) {
            self.remove_castling_rights(opp, 0b10);
        } else {
            return;
        }
    }

    /// Unmakes last move
    /// # Safety
    /// There must be a move to unmake
    pub unsafe fn unmake_move(&mut self) {
        debug_assert!(!self.history.is_empty());
        let unmake = self.history.pop()
            .unwrap_or_else(|| unreachable_unchecked());

        self.toggle_zobrist(&self.en_passant_sq.clone());
        self.en_passant_sq = unmake.en_passant_sq;
        self.toggle_zobrist(&self.en_passant_sq.clone());
        self.ply_clock = unmake.ply_clock;

        match self.to_move {
            Color::White => {
                self.to_move = Color::Black;
                self.move_number -= 1;
            },
            Color::Black => self.to_move = Color::White,
        }
        self.toggle_zobrist(&Color::White);

        let pawn_pce = Piece(PieceKind::Pawn, self.to_move);
        let enemy_pawn_pce = Piece(PieceKind::Pawn, !self.to_move);
        let rook_pce = Piece(PieceKind::Rook, self.to_move);
        let king_pce = Piece(PieceKind::King, self.to_move);

        match unmake.mv {
            Move::Regular(from, to) => {
                debug_assert!(self.pieces.get(to).is_some());
                let pce = self.pieces.get(to)
                    .unwrap_or_else(|| unreachable_unchecked());

                self.set_sq(from, pce);
                self.unset_sq(to, pce);
                if let Some(cap_pce) = unmake.capture {
                    self.set_sq(to, cap_pce);
                }

                self.set_castling(unmake.castling);
            },
            Move::Castling(from, to) => {
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
            },
            Move::Promotion(from, to, _) => {
                self.set_sq(from, pawn_pce);
                self.unset_sq(to, pawn_pce);
                if let Some(pce) = unmake.capture {
                    self.set_sq(to, pce);
                }

                self.set_castling(unmake.castling);
            },
            Move::EnPassant(from, to) => {
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
        self.history.last()
            .map(|um| um.mv)
    }
}

impl crate::framework::Position for Position {
    type PieceMap = BitboardPieceMap;

    fn pieces(&self) -> &BitboardPieceMap {
        &self.pieces
    }

    fn to_move(&self) -> Color {
        self.to_move
    }

    fn castling(&self) -> &CastlingRights {
        &self.castling
    }

    fn en_passant_sq(&self) -> Option<Square> {
        self.en_passant_sq
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


#[derive(Copy, Clone)]
struct Unmake {
    mv: Move,
    capture: Option<Piece>,
    castling: CastlingRights,
    en_passant_sq: Option<Square>,
    ply_clock: u8,
}