use crate::framework::PieceMap;
use crate::framework::piece::{PieceKind, Piece};
use crate::framework::square::Square;
use crate::framework::color::Color;
use std::ops::Index;
use crate::framework::square_map::SquareMap;
use crate::standard::bitboard::Bitboard;

pub struct BitboardPieceMap {
    white_pieces: PieceBoards,
    black_pieces: PieceBoards,
    map: SquareMap<Option<Piece>>,
}

impl BitboardPieceMap {
    pub fn get_sqs(&self, pce: Piece) -> Bitboard {
        match pce.1 {
            Color::White => self.white_pieces.get(pce.0),
            Color::Black => self.black_pieces.get(pce.0),
        }
    }
    
    pub fn get_sqs_for(&self, col: Color) -> Bitboard {
        match col {
            Color::White => self.white_pieces.pawn
                | self.white_pieces.knight
                | self.white_pieces.bishop
                | self.white_pieces.rook
                | self.white_pieces.queen
                | self.white_pieces.king,
            Color::Black => self.black_pieces.pawn
                | self.black_pieces.knight
                | self.black_pieces.bishop
                | self.black_pieces.rook
                | self.black_pieces.queen
                | self.black_pieces.king,
        }
    }

    /// Gets a `SquareSet` of all occupied squares
    pub fn get_occupied(&self) -> Bitboard {
        self.get_sqs_for(Color::White) | self.get_sqs_for(Color::Black)
    }
}

impl PieceMap for BitboardPieceMap {
    fn new() -> Self {
        BitboardPieceMap {
            white_pieces: PieceBoards::new(),
            black_pieces: PieceBoards::new(),
            map: SquareMap::default(),
        }
    }

    fn set_sq(&mut self, sq: Square, pce: Piece) {
        let bb = match pce.1 {
            Color::White => self.white_pieces.get_mut(pce.0),
            Color::Black => self.black_pieces.get_mut(pce.0),
        };
        *bb = bb.add_sq(sq);

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

    fn get_mut(&mut self, kind: PieceKind) -> &mut Bitboard {
        match kind {
            PieceKind::Pawn => &mut self.pawn,
            PieceKind::Knight => &mut self.knight,
            PieceKind::Bishop => &mut self.bishop,
            PieceKind::Rook => &mut self.rook,
            PieceKind::Queen => &mut self.queen,
            PieceKind::King => &mut self.king,
        }
    }
}