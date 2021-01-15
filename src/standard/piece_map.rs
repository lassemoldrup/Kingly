use crate::framework::{SquareSet, PieceMap};
use crate::framework::piece::{PieceKind, Piece};
use crate::framework::square::Square;

pub struct SquareSetPieceMap<S: SquareSet> {
    white_pieces: PieceBoards<S>,
    black_pieces: PieceBoards<S>,
}

impl<S: SquareSet> SquareSetPieceMap<S> {
    fn get_sqs(&self, pce: Piece) -> S {
        unimplemented!()
    }
}

impl<S: SquareSet> PieceMap for SquareSetPieceMap<S> {
    fn set_square(&mut self, sq: Square, pce: Piece) {
        unimplemented!()
    }

    fn get(&self, sq: Square) -> Piece {
        unimplemented!()
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
    fn get(kind: PieceKind) -> S {
        unimplemented!()
    }
}