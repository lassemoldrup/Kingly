use crate::framework::value::Value;
use crate::framework::piece::{Piece, PieceKind};
use crate::framework::color::Color;
use crate::framework::Position;
use crate::standard::piece_map::BitboardPieceMap;

pub struct Eval;

impl Eval {
    fn eval_color<P: Position<PieceMap = BitboardPieceMap>>(position: &P, color: Color) -> i32 {
        (position.pieces().get_bb(Piece(PieceKind::Pawn, color)).len() * 100
            + position.pieces().get_bb(Piece(PieceKind::Knight, color)).len() * 300
            + position.pieces().get_bb(Piece(PieceKind::Bishop, color)).len() * 300
            + position.pieces().get_bb(Piece(PieceKind::Rook, color)).len() * 500
            + position.pieces().get_bb(Piece(PieceKind::Queen, color)).len() * 900) as i32
    }
}

impl<P: Position<PieceMap = BitboardPieceMap>> crate::framework::Eval<P> for Eval {
    fn eval(&self, position: &P) -> Value {
        let white_eval = Self::eval_color(position, Color::White);
        let black_eval = Self::eval_color(position, Color::Black);
        Value::CentiPawn((white_eval - black_eval) * position.to_move() as i32)
    }
}