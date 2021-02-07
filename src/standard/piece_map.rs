use crate::framework::PieceMap;
use crate::framework::piece::{PieceKind, Piece};
use crate::framework::square::Square;
use crate::framework::color::Color;
use crate::framework::square_map::SquareMap;
use crate::standard::bitboard::Bitboard;
use bitintr::Tzcnt;

pub struct BitboardPieceMap {
    white_pieces: PieceBoards,
    black_pieces: PieceBoards,
    map: SquareMap<Option<Piece>>,
    occupied: Bitboard,
}

impl BitboardPieceMap {
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
        let king: u64 = self.get_bb(Piece(PieceKind::King, color)).into();
        unsafe {
            Square::from_unchecked(king.tzcnt() as u8)
        }
    }

    /// Gets a `SquareSet` of all occupied squares
    pub fn get_occ(&self) -> Bitboard {
        self.get_occ_for(Color::White) | self.get_occ_for(Color::Black)
    }
}

impl PieceMap for BitboardPieceMap {
    fn new() -> Self {
        BitboardPieceMap {
            white_pieces: PieceBoards::new(),
            black_pieces: PieceBoards::new(),
            map: SquareMap::default(),
            occupied: Bitboard::new(),
        }
    }

    fn set_sq(&mut self, sq: Square, pce: Piece) {
        match pce.color() {
            Color::White => self.white_pieces.set_sq(pce.kind(), sq),
            Color::Black => self.black_pieces.set_sq(pce.kind(), sq),
        }
        self.occupied = self.occupied.add_sq(sq);
        self.map[sq] = Some(pce);
    }

    fn get(&self, sq: Square) -> Option<Piece> {
        self.map[sq]
    }
}

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
    fn new() -> Self {
        PieceBoards {
            pawn: Bitboard::new(),
            knight: Bitboard::new(),
            bishop: Bitboard::new(),
            rook: Bitboard::new(),
            queen: Bitboard::new(),
            king: Bitboard::new(),
            occupied: Bitboard::new(),
        }
    }

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
}