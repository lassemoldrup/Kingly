use crate::framework::{Position, PieceMap};
use crate::framework::fen::Fen;
use crate::framework::moves::Move;
use crate::framework::square::Square;
use crate::framework::piece::Piece;
use crate::framework::color::Color;

pub struct StandardPosition<P: PieceMap> {
    pieces: P,
    to_move: Color,
    en_passant_sq: Option<Square>,
    ply_clock: u32,
    move_number: u32,
}

impl<P: PieceMap> Position for StandardPosition<P> {
    fn new() -> Self {
        unimplemented!()
    }

    fn from_fen(fen: &Fen) -> Self {
        unimplemented!()
    }

    fn gen_moves(&self) -> Vec<Move> {
        unimplemented!()
    }

    fn make_move(&mut self, m: Move) {
        unimplemented!()
    }

    fn unmake_move(&mut self) {
        unimplemented!()
    }

    fn evaluate(&self) -> i32 {
        unimplemented!()
    }
}