use crate::framework::{SquareSet, PieceMap};
use crate::framework::piece::{PieceKind, Piece};
use crate::framework::square::Square;
use crate::framework::color::Color;

pub struct SquareSetPieceMap<S: SquareSet + Copy> {
    white_pieces: PieceBoards<S>,
    black_pieces: PieceBoards<S>,
    map: [Option<Piece>; 64],
}

impl<S: SquareSet + Copy> SquareSetPieceMap<S> {
    fn get_sqs(&self, pce: Piece) -> S {
        unimplemented!()
    }
}

impl<S: SquareSet + Copy> PieceMap for SquareSetPieceMap<S> {
    fn new() -> Self {
        SquareSetPieceMap {
            white_pieces: PieceBoards::new(),
            black_pieces: PieceBoards::new(),
            map: [None; 64],
        }
    }

    fn set_sq(&mut self, sq: Square, pce: Piece) {
        let sq_set = &mut match pce.1 {
            Color::White => self.white_pieces.get_mut(pce.0),
            Color::Black => self.black_pieces.get_mut(pce.0),
        };
        sq_set.add(sq);

        self.map[sq as usize] = Some(pce);
    }

    fn get(&self, sq: Square) -> Option<Piece> {
        self.map[sq as usize]
    }
}

struct PieceBoards<S: SquareSet> {
    pawn: S,
    knight: S,
    bishop: S,
    rook: S,
    queen: S,
    king: S,
}

impl<S: SquareSet> PieceBoards<S> {
    fn new() -> Self {
        PieceBoards {
            pawn: S::new(),
            knight: S::new(),
            bishop: S::new(),
            rook: S::new(),
            queen: S::new(),
            king: S::new(),
        }
    }

    fn get(&self, kind: PieceKind) -> S {
        unimplemented!()
    }

    fn get_mut(&mut self, kind: PieceKind) -> &mut S {
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