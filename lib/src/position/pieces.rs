use std::convert::TryFrom;
use std::fmt::{Debug, Display, Formatter};

use crate::bb;
use crate::square_map::SquareMap;
use crate::types::{Bitboard, Color, Piece, PieceKind, Square};

#[derive(PartialEq, Debug, Copy, Clone, Default)]
pub struct Pieces {
    white_pieces: PieceBoards,
    black_pieces: PieceBoards,
    map: SquareMap<Option<Piece>>,
    occupied: Bitboard,
}

impl Pieces {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, sq: Square) -> Option<Piece> {
        self.map[sq]
    }

    pub fn get_bb(&self, pce: Piece) -> Bitboard {
        match pce.color() {
            Color::White => self.white_pieces.get(pce.kind()),
            Color::Black => self.black_pieces.get(pce.kind()),
        }
    }

    pub fn get_occ_for(&self, color: Color) -> Bitboard {
        match color {
            Color::White => self.white_pieces.occupied,
            Color::Black => self.black_pieces.occupied,
        }
    }

    pub fn get_king_sq(&self, color: Color) -> Square {
        let king = self.get_bb(Piece(PieceKind::King, color));
        unsafe { king.first_sq_unchecked() }
    }

    /// Gets a `Bitboard` of all occupied squares
    pub fn get_occ(&self) -> Bitboard {
        self.get_occ_for(Color::White) | self.get_occ_for(Color::Black)
    }

    pub fn set_sq(&mut self, sq: Square, pce: Piece) {
        match pce.color() {
            Color::White => self.white_pieces.set_sq(pce.kind(), sq),
            Color::Black => self.black_pieces.set_sq(pce.kind(), sq),
        }
        self.occupied = self.occupied.add_sq(sq);

        self.map[sq] = Some(pce);
    }

    pub fn unset_sq(&mut self, sq: Square) {
        let bb = bb!(sq);
        self.white_pieces.unset_sqs(bb);
        self.black_pieces.unset_sqs(bb);
        self.occupied -= bb;
        self.map[sq] = None;
    }
}

impl Display for Pieces {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;
        for rank in (0..8).rev() {
            for file in 0..8 {
                let sq = Square::try_from(8 * rank + file).unwrap();
                match self.get(sq) {
                    None => write!(f, ". ")?,
                    Some(pce) => write!(f, "{} ", pce)?,
                }
            }
            writeln!(f)?;
        }
        Ok(())
    }
}

#[derive(PartialEq, Debug, Copy, Clone, Default)]
struct PieceBoards {
    pawn: Bitboard,
    knight: Bitboard,
    bishop: Bitboard,
    rook: Bitboard,
    queen: Bitboard,
    king: Bitboard,
    occupied: Bitboard,
}

impl PieceBoards {
    fn get(&self, kind: PieceKind) -> Bitboard {
        match kind {
            PieceKind::Pawn => self.pawn,
            PieceKind::Knight => self.knight,
            PieceKind::Bishop => self.bishop,
            PieceKind::Rook => self.rook,
            PieceKind::Queen => self.queen,
            PieceKind::King => self.king,
        }
    }

    fn set_sq(&mut self, kind: PieceKind, sq: Square) {
        match kind {
            PieceKind::Pawn => self.pawn = self.pawn.add_sq(sq),
            PieceKind::Knight => self.knight = self.knight.add_sq(sq),
            PieceKind::Bishop => self.bishop = self.bishop.add_sq(sq),
            PieceKind::Rook => self.rook = self.rook.add_sq(sq),
            PieceKind::Queen => self.queen = self.queen.add_sq(sq),
            PieceKind::King => self.king = self.king.add_sq(sq),
        }
        self.occupied = self.occupied.add_sq(sq);
    }

    fn unset_sqs(&mut self, bb: Bitboard) {
        self.pawn -= bb;
        self.knight -= bb;
        self.bishop -= bb;
        self.rook -= bb;
        self.queen -= bb;
        self.king -= bb;
        self.occupied -= bb;
    }
}
